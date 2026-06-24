use crate::downloader::{ActiveDownload, DOWNLOAD_OPTIONS, DlMessage, spawn_download};
use crate::invidious::get_direct_stream_url;
use crate::models::{
    Playlist, PlaylistItem, Subscription, Video, load_playlists, load_subscriptions,
    save_playlists, save_subscriptions,
};
use crate::player::{MpvCommand, PlayerStatus};
use crate::scraper::{SearchOutcome, SubVideosOutcome, fetch_channel_videos, spawn_search};
use crate::theme::{ICON_QUEUE, Theme};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::thread;

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Pane {
    Feed,
    Queue,
    Download,
    Playlists,
    Subscriptions,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum SubPane {
    Channels,
    Videos,
}

pub(crate) enum Mode {
    Normal,
    SearchInput,
    CreatePlaylistInput,
    SelectPlaylistModal(Video),
}

pub(crate) struct App {
    pub(crate) theme: Theme,
    pub(crate) mode: Mode,
    pub(crate) search_query: String,
    pub(crate) search_buffer: String,
    pub(crate) searching: bool,
    pub(crate) search_seq: u64,
    pub(crate) search_err: Option<String>,
    pub(crate) search_tx: Sender<SearchOutcome>,
    pub(crate) search_rx: Receiver<SearchOutcome>,

    pub(crate) results: Vec<Video>,
    pub(crate) results_state: ListState,

    pub(crate) queue: Vec<Video>,
    pub(crate) queue_state: ListState,

    // Playlists
    pub(crate) playlists: Vec<Playlist>,
    pub(crate) playlists_state: ListState,
    pub(crate) expanded_playlists: HashSet<String>,
    pub(crate) playlist_modal_state: ListState,

    // Subscriptions
    pub(crate) subscriptions: Vec<Subscription>,
    pub(crate) subscriptions_state: ListState,
    pub(crate) sub_videos: Vec<Video>,
    pub(crate) sub_videos_state: ListState,
    pub(crate) sub_loading: bool,
    pub(crate) focused_sub_pane: SubPane,
    pub(crate) sub_tx: Sender<SubVideosOutcome>,
    pub(crate) sub_rx: Receiver<SubVideosOutcome>,

    // Download Pane
    pub(crate) download_target: Option<Video>,
    pub(crate) download_state: ListState,
    pub(crate) dl_tx: Sender<DlMessage>,
    pub(crate) dl_rx: Receiver<DlMessage>,
    pub(crate) active_download: Option<ActiveDownload>,

    pub(crate) is_audio_mode: bool,
    pub(crate) notification: Option<(String, u64)>,

    pub(crate) focused: Pane,
    pub(crate) now_playing: Option<Video>,
    pub(crate) show_help: bool,
    pub(crate) should_quit: bool,
    pub(crate) tick: u64,

    pub(crate) player_status: Arc<Mutex<PlayerStatus>>,
    pub(crate) mpv_cmd_tx: Sender<MpvCommand>,
    pub(crate) finished_rx: Receiver<()>,
}

impl App {
    pub(crate) fn new(
        player_status: Arc<Mutex<PlayerStatus>>,
        mpv_cmd_tx: Sender<MpvCommand>,
        finished_rx: Receiver<()>,
    ) -> Self {
        let (search_tx, search_rx) = mpsc::channel();
        let (dl_tx, dl_rx) = mpsc::channel();
        let (sub_tx, sub_rx) = mpsc::channel();

        App {
            theme: Theme::gruvbox(),
            mode: Mode::Normal,
            search_query: String::new(),
            search_buffer: String::new(),
            searching: false,
            search_seq: 0,
            search_err: None,
            search_tx,
            search_rx,
            results: Vec::new(),
            results_state: ListState::default(),
            queue: Vec::new(),
            queue_state: ListState::default(),

            playlists: load_playlists(),
            playlists_state: ListState::default(),
            expanded_playlists: HashSet::new(),
            playlist_modal_state: ListState::default(),

            subscriptions: load_subscriptions(),
            subscriptions_state: ListState::default(),
            sub_videos: Vec::new(),
            sub_videos_state: ListState::default(),
            sub_loading: false,
            focused_sub_pane: SubPane::Channels,
            sub_tx,
            sub_rx,

            download_target: None,
            download_state: ListState::default(),
            dl_tx,
            dl_rx,
            active_download: None,

            is_audio_mode: false,
            notification: None,

            focused: Pane::Feed,
            now_playing: None,
            show_help: false,
            should_quit: false,
            tick: 0,
            player_status,
            mpv_cmd_tx,
            finished_rx,
        }
    }

    pub(crate) fn get_flat_playlists(&self) -> Vec<PlaylistItem> {
        let mut flat = Vec::new();
        for (p_idx, p) in self.playlists.iter().enumerate() {
            flat.push(PlaylistItem::Header(p_idx));
            if self.expanded_playlists.contains(&p.name) {
                for (v_idx, _) in p.videos.iter().enumerate() {
                    flat.push(PlaylistItem::Video(p_idx, v_idx));
                }
            }
        }
        flat
    }

    pub(crate) fn move_selection(&mut self, delta: i32) {
        if let Mode::SelectPlaylistModal(_) = self.mode {
            let len = self.playlists.len();
            if len == 0 {
                return;
            }
            let current = self.playlist_modal_state.selected().unwrap_or(0) as i32;
            let next = (current + delta).rem_euclid(len as i32);
            self.playlist_modal_state.select(Some(next as usize));
            return;
        }

        let (len, state) = match self.focused {
            Pane::Feed => (self.results.len(), &mut self.results_state),
            Pane::Queue => (self.queue.len(), &mut self.queue_state),
            Pane::Download => (DOWNLOAD_OPTIONS.len(), &mut self.download_state),
            Pane::Playlists => (self.get_flat_playlists().len(), &mut self.playlists_state),
            Pane::Subscriptions => match self.focused_sub_pane {
                SubPane::Channels => (self.subscriptions.len(), &mut self.subscriptions_state),
                SubPane::Videos => (self.sub_videos.len(), &mut self.sub_videos_state),
            },
        };
        if len == 0 {
            return;
        }
        let current = state.selected().unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(len as i32);
        state.select(Some(next as usize));

        if self.focused == Pane::Subscriptions && self.focused_sub_pane == SubPane::Channels {
            self.load_selected_channel_videos();
        }
    }

    pub(crate) fn toggle_pane(&mut self) {
        self.focused = match self.focused {
            Pane::Feed => {
                if self.queue.is_empty() {
                    Pane::Feed
                } else {
                    Pane::Queue
                }
            }
            Pane::Queue => Pane::Feed,
            Pane::Download | Pane::Playlists | Pane::Subscriptions => {
                self.download_target = None;
                Pane::Feed
            }
        };
    }

    pub(crate) fn play_video(&mut self, video: Video, audio_only: bool) {
        let tx = self.mpv_cmd_tx.clone();
        let title = video.title.clone();
        let vid_id = video.id.clone();

        self.now_playing = Some(video);
        self.is_audio_mode = audio_only;

        {
            let mut st = self.player_status.lock().unwrap();
            st.loading = true;
            st.fallback_title = Some(format!("Fetching stream... {}", title));
            st.media_title = None;
        }

        thread::spawn(move || {
            let direct_url = match get_direct_stream_url(&vid_id) {
                Some(url) => url,
                None => format!("https://www.youtube.com/watch?v={}", vid_id),
            };

            let _ = tx.send(MpvCommand::Load(direct_url, title, audio_only));
        });
    }

    pub(crate) fn activate_selected(&mut self, audio_only: bool) {
        match self.focused {
            Pane::Feed => {
                if let Some(v) = self
                    .results_state
                    .selected()
                    .and_then(|i| self.results.get(i).cloned())
                {
                    self.play_video(v, audio_only);
                }
            }
            Pane::Queue => {
                if let Some(i) = self
                    .queue_state
                    .selected()
                    .filter(|&i| i < self.queue.len())
                {
                    let v = self.queue.remove(i);
                    clamp_selection(&mut self.queue_state, self.queue.len());
                    if self.queue.is_empty() {
                        self.focused = Pane::Feed;
                    }
                    self.play_video(v, audio_only);
                }
            }
            Pane::Playlists => {
                let flat = self.get_flat_playlists();
                if let Some(item) = self
                    .playlists_state
                    .selected()
                    .and_then(|idx| flat.get(idx))
                {
                    match item {
                        PlaylistItem::Header(p_idx) => {
                            let pl = &self.playlists[*p_idx];
                            if !pl.videos.is_empty() {
                                self.queue = pl.videos.clone();
                                self.focused = Pane::Queue;
                                self.advance_queue_force_mode(audio_only);
                            } else {
                                self.notification =
                                    Some(("Playlist is empty!".to_string(), self.tick + 100));
                            }
                        }
                        PlaylistItem::Video(p_idx, v_idx) => {
                            let pl = &self.playlists[*p_idx];
                            self.queue = pl.videos[*v_idx..].to_vec();
                            self.focused = Pane::Queue;
                            self.advance_queue_force_mode(audio_only);
                        }
                    }
                }
            }
            Pane::Subscriptions => {
                if self.focused_sub_pane == SubPane::Videos {
                    #[allow(clippy::collapsible_if)]
                    if let Some(v) = self
                        .sub_videos_state
                        .selected()
                        .and_then(|i| self.sub_videos.get(i).cloned())
                    {
                        self.play_video(v, audio_only);
                    }
                }
            }
            Pane::Download => {
                if let Some((idx, v)) = self
                    .download_state
                    .selected()
                    .zip(self.download_target.as_ref())
                {
                    let tx = self.dl_tx.clone();
                    spawn_download(v.id.clone(), idx, tx);

                    self.active_download = Some(ActiveDownload {
                        title: v.title.clone(),
                        progress: 0.0,
                        size: None,
                        speed: None,
                        eta: None,
                        finished: false,
                        finish_tick: 0,
                    });
                }
                self.focused = Pane::Feed;
                self.download_target = None;
            }
        }
    }

    pub(crate) fn queue_selected(&mut self) {
        let selected_vid = match self.focused {
            Pane::Feed => {
                if let Some(i) = self.results_state.selected() {
                    self.results.get(i).cloned()
                } else {
                    None
                }
            }
            Pane::Subscriptions => {
                if self.focused_sub_pane == SubPane::Videos {
                    if let Some(i) = self.sub_videos_state.selected() {
                        self.sub_videos.get(i).cloned()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(v) = selected_vid {
            let title = v.title.clone();
            self.queue.push(v);
            if self.queue_state.selected().is_none() {
                self.queue_state.select(Some(0));
            }
            self.notification = Some((format!("{} Queued: {}", ICON_QUEUE, title), self.tick + 70));
        }
    }

    pub(crate) fn remove_selected_from_pane(&mut self) {
        match self.focused {
            Pane::Queue => {
                if let Some(i) = self
                    .queue_state
                    .selected()
                    .filter(|&i| i < self.queue.len())
                {
                    self.queue.remove(i);
                    clamp_selection(&mut self.queue_state, self.queue.len());
                    if self.queue.is_empty() {
                        self.focused = Pane::Feed;
                    }
                }
            }
            Pane::Playlists => {
                let flat = self.get_flat_playlists();
                if let Some(item) = self
                    .playlists_state
                    .selected()
                    .and_then(|idx| flat.get(idx))
                {
                    match item {
                        PlaylistItem::Header(p_idx) => {
                            let removed = self.playlists.remove(*p_idx);
                            self.expanded_playlists.remove(&removed.name);
                            save_playlists(&self.playlists);
                            self.notification = Some((
                                format!("Deleted playlist: {}", removed.name),
                                self.tick + 100,
                            ));
                        }
                        PlaylistItem::Video(p_idx, v_idx) => {
                            let pl = &mut self.playlists[*p_idx];
                            let removed = pl.videos.remove(*v_idx);
                            save_playlists(&self.playlists);
                            self.notification = Some((
                                format!("Removed {} from playlist", removed.title),
                                self.tick + 100,
                            ));
                        }
                    }
                    let new_len = self.get_flat_playlists().len();
                    clamp_selection(&mut self.playlists_state, new_len);
                    if self.playlists.is_empty() {
                        self.focused = Pane::Feed;
                    }
                }
            }
            Pane::Subscriptions if self.focused_sub_pane == SubPane::Channels => {
                if let Some(idx) = self
                    .subscriptions_state
                    .selected()
                    .filter(|&idx| idx < self.subscriptions.len())
                {
                    let removed = self.subscriptions.remove(idx);
                    save_subscriptions(&self.subscriptions);
                    self.notification = Some((
                        format!("Unsubscribed from: {}", removed.name),
                        self.tick + 100,
                    ));
                    let new_len = self.subscriptions.len();
                    clamp_selection(&mut self.subscriptions_state, new_len);
                    self.sub_videos.clear();
                    self.sub_videos_state.select(None);
                    if self.subscriptions.is_empty() {
                        self.focused = Pane::Feed;
                    } else {
                        self.load_selected_channel_videos();
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn advance_queue(&mut self) {
        let mode = self.is_audio_mode;
        self.advance_queue_force_mode(mode);
    }

    pub(crate) fn advance_queue_force_mode(&mut self, audio_only: bool) {
        if self.queue.is_empty() {
            self.now_playing = None;
            return;
        }
        let next = self.queue.remove(0);
        clamp_selection(&mut self.queue_state, self.queue.len());
        if self.queue.is_empty() && self.focused == Pane::Queue {
            self.focused = Pane::Feed;
        }
        self.play_video(next, audio_only);
    }

    pub(crate) fn start_search(&mut self) {
        let query = self.search_buffer.trim().to_string();
        self.mode = Mode::Normal;
        if query.is_empty() {
            return;
        }
        self.search_query = query.clone();
        self.search_seq += 1;
        self.searching = true;
        self.search_err = None;
        spawn_search(query, self.search_seq, self.search_tx.clone());
    }

    pub(crate) fn load_selected_channel_videos(&mut self) {
        self.sub_videos.clear();
        self.sub_videos_state.select(None);
        if let Some(idx) = self
            .subscriptions_state
            .selected()
            .filter(|&idx| idx < self.subscriptions.len())
        {
            let ch_id = self.subscriptions[idx].id.clone();

            // Add any matching videos from current search results immediately
            for v in &self.results {
                if v.channel_id.as_ref() == Some(&ch_id) {
                    self.sub_videos.push(v.clone());
                }
            }
            if !self.sub_videos.is_empty() {
                self.sub_videos_state.select(Some(0));
            }

            self.sub_loading = true;
            let tx = self.sub_tx.clone();
            thread::spawn(move || {
                let result = fetch_channel_videos(&ch_id);
                let _ = tx.send(SubVideosOutcome {
                    channel_id: ch_id,
                    result,
                });
            });
        }
    }

    pub(crate) fn toggle_subscription_selected(&mut self) {
        let selected_vid = match self.focused {
            Pane::Feed => self
                .results
                .get(self.results_state.selected().unwrap_or(0))
                .cloned(),
            Pane::Queue => self
                .queue
                .get(self.queue_state.selected().unwrap_or(0))
                .cloned(),
            Pane::Playlists => {
                let flat = self.get_flat_playlists();
                if let Some(idx) = self.playlists_state.selected() {
                    if let Some(PlaylistItem::Video(p_idx, v_idx)) = flat.get(idx) {
                        self.playlists[*p_idx].videos.get(*v_idx).cloned()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Pane::Subscriptions => {
                if self.focused_sub_pane == SubPane::Videos {
                    self.sub_videos
                        .get(self.sub_videos_state.selected().unwrap_or(0))
                        .cloned()
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(v) = selected_vid {
            if let Some(ch_id) = v.channel_id {
                let exists = self.subscriptions.iter().position(|s| s.id == ch_id);
                if let Some(pos) = exists {
                    let removed = self.subscriptions.remove(pos);
                    save_subscriptions(&self.subscriptions);
                    self.notification = Some((
                        format!("Unsubscribed from: {}", removed.name),
                        self.tick + 100,
                    ));

                    let new_len = self.subscriptions.len();
                    clamp_selection(&mut self.subscriptions_state, new_len);
                    if self.focused == Pane::Subscriptions {
                        self.sub_videos.clear();
                        self.sub_videos_state.select(None);
                        if self.subscriptions.is_empty() {
                            self.focused = Pane::Feed;
                        } else {
                            self.load_selected_channel_videos();
                        }
                    }
                } else {
                    let new_sub = Subscription {
                        id: ch_id,
                        name: v.uploader.clone(),
                    };
                    self.subscriptions.push(new_sub);
                    save_subscriptions(&self.subscriptions);
                    self.notification =
                        Some((format!("Subscribed to: {}", v.uploader), self.tick + 100));

                    if self.focused == Pane::Subscriptions {
                        let new_len = self.subscriptions.len();
                        self.subscriptions_state.select(Some(new_len - 1));
                        self.load_selected_channel_videos();
                    }
                }
            } else {
                self.notification = Some((
                    "Channel metadata not available for this video".to_string(),
                    self.tick + 100,
                ));
            }
        }
    }
}

pub(crate) fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else {
        let idx = state.selected().unwrap_or(0).min(len - 1);
        state.select(Some(idx));
    }
}
