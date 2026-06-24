use crate::app::{App, Mode, Pane, SubPane};
use crate::models::{Playlist, PlaylistItem, save_playlists};
use crate::player::MpvCommand;
use crate::theme::*;
use crossterm::event::{self, KeyCode, KeyEvent};

pub(crate) fn handle_key(app: &mut App, key: KeyEvent) {
    if app.show_help {
        app.show_help = false;
        return;
    }

    match app.mode {
        Mode::SearchInput => match key.code {
            KeyCode::Enter => app.start_search(),
            KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.search_buffer = app.search_query.clone();
            }
            KeyCode::Backspace => {
                app.search_buffer.pop();
            }
            KeyCode::Char(c) => app.search_buffer.push(c),
            _ => {}
        },
        Mode::CreatePlaylistInput => match key.code {
            KeyCode::Enter => {
                let name = app.search_buffer.trim().to_string();
                if !name.is_empty() {
                    if !app.playlists.iter().any(|p| p.name == name) {
                        app.playlists.push(Playlist {
                            name: name.clone(),
                            videos: vec![],
                        });
                        save_playlists(&app.playlists);
                        app.notification = Some((
                            format!("{} Playlist created: {}", ICON_PLAYLIST, name),
                            app.tick + 100,
                        ));
                    } else {
                        app.notification =
                            Some(("Playlist already exists!".to_string(), app.tick + 100));
                    }
                }
                app.mode = Mode::Normal;
                app.search_buffer = app.search_query.clone();
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.search_buffer = app.search_query.clone();
            }
            KeyCode::Backspace => {
                app.search_buffer.pop();
            }
            KeyCode::Char(c) => app.search_buffer.push(c),
            _ => {}
        },
        Mode::SelectPlaylistModal(ref vid) => match key.code {
            KeyCode::Enter => {
                if let Some(idx) = app
                    .playlist_modal_state
                    .selected()
                    .filter(|&idx| idx < app.playlists.len())
                {
                    app.playlists[idx].videos.push(vid.clone());
                    save_playlists(&app.playlists);
                    app.notification = Some((
                        format!("{} Added to: {}", ICON_PLAYLIST, app.playlists[idx].name),
                        app.tick + 100,
                    ));
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            _ => {}
        },
        Mode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('/') => {
                app.search_buffer = app.search_query.clone();
                app.mode = Mode::SearchInput;
            }
            KeyCode::Esc => {
                if app.focused == Pane::Download
                    || app.focused == Pane::Playlists
                    || app.focused == Pane::Subscriptions
                {
                    app.focused = Pane::Feed;
                    app.download_target = None;
                }
            }
            KeyCode::Char('P') => {
                if app.focused == Pane::Playlists {
                    app.focused = Pane::Feed;
                } else {
                    app.focused = Pane::Playlists;
                    if app.playlists_state.selected().is_none() && !app.playlists.is_empty() {
                        app.playlists_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('S') => {
                if app.focused == Pane::Subscriptions {
                    app.focused = Pane::Feed;
                } else {
                    app.focused = Pane::Subscriptions;
                    app.focused_sub_pane = SubPane::Channels;
                    if app.subscriptions_state.selected().is_none() && !app.subscriptions.is_empty()
                    {
                        app.subscriptions_state.select(Some(0));
                        app.load_selected_channel_videos();
                    }
                }
            }
            KeyCode::Char('u') => {
                app.toggle_subscription_selected();
            }
            KeyCode::Char('r') => {
                if app.focused == Pane::Subscriptions {
                    app.load_selected_channel_videos();
                    app.notification = Some(("Refreshing feed...".to_string(), app.tick + 60));
                }
            }
            KeyCode::Char('D') => {
                if app.focused == Pane::Download {
                    app.focused = Pane::Feed;
                    app.download_target = None;
                } else {
                    let selected_vid = match app.focused {
                        Pane::Feed => app
                            .results
                            .get(app.results_state.selected().unwrap_or(0))
                            .cloned(),
                        Pane::Queue => app
                            .queue
                            .get(app.queue_state.selected().unwrap_or(0))
                            .cloned(),
                        Pane::Playlists => {
                            let flat = app.get_flat_playlists();
                            if let Some(idx) = app.playlists_state.selected() {
                                if let Some(PlaylistItem::Video(p_idx, v_idx)) = flat.get(idx) {
                                    app.playlists[*p_idx].videos.get(*v_idx).cloned()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        Pane::Subscriptions => {
                            if app.focused_sub_pane == SubPane::Videos {
                                app.sub_videos
                                    .get(app.sub_videos_state.selected().unwrap_or(0))
                                    .cloned()
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(v) = selected_vid {
                        app.download_target = Some(v);
                        app.focused = Pane::Download;
                        app.download_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('c') => {
                if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                    app.should_quit = true;
                } else {
                    app.mode = Mode::CreatePlaylistInput;
                    app.search_buffer.clear();
                }
            }
            KeyCode::Char('s') => {
                let selected_vid = match app.focused {
                    Pane::Feed => app
                        .results
                        .get(app.results_state.selected().unwrap_or(0))
                        .cloned(),
                    Pane::Queue => app
                        .queue
                        .get(app.queue_state.selected().unwrap_or(0))
                        .cloned(),
                    Pane::Playlists => {
                        let flat = app.get_flat_playlists();
                        if let Some(idx) = app.playlists_state.selected() {
                            if let Some(PlaylistItem::Video(p_idx, v_idx)) = flat.get(idx) {
                                app.playlists[*p_idx].videos.get(*v_idx).cloned()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Pane::Subscriptions => {
                        if app.focused_sub_pane == SubPane::Videos {
                            app.sub_videos
                                .get(app.sub_videos_state.selected().unwrap_or(0))
                                .cloned()
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(v) = selected_vid {
                    if app.playlists.is_empty() {
                        app.mode = Mode::CreatePlaylistInput;
                        app.search_buffer.clear();
                        app.notification = Some((
                            "No playlists exist. Create one first!".to_string(),
                            app.tick + 100,
                        ));
                    } else {
                        app.mode = Mode::SelectPlaylistModal(v);
                        app.playlist_modal_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            KeyCode::Char('l') | KeyCode::Right => {
                if app.focused == Pane::Playlists {
                    let flat = app.get_flat_playlists();
                    if let Some(PlaylistItem::Header(p_idx)) =
                        app.playlists_state.selected().and_then(|idx| flat.get(idx))
                    {
                        let name = app.playlists[*p_idx].name.clone();
                        if app.expanded_playlists.contains(&name) {
                            app.expanded_playlists.remove(&name);
                        } else {
                            app.expanded_playlists.insert(name);
                        }
                    }
                } else {
                    let _ = app.mpv_cmd_tx.send(MpvCommand::Seek(5.0));
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if app.focused == Pane::Playlists {
                    let flat = app.get_flat_playlists();
                    if let Some(idx) = app.playlists_state.selected() {
                        match flat.get(idx) {
                            Some(PlaylistItem::Header(p_idx)) => {
                                let name = app.playlists[*p_idx].name.clone();
                                app.expanded_playlists.remove(&name);
                            }
                            Some(PlaylistItem::Video(p_idx, _)) => {
                                let name = app.playlists[*p_idx].name.clone();
                                app.expanded_playlists.remove(&name);
                                if let Some(header_idx) = app.get_flat_playlists().iter().position(|item| matches!(item, PlaylistItem::Header(idx) if idx == p_idx)) {
                                    app.playlists_state.select(Some(header_idx));
                                }
                            }
                            None => {}
                        }
                    }
                } else {
                    let _ = app.mpv_cmd_tx.send(MpvCommand::Seek(-5.0));
                }
            }
            KeyCode::Tab => {
                if app.focused == Pane::Subscriptions {
                    app.focused_sub_pane = match app.focused_sub_pane {
                        SubPane::Channels => {
                            if app.sub_videos_state.selected().is_none()
                                && !app.sub_videos.is_empty()
                            {
                                app.sub_videos_state.select(Some(0));
                            }
                            SubPane::Videos
                        }
                        SubPane::Videos => SubPane::Channels,
                    };
                } else {
                    app.toggle_pane();
                }
            }
            KeyCode::Enter => app.activate_selected(false),
            KeyCode::Char('m') => app.activate_selected(true),
            KeyCode::Char('a') => app.queue_selected(),
            KeyCode::Char('d') => app.remove_selected_from_pane(),
            KeyCode::Char('N') => app.advance_queue(),
            KeyCode::Char(' ') => {
                let _ = app.mpv_cmd_tx.send(MpvCommand::TogglePause);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let _ = app.mpv_cmd_tx.send(MpvCommand::Volume(5.0));
            }
            KeyCode::Char('-') => {
                let _ = app.mpv_cmd_tx.send(MpvCommand::Volume(-5.0));
            }
            KeyCode::Char('?') => app.show_help = true,
            _ => {}
        },
    }
}
