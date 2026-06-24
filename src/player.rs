use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub(crate) enum MpvCommand {
    Load(String, String, bool), // url, fallback title, is_audio_only
    TogglePause,
    Volume(f64),
    Seek(f64),
    Quit,
}

#[derive(Clone, Default)]
pub(crate) struct PlayerStatus {
    pub(crate) connected: bool,
    pub(crate) paused: bool,
    pub(crate) loading: bool,
    pub(crate) media_title: Option<String>,
    pub(crate) fallback_title: Option<String>,
    pub(crate) time_pos: f64,
    pub(crate) duration: f64,
    pub(crate) volume: f64,
}

impl PlayerStatus {
    pub(crate) fn effective_title(&self) -> Option<String> {
        self.media_title
            .clone()
            .or_else(|| self.fallback_title.clone())
    }
}

pub(crate) fn spawn_mpv_controller(
    status: Arc<Mutex<PlayerStatus>>,
    cmd_rx: Receiver<MpvCommand>,
    finished_tx: Sender<()>,
) {
    thread::spawn(move || {
        let socket_path = format!("/tmp/yttui-mpv-{}.sock", std::process::id());
        let mut current_child: Option<std::process::Child> = None;
        let mut current_stream: Option<UnixStream> = None;

        while let Ok(cmd) = cmd_rx.recv() {
            if let MpvCommand::Quit = cmd {
                if let Some(stream) = current_stream.as_ref() {
                    let mut stream_ref = stream;
                    let _ = writeln!(stream_ref, "{}", json!({"command": ["quit"]}));
                }
                if let Some(mut child) = current_child.take() {
                    let _ = child.wait();
                }
                let _ = std::fs::remove_file(&socket_path);
                break;
            }

            let mut needs_restart = false;
            if let Some(ref mut child) = current_child {
                match child.try_wait() {
                    Ok(Some(_)) => needs_restart = true,
                    Ok(None) => {}
                    Err(_) => needs_restart = true,
                }
            } else {
                needs_restart = true;
            }

            if needs_restart {
                let _ = std::fs::remove_file(&socket_path);

                let child = Command::new("mpv")
                    .arg("--idle=yes")
                    .arg("--no-terminal")
                    .arg("--ytdl-format=best")
                    .arg(format!("--input-ipc-server={socket_path}"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();

                current_child = child.ok();
                current_stream = None;

                if current_child.is_some() {
                    for _ in 0..100 {
                        if let Ok(s) = UnixStream::connect(&socket_path) {
                            current_stream = Some(s);
                            break;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                }

                if let Some(stream) = current_stream.as_ref() {
                    let Ok(read_stream) = stream.try_clone() else {
                        continue;
                    };
                    let read_status = Arc::clone(&status);
                    let finished_tx2 = finished_tx.clone();

                    thread::spawn(move || {
                        let reader = BufReader::new(read_stream);
                        let mut was_active = false;

                        for line in reader.lines().map_while(Result::ok) {
                            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                                continue;
                            };
                            let event_type = v.get("event").and_then(|e| e.as_str());

                            if event_type == Some("file-loaded") || event_type == Some("end-file") {
                                let mut st = read_status.lock().unwrap();
                                st.loading = false;
                                continue;
                            }
                            if event_type != Some("property-change") {
                                continue;
                            }

                            let Some(name) = v.get("name").and_then(|n| n.as_str()) else {
                                continue;
                            };
                            let data = v.get("data");
                            let mut st = read_status.lock().unwrap();
                            match name {
                                "time-pos" => {
                                    st.time_pos =
                                        data.and_then(|d| d.as_f64()).unwrap_or(st.time_pos)
                                }
                                "duration" => {
                                    st.duration =
                                        data.and_then(|d| d.as_f64()).unwrap_or(st.duration)
                                }
                                "pause" => {
                                    st.paused = data.and_then(|d| d.as_bool()).unwrap_or(st.paused)
                                }
                                "volume" => {
                                    st.volume = data.and_then(|d| d.as_f64()).unwrap_or(st.volume)
                                }
                                "media-title" => {
                                    st.media_title = data.and_then(|d| d.as_str()).map(String::from)
                                }
                                "idle-active" => {
                                    let idle = data.and_then(|d| d.as_bool()).unwrap_or(false);
                                    if idle && was_active {
                                        was_active = false;
                                        st.loading = false;
                                        drop(st);
                                        let _ = finished_tx2.send(());
                                        continue;
                                    }
                                    if !idle {
                                        was_active = true;
                                    }
                                }
                                _ => {}
                            }
                        }
                        let mut st = read_status.lock().unwrap();
                        st.connected = false;
                    });

                    let mut stream_ref = stream;
                    for (idx, prop) in [
                        "time-pos",
                        "duration",
                        "pause",
                        "volume",
                        "media-title",
                        "idle-active",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let observe_id = (idx + 1) as u64;
                        let _ = writeln!(
                            stream_ref,
                            "{}",
                            json!({"command": ["observe_property", observe_id, prop]})
                        );
                    }

                    {
                        let mut st = status.lock().unwrap();
                        st.connected = true;
                    }
                }
            }

            if let Some(stream) = current_stream.as_ref() {
                let mut stream_ref = stream;
                let payload = match cmd {
                    MpvCommand::Load(url, title, is_audio_only) => {
                        {
                            let mut st = status.lock().unwrap();
                            st.fallback_title = Some(title);
                            st.media_title = None;
                            st.time_pos = 0.0;
                            st.duration = 0.0;
                            st.loading = true;
                        }
                        let vid_prop = if is_audio_only { "no" } else { "auto" };
                        let _ = writeln!(
                            stream_ref,
                            "{}",
                            json!({"command": ["set_property", "vid", vid_prop]})
                        );

                        json!({"command": ["loadfile", url, "replace"]})
                    }
                    MpvCommand::TogglePause => json!({"command": ["cycle", "pause"]}),
                    MpvCommand::Volume(delta) => json!({"command": ["add", "volume", delta]}),
                    MpvCommand::Seek(delta) => json!({"command": ["seek", delta]}),
                    MpvCommand::Quit => unreachable!(),
                };
                let _ = writeln!(stream_ref, "{}", payload);
            }
        }

        // Cleanup if loop exited due to channel disconnection (app panicked or exited abnormally)
        if let Some(mut child) = current_child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(&socket_path);
    });
}
