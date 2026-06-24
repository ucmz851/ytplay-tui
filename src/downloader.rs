use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

pub(crate) const DOWNLOAD_OPTIONS: [&str; 4] = [
    "Video - Best Quality (MP4)",
    "Video - 1080p (MP4)",
    "Video - 720p (MP4)",
    "Audio Only - Best (MP3)",
];

pub(crate) enum DlMessage {
    Progress {
        pct: f64,
        size: Option<String>,
        speed: Option<String>,
        eta: Option<String>,
    },
    Finished,
    Error(String),
}

pub(crate) struct ActiveDownload {
    pub(crate) title: String,
    pub(crate) progress: f64,
    pub(crate) size: Option<String>,
    pub(crate) speed: Option<String>,
    pub(crate) eta: Option<String>,
    pub(crate) finished: bool,
    pub(crate) finish_tick: u64,
}

pub(crate) fn spawn_download(video_id: String, option_idx: usize, tx: Sender<DlMessage>) {
    thread::spawn(move || {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let out_tmpl = format!("{}/Downloads/%(title)s.%(ext)s", home);
        let url = format!("https://www.youtube.com/watch?v={}", video_id);

        let mut cmd = Command::new("yt-dlp");

        match option_idx {
            0 => {
                cmd.args([
                    "-f",
                    "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
                ]);
            }
            1 => {
                cmd.args([
                    "-f",
                    "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080]/best",
                ]);
            }
            2 => {
                cmd.args([
                    "-f",
                    "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/best[height<=720]/best",
                ]);
            }
            3 => {
                cmd.args(["-x", "--audio-format", "mp3", "--audio-quality", "0"]);
            }
            _ => {}
        };

        cmd.arg("--newline");
        cmd.arg("-o").arg(&out_tmpl).arg(&url);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                let re_full = regex::Regex::new(
                    r"\[download\]\s+([\d\.]+)%\s+of\s+(\S+)\s+at\s+(\S+)\s+ETA\s+(\S+)",
                )
                .unwrap();
                let re_pct = regex::Regex::new(r"\[download\]\s+([\d\.]+)%").unwrap();

                for line in reader.lines().map_while(Result::ok) {
                    if let Some((cap, pct)) = re_full
                        .captures(&line)
                        .and_then(|cap| cap[1].parse::<f64>().ok().map(|pct| (cap, pct)))
                    {
                        let _ = tx.send(DlMessage::Progress {
                            pct,
                            size: Some(cap[2].to_string()),
                            speed: Some(cap[3].to_string()),
                            eta: Some(cap[4].to_string()),
                        });
                    } else if let Some(pct) = re_pct
                        .captures(&line)
                        .and_then(|cap| cap[1].parse::<f64>().ok())
                    {
                        let _ = tx.send(DlMessage::Progress {
                            pct,
                            size: None,
                            speed: None,
                            eta: None,
                        });
                    }
                }
            }
            let _ = child.wait();
            let _ = tx.send(DlMessage::Finished);
        } else {
            let _ = tx.send(DlMessage::Error("Failed to start yt-dlp".to_string()));
        }
    });
}
