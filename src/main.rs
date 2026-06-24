mod app;
mod downloader;
mod invidious;
mod keyboard;
mod models;
mod player;
mod scraper;
mod theme;
mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use app::{App, Pane};
use downloader::DlMessage;
use keyboard::handle_key;
use player::{MpvCommand, PlayerStatus, spawn_mpv_controller};
use ui::ui;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        let _ = disable_raw_mode();
        return Err(e);
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = disable_raw_mode();
            let mut stdout_err = io::stdout();
            let _ = execute!(stdout_err, LeaveAlternateScreen, DisableMouseCapture);
            return Err(e);
        }
    };

    let (mpv_cmd_tx, mpv_cmd_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let player_status = Arc::new(Mutex::new(PlayerStatus::default()));
    spawn_mpv_controller(Arc::clone(&player_status), mpv_cmd_rx, finished_tx);

    let mut app = App::new(player_status, mpv_cmd_tx, finished_rx);

    loop {
        // Handle Search Outcomes
        if let Some(outcome) = app
            .search_rx
            .try_recv()
            .ok()
            .filter(|o| o.seq == app.search_seq)
        {
            app.searching = false;
            match outcome.result {
                Ok(videos) => {
                    app.search_err = None;
                    app.results = videos;
                    app.results_state.select(if app.results.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                    app.focused = Pane::Feed;
                }
                Err(e) => {
                    app.search_err = Some(e);
                    app.results.clear();
                    app.results_state.select(None);
                }
            }
        }

        // Handle Subscriptions Outcomes
        while let Ok(outcome) = app.sub_rx.try_recv() {
            if app.subscriptions_state.selected().is_some_and(|idx| {
                idx < app.subscriptions.len() && app.subscriptions[idx].id == outcome.channel_id
            }) {
                app.sub_loading = false;
                match outcome.result {
                    Ok(videos) => {
                        // Merge RSS videos and matching search results, removing duplicates
                        let mut merged = videos;
                        for v in &app.results {
                            if v.channel_id.as_ref() == Some(&outcome.channel_id)
                                && !merged.iter().any(|mv| mv.id == v.id)
                            {
                                merged.push(v.clone());
                            }
                        }
                        app.sub_videos = merged;
                        app.sub_videos_state.select(if app.sub_videos.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
                    }
                    Err(e) => {
                        // Fallback: Populate from search results if RSS failed
                        let mut merged = Vec::new();
                        for v in &app.results {
                            if v.channel_id.as_ref() == Some(&outcome.channel_id) {
                                merged.push(v.clone());
                            }
                        }
                        if !merged.is_empty() {
                            app.sub_videos = merged;
                            app.sub_videos_state.select(Some(0));
                        } else {
                            app.sub_videos.clear();
                            app.sub_videos_state.select(None);
                            app.notification = Some((format!("RSS Error: {}", e), app.tick + 100));
                        }
                    }
                }
            }
        }

        // Handle Background Downloads Output
        while let Ok(msg) = app.dl_rx.try_recv() {
            match msg {
                DlMessage::Progress {
                    pct,
                    size,
                    speed,
                    eta,
                } => {
                    if let Some(dl) = &mut app.active_download {
                        dl.progress = pct.clamp(0.0, 100.0);
                        dl.size = size;
                        dl.speed = speed;
                        dl.eta = eta;
                    }
                }
                DlMessage::Finished => {
                    if let Some(dl) = &mut app.active_download {
                        dl.progress = 100.0;
                        dl.finished = true;
                        dl.finish_tick = app.tick + 150;
                    }
                }
                DlMessage::Error(e) => {
                    app.active_download = None;
                    app.notification = Some((format!("Download Error: {}", e), app.tick + 200));
                }
            }
        }

        // Clear finished downloads
        if app
            .active_download
            .as_ref()
            .is_some_and(|dl| dl.finished && app.tick >= dl.finish_tick)
        {
            app.active_download = None;
        }

        // Handle Queue Progression
        while app.finished_rx.try_recv().is_ok() {
            app.advance_queue();
        }

        if let Err(e) = terminal.draw(|f| ui(f, &mut app)) {
            // Restore terminal state on draw failure
            let _ = disable_raw_mode();
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            return Err(e);
        }
        app.tick = app.tick.wrapping_add(1);

        if event::poll(Duration::from_millis(16))? {
            #[allow(clippy::collapsible_if)]
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);
                if app.should_quit {
                    break;
                }
            }
        }
    }

    let _ = app.mpv_cmd_tx.send(MpvCommand::Quit);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
