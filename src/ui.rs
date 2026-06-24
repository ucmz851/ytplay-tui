use crate::app::{App, Mode, Pane};
use crate::downloader::DOWNLOAD_OPTIONS;
use crate::models::{PlaylistItem, fmt_clock};
use crate::theme::*;
use rand::Rng;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Sparkline,
};

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let theme = &app.theme;

    let bottom_height = if app.active_download.is_some() { 2 } else { 1 };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(if app.is_audio_mode { 8 } else { 4 }),
            Constraint::Length(bottom_height),
        ])
        .split(f.size());

    // ---- Search bar / Input bar ----
    let input_active = !matches!(app.mode, Mode::Normal);
    let search_border = if input_active {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let (display_text, text_style, title) = match &app.mode {
        Mode::SearchInput => (
            format!("{}█", app.search_buffer),
            Style::default().fg(theme.fg),
            format!(" {} Search ", ICON_SEARCH),
        ),
        Mode::CreatePlaylistInput => (
            format!("{}█", app.search_buffer),
            Style::default().fg(theme.success),
            format!(" {} Create New Playlist ", ICON_PLAYLIST),
        ),
        Mode::SelectPlaylistModal(_) | Mode::Normal => {
            if !app.search_query.is_empty() {
                (
                    app.search_query.clone(),
                    Style::default().fg(theme.fg),
                    format!(" {} Search ", ICON_SEARCH),
                )
            } else {
                (
                    "Press / to search YouTube".to_string(),
                    Style::default().fg(theme.text_dim),
                    format!(" {} Search ", ICON_SEARCH),
                )
            }
        }
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(search_border)
        .title(title);

    f.render_widget(
        Paragraph::new(Span::styled(format!(" {display_text}"), text_style)).block(search_block),
        root[0],
    );

    // ---- Dynamic Workspace ----
    let pointer_symbol = format!(" {} ", ICON_POINTER);

    if app.focused == Pane::Subscriptions {
        let sub_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(root[1]);

        let channels_focused = app.focused_sub_pane == crate::app::SubPane::Channels;
        let channels_border = if channels_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border_inactive)
        };
        let videos_border = if !channels_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border_inactive)
        };

        // Left Column: Channels
        let ch_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(channels_border)
            .title(format!(
                " {} Subscriptions ({}) ",
                ICON_PLAYLIST,
                app.subscriptions.len()
            ));

        let ch_items: Vec<ListItem> = if app.subscriptions.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                " No subscriptions. Press 'u' on results to subscribe.",
                Style::default().fg(theme.text_dim),
            )))]
        } else {
            app.subscriptions
                .iter()
                .map(|sub| {
                    ListItem::new(Line::from(Span::styled(
                        format!(" {}", sub.name),
                        Style::default().fg(theme.fg),
                    )))
                })
                .collect()
        };

        let ch_list = List::new(ch_items)
            .block(ch_block)
            .highlight_style(
                Style::default()
                    .bg(theme.accent_dim)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(&pointer_symbol);
        f.render_stateful_widget(ch_list, sub_chunks[0], &mut app.subscriptions_state);

        // Right Column: Videos
        let videos_title = if let Some(idx) = app.subscriptions_state.selected() {
            if idx < app.subscriptions.len() {
                let name = &app.subscriptions[idx].name;
                if app.sub_loading {
                    let spinner = SPINNER[(app.tick / 4) as usize % SPINNER.len()];
                    format!(" {spinner} Loading {name}... ")
                } else {
                    format!(
                        " {} {name}'s Videos ({}) ",
                        ICON_YOUTUBE,
                        app.sub_videos.len()
                    )
                }
            } else {
                " Videos ".to_string()
            }
        } else {
            " Videos ".to_string()
        };

        let vid_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(videos_border)
            .title(videos_title);

        let vid_items: Vec<ListItem> = if app.subscriptions.is_empty() {
            vec![]
        } else if app.sub_loading && app.sub_videos.is_empty() {
            let spinner = SPINNER[(app.tick / 4) as usize % SPINNER.len()];
            vec![ListItem::new(Line::from(Span::styled(
                format!(" {} Loading subscription feed...", spinner),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))]
        } else if app.sub_videos.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                " No videos found or RSS feed unavailable.",
                Style::default().fg(theme.text_dim),
            )))]
        } else {
            app.sub_videos
                .iter()
                .map(|v| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            format!(" {}", v.title),
                            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            format!(
                                "   {} {}  •  {} {}",
                                ICON_TIME,
                                v.duration_str(),
                                ICON_VIEWS,
                                v.views_str()
                            ),
                            Style::default().fg(theme.text_dim),
                        )),
                    ])
                })
                .collect()
        };

        let vid_list = List::new(vid_items)
            .block(vid_block)
            .highlight_style(
                Style::default()
                    .bg(theme.accent_dim)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(&pointer_symbol);
        f.render_stateful_widget(vid_list, sub_chunks[1], &mut app.sub_videos_state);
    } else {
        let show_right_pane = app.focused == Pane::Download
            || app.focused == Pane::Playlists
            || !app.queue.is_empty();
        let layout_constraints = if show_right_pane {
            vec![Constraint::Percentage(60), Constraint::Percentage(40)]
        } else {
            vec![Constraint::Percentage(100)]
        };

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(layout_constraints)
            .split(root[1]);

        let feed_border = if app.focused == Pane::Feed {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border_inactive)
        };
        let right_border = if app.focused != Pane::Feed {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border_inactive)
        };

        // Feed Widget
        let feed_title = if app.searching {
            let spinner = SPINNER[(app.tick / 4) as usize % SPINNER.len()];
            format!(" {spinner} Searching... ")
        } else {
            format!(" {} Results ({}) ", ICON_YOUTUBE, app.results.len())
        };

        let feed_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(feed_border)
            .title(feed_title);

        let feed_items: Vec<ListItem> = if let Some(err) = &app.search_err {
            vec![ListItem::new(Line::from(Span::styled(
                format!("  {err}"),
                Style::default().fg(theme.error),
            )))]
        } else if app.searching && app.results.is_empty() {
            let spinner = SPINNER[(app.tick / 4) as usize % SPINNER.len()];
            vec![ListItem::new(Line::from(Span::styled(
                format!(" {} Searching YouTube...", spinner),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))]
        } else if app.results.is_empty() && !app.searching {
            vec![ListItem::new(Line::from(Span::styled(
                " Search for a video to start...",
                Style::default().fg(theme.text_dim),
            )))]
        } else {
            app.results
                .iter()
                .map(|v| {
                    let is_subscribed = if let Some(ch_id) = &v.channel_id {
                        app.subscriptions.iter().any(|s| s.id == *ch_id)
                    } else {
                        false
                    };

                    let meta_line = if is_subscribed {
                        Line::from(vec![
                            Span::styled(
                                format!("   {} ", ICON_BELL),
                                Style::default().fg(theme.warning),
                            ),
                            Span::styled(
                                format!(
                                    "{}  •  {} {}  •  {} {}",
                                    v.uploader,
                                    ICON_TIME,
                                    v.duration_str(),
                                    ICON_VIEWS,
                                    v.views_str()
                                ),
                                Style::default().fg(theme.text_dim),
                            ),
                        ])
                    } else {
                        Line::from(Span::styled(
                            format!(
                                "   {}  •  {} {}  •  {} {}",
                                v.uploader,
                                ICON_TIME,
                                v.duration_str(),
                                ICON_VIEWS,
                                v.views_str()
                            ),
                            Style::default().fg(theme.text_dim),
                        ))
                    };

                    ListItem::new(vec![
                        Line::from(Span::styled(
                            format!(" {}", v.title),
                            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                        )),
                        meta_line,
                    ])
                })
                .collect()
        };

        let feed_list = List::new(feed_items)
            .block(feed_block)
            .highlight_style(
                Style::default()
                    .bg(theme.accent_dim)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(&pointer_symbol);
        f.render_stateful_widget(feed_list, main_chunks[0], &mut app.results_state);

        // Right Pane Widget
        if app.focused == Pane::Download {
            let dl_title = if let Some(v) = &app.download_target {
                let trunc_title: String = v.title.chars().take(25).collect();
                format!(" {} Download: {}... ", ICON_DOWNLOAD, trunc_title)
            } else {
                format!(" {} Download Options ", ICON_DOWNLOAD)
            };

            let dl_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(right_border)
                .title(dl_title);

            let dl_items: Vec<ListItem> = DOWNLOAD_OPTIONS
                .iter()
                .map(|opt| {
                    ListItem::new(Line::from(vec![Span::styled(
                        format!(" {} ", opt),
                        Style::default().fg(theme.fg),
                    )]))
                })
                .collect();

            let dl_list = List::new(dl_items)
                .block(dl_block)
                .highlight_style(
                    Style::default()
                        .bg(theme.accent_dim)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(&pointer_symbol);
            f.render_stateful_widget(dl_list, main_chunks[1], &mut app.download_state);
        } else if app.focused == Pane::Playlists {
            let pl_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(right_border)
                .title(format!(" {} Your Playlists ", ICON_PLAYLIST));

            let flat = app.get_flat_playlists();
            let pl_items: Vec<ListItem> = if flat.is_empty() {
                vec![ListItem::new(Line::from(Span::styled(
                    " No playlists found. Press 'c' to create.",
                    Style::default().fg(theme.text_dim),
                )))]
            } else {
                flat.iter()
                    .map(|item| match item {
                        PlaylistItem::Header(p_idx) => {
                            let p = &app.playlists[*p_idx];
                            let is_expanded = app.expanded_playlists.contains(&p.name);
                            let icon = if is_expanded {
                                ICON_FOLDER_OPEN
                            } else {
                                ICON_FOLDER
                            };
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!(" {} ", icon),
                                    Style::default().fg(theme.accent),
                                ),
                                Span::styled(
                                    p.name.clone(),
                                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!(" ({} videos)", p.videos.len()),
                                    Style::default().fg(theme.text_dim),
                                ),
                            ]))
                        }
                        PlaylistItem::Video(p_idx, v_idx) => {
                            let v = &app.playlists[*p_idx].videos[*v_idx];
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!("    {}. ", v_idx + 1),
                                    Style::default().fg(theme.text_dim),
                                ),
                                Span::styled(v.title.clone(), Style::default().fg(theme.fg)),
                            ]))
                        }
                    })
                    .collect()
            };

            let pl_symbol = format!(" {} ", ICON_PLAY);
            let pl_list = List::new(pl_items)
                .block(pl_block)
                .highlight_style(
                    Style::default()
                        .bg(theme.accent_dim)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(&pl_symbol);
            f.render_stateful_widget(pl_list, main_chunks[1], &mut app.playlists_state);
        } else if !app.queue.is_empty() {
            let queue_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(right_border)
                .title(format!(" {} Up Next ({}) ", ICON_QUEUE, app.queue.len()));

            let queue_items: Vec<ListItem> = app
                .queue
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {}. ", i + 1), Style::default().fg(theme.text_dim)),
                        Span::styled(v.title.clone(), Style::default().fg(theme.fg)),
                    ]))
                })
                .collect();

            let play_symbol = format!(" {} ", ICON_PLAY);
            let queue_list = List::new(queue_items)
                .block(queue_block)
                .highlight_style(
                    Style::default()
                        .bg(theme.accent_dim)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(&play_symbol);
            f.render_stateful_widget(queue_list, main_chunks[1], &mut app.queue_state);
        }
    }

    // ---- Player ----
    let status = app.player_status.lock().unwrap().clone();

    let player_title = if app.is_audio_mode {
        format!(" {} Now Playing (Audio) ", ICON_MUSIC)
    } else {
        format!(" {} Now Playing (Video) ", ICON_PLAY)
    };

    let player_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_inactive))
        .title(player_title);

    f.render_widget(player_block, root[2]);

    let player_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.is_audio_mode {
            vec![
                Constraint::Length(1),
                Constraint::Min(2),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Length(1), Constraint::Length(1)]
        })
        .margin(1)
        .split(root[2]);

    let play_icon = if status.loading {
        SPINNER[(app.tick / 4) as usize % SPINNER.len()]
    } else if status.paused {
        ICON_PAUSE
    } else {
        ICON_PLAY
    };

    let title_text = if status.loading {
        status
            .effective_title()
            .unwrap_or_else(|| "Loading...".to_string())
    } else {
        status
            .effective_title()
            .unwrap_or_else(|| "Ready to play".to_string())
    };

    let icon_color = if status.loading {
        theme.warning
    } else {
        theme.accent
    };
    let uploader = app
        .now_playing
        .as_ref()
        .map(|v| v.uploader.clone())
        .unwrap_or_default();

    let title_line = Line::from(vec![
        Span::styled(
            format!("{play_icon} "),
            Style::default().fg(icon_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            title_text,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if uploader.is_empty() || status.loading {
                String::new()
            } else {
                format!("  — {uploader}")
            },
            Style::default().fg(theme.text_dim),
        ),
        Span::styled(
            format!("    {} {:.0}%", ICON_VOLUME, status.volume),
            Style::default().fg(theme.text_dim),
        ),
    ]);
    f.render_widget(Paragraph::new(title_line), player_inner[0]);

    if app.is_audio_mode {
        let width = f.size().width as usize;
        if width > 0 {
            let mut visualizer_data = Vec::with_capacity(width);
            let mut rng = rand::thread_rng();

            if status.loading {
                for i in 0..width {
                    let pos = (app.tick % width as u64) as f64;
                    let dist = (i as f64 - pos).abs();
                    let val = if dist < 8.0 { (8.0 - dist) * 12.0 } else { 0.0 };
                    visualizer_data.push(val as u64);
                }
            } else if !status.paused && status.duration > 0.0 {
                for i in 0..width {
                    let wave1 = f64::sin(app.tick as f64 / 3.0 + i as f64 * 0.4);
                    let wave2 = f64::sin(app.tick as f64 / 5.0 - i as f64 * 0.2);
                    let combined = ((wave1 + wave2) / 2.0 * 40.0 + 50.0).clamp(0.0, 100.0);
                    let noise = rng.gen_range(0..20);
                    visualizer_data.push((combined as u64 + noise).clamp(0, 100));
                }
            } else {
                visualizer_data.resize(width, 2);
            }

            let sparkline = Sparkline::default()
                .data(&visualizer_data)
                .max(100)
                .style(Style::default().fg(theme.accent));
            f.render_widget(sparkline, player_inner[1]);
        }
    }

    let progress_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(if app.is_audio_mode {
            player_inner[2]
        } else {
            player_inner[1]
        });

    f.render_widget(
        Paragraph::new(Span::styled(
            fmt_clock(status.time_pos),
            Style::default().fg(theme.text_dim),
        )),
        progress_chunks[0],
    );

    let pct = if status.duration > 0.0 {
        ((status.time_pos / status.duration) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };
    let gauge_bg = if status.loading {
        theme.border_inactive
    } else {
        theme.modal_bg
    };

    let gauge = Gauge::default()
        .percent(pct)
        .gauge_style(
            Style::default()
                .fg(theme.accent)
                .bg(gauge_bg)
                .add_modifier(Modifier::BOLD),
        )
        .use_unicode(true)
        .label("");
    f.render_widget(gauge, progress_chunks[1]);

    f.render_widget(
        Paragraph::new(Span::styled(
            fmt_clock(status.duration),
            Style::default().fg(theme.text_dim),
        )),
        progress_chunks[2],
    );

    // ---- Download Progress Bar & Status Bar ----
    if let Some(dl) = &app.active_download {
        let color = if dl.finished {
            theme.success
        } else if dl.progress < 20.0 {
            theme.error
        } else if dl.progress < 80.0 {
            theme.warning
        } else {
            theme.success
        };

        let dl_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(root[3]);

        let (status_line, progress_line) = if dl.finished {
            let anim = FINISH_ANIM[(app.tick / 8) as usize % FINISH_ANIM.len()];
            let status = format!("  {} Download Complete! '{}'", anim, dl.title);
            let bar_width = (f.size().width as usize).saturating_sub(12).max(10);
            let bar_str = "█".repeat(bar_width);
            let line = Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(bar_str, Style::default().fg(theme.success)),
                Span::styled(
                    " 100%",
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
            ]);
            (status, line)
        } else {
            let speed_str = dl.speed.as_deref().unwrap_or("— B/s");
            let size_str = dl.size.as_deref().unwrap_or("— MiB");
            let eta_str = dl.eta.as_deref().unwrap_or("—:—");
            let title_trunc: String = dl.title.chars().take(40).collect();
            let status = format!(
                "  {}  •  Size: {}  •  Speed: {}  •  ETA: {}",
                title_trunc, size_str, speed_str, eta_str
            );

            let bar_width = (f.size().width as usize).saturating_sub(12).max(10);
            let progress_fraction = (dl.progress / 100.0) * bar_width as f64;
            let filled_chars = progress_fraction.floor() as usize;
            let fraction = progress_fraction - progress_fraction.floor();
            let fraction_idx = (fraction * 8.0).round() as usize;

            let fraction_chars = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
            let filled_part = "█".repeat(filled_chars);
            let leading_char = if fraction_idx > 0 && fraction_idx < 8 {
                fraction_chars[fraction_idx]
            } else if fraction_idx == 8 {
                "█"
            } else {
                ""
            };

            let unfilled_len = bar_width
                .saturating_sub(filled_chars)
                .saturating_sub(if leading_char.is_empty() { 0 } else { 1 });

            let mut spans = Vec::new();
            spans.push(Span::raw("  "));
            spans.push(Span::styled(filled_part, Style::default().fg(color)));
            if !leading_char.is_empty() {
                spans.push(Span::styled(leading_char, Style::default().fg(color)));
            }
            if unfilled_len > 0 {
                spans.push(Span::styled(
                    "░".repeat(unfilled_len),
                    Style::default().fg(theme.border_inactive),
                ));
            }
            spans.push(Span::styled(
                format!(" {:.1}%", dl.progress),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ));

            (status, Line::from(spans))
        };

        f.render_widget(
            Paragraph::new(Span::styled(
                status_line,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.modal_bg)),
            dl_layout[0],
        );
        f.render_widget(
            Paragraph::new(progress_line).style(Style::default().bg(theme.modal_bg)),
            dl_layout[1],
        );
    } else if let Some((msg, expiry)) = &app.notification {
        if app.tick < *expiry {
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("  {}  ", msg),
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(theme.modal_bg)),
                root[3],
            );
        } else {
            app.notification = None;
        }
    } else {
        let status_text = Span::styled(
            "   ? Help   |   q Quit   |   / Search   |   S Subscriptions   |   r Refresh   |   u Subscribe   |   P Playlists   |   D Download",
            Style::default().fg(theme.text_dim),
        );
        f.render_widget(
            Paragraph::new(status_text).style(Style::default().bg(theme.modal_bg)),
            root[3],
        );
    }

    // ---- Floating Modals ----

    // 1. Select Playlist Modal (Triggered by 's')
    if let Mode::SelectPlaylistModal(_) = app.mode {
        let area = centered_rect(40, 50, f.size());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(format!(" {} Select Playlist to Add Video ", ICON_PLAYLIST))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.modal_bg));

        let items: Vec<ListItem> = app
            .playlists
            .iter()
            .map(|p| {
                ListItem::new(Span::styled(
                    format!(" {} ({} videos) ", p.name, p.videos.len()),
                    Style::default().fg(theme.fg),
                ))
            })
            .collect();

        // FIX: Extract formatted string to a variable that lives long enough
        let pointer_symbol = format!(" {} ", ICON_POINTER);
        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(theme.accent_dim)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(&pointer_symbol);

        f.render_stateful_widget(list, area, &mut app.playlist_modal_state);
    }

    // 2. Help Menu
    if app.show_help {
        let area = centered_rect(50, 65, f.size());
        f.render_widget(Clear, area);

        let key_style = Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(theme.fg);

        let help_text = vec![
            Line::from(Span::styled(
                "   Keyboard Shortcuts",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("     /          ", key_style),
                Span::styled("Focus Search Bar", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     ?          ", key_style),
                Span::styled("Toggle Help Menu", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     q          ", key_style),
                Span::styled("Quit Application", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     Tab        ", key_style),
                Span::styled("Switch Panes / Columns", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "   Navigation & Actions",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("     j / ↓      ", key_style),
                Span::styled("Move Selection Down", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     k / ↑      ", key_style),
                Span::styled("Move Selection Up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     Enter      ", key_style),
                Span::styled("Play Selected / Confirm Action", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     m          ", key_style),
                Span::styled("Play Selected Audio (Headless)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     D          ", key_style),
                Span::styled("Open Download Menu (Shift+D)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     P          ", key_style),
                Span::styled("Open Playlists Menu (Shift+P)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     S          ", key_style),
                Span::styled("Open Subscriptions (Shift+S)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     r          ", key_style),
                Span::styled("Refresh Subscription Feed", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     u          ", key_style),
                Span::styled("Subscribe / Unsubscribe Channel", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     c          ", key_style),
                Span::styled("Create New Empty Playlist", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     s          ", key_style),
                Span::styled("Add Selected Video to Playlist", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     a          ", key_style),
                Span::styled("Add Selected Video to Queue", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     d          ", key_style),
                Span::styled("Remove Queue / Delete Playlist / Unsubscribe", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     l / →      ", key_style),
                Span::styled("Expand Playlist View", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     h / ←      ", key_style),
                Span::styled("Collapse Playlist and View Header", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "   Playback Controls",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("     Space      ", key_style),
                Span::styled("Play / Pause Current Track", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     h / l      ", key_style),
                Span::styled("Seek Back / Forward 5s (Outside Playlists)", desc_style),
            ]),
            Line::from(vec![
                Span::styled("     - / +      ", key_style),
                Span::styled("Volume Down / Up 5%", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "     Press any key to close",
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::ITALIC),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.modal_bg));

        f.render_widget(Paragraph::new(help_text).block(block), area);
    }
}
