use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Video {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) uploader: String,
    pub(crate) duration_secs: Option<f64>,
    pub(crate) view_count: Option<u64>,
    pub(crate) channel_id: Option<String>,
}

impl Video {
    pub(crate) fn duration_str(&self) -> String {
        match self.duration_secs {
            Some(s) => fmt_clock(s),
            None => "--:--".to_string(),
        }
    }

    pub(crate) fn views_str(&self) -> String {
        match self.view_count {
            Some(n) if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
            Some(n) if n >= 1_000 => format!("{:.1}K", n as f64 / 1_000.0),
            Some(n) => format!("{n}"),
            None => "—".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Playlist {
    pub(crate) name: String,
    pub(crate) videos: Vec<Video>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Subscription {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum PlaylistItem {
    Header(usize),       // Playlist Index
    Video(usize, usize), // Playlist Index, Video Index
}

pub(crate) fn get_playlists_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.config/yttui/playlists.json", home))
}

pub(crate) fn load_playlists() -> Vec<Playlist> {
    let path = get_playlists_path();
    if let Ok(data) = std::fs::read_to_string(path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub(crate) fn save_playlists(playlists: &[Playlist]) {
    let path = get_playlists_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(playlists) {
        let _ = std::fs::write(path, data);
    }
}

pub(crate) fn get_subscriptions_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.config/yttui/subscriptions.json", home))
}

pub(crate) fn load_subscriptions() -> Vec<Subscription> {
    let path = get_subscriptions_path();
    if let Ok(data) = std::fs::read_to_string(path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub(crate) fn save_subscriptions(subs: &[Subscription]) {
    let path = get_subscriptions_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(subs) {
        let _ = std::fs::write(path, data);
    }
}

pub(crate) fn fmt_clock(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let total = secs.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
