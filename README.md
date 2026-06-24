<p align="center">
  <img src="assets/logo.svg" width="320" alt="ytplay-tui logo"/>
</p>

<h1 align="center">ytplay-tui</h1>

<p align="center">
  <a href="https://github.com/ucheema/ytplay-tui/actions"><img src="https://github.com/ucheema/ytplay-tui/workflows/CI/badge.svg" alt="CI Status"/></a>
  <a href="https://crates.io/crates/ytplay-tui"><img src="https://img.shields.io/badge/crates.io-v0.1.0-orange.svg" alt="Crates.io"/></a>
  <a href="https://github.com/ucheema/ytplay-tui/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"/></a>
  <a href="https://rust-lang.org"><img src="https://img.shields.io/badge/rust-1.74%2B-blue.svg" alt="Rust Version"/></a>
</p>

<p align="center">
  <strong>ytplay-tui</strong> is a lightweight, high-performance terminal user interface (TUI) for searching, streaming, queuing, and downloading YouTube content. Written in Rust using the <code>ratatui</code> and <code>crossterm</code> libraries, it controls <code>mpv</code> via IPC sockets for playback and utilizes <code>yt-dlp</code> for downloads.
</p>

---

## ✨ Features

- **🔍 Smart Search & Feed**: Fast scrape-based YouTube search with an intuitive interface.
- **📺 Media Playback via `mpv` IPC**:
  - Direct video/audio streaming using healthy, randomly selected public Invidious API instances as a fallback.
  - Bidirectional communication with `mpv` via local UNIX sockets for state synchronization.
  - Adjust volume, toggle pause, seek, and track current playing time.
  - Play in full video mode or stream **audio-only** to save bandwidth.
- **📁 Playback Queue**: Queue up multiple videos and advance automatically when the current track finishes.
- **💾 Local Playlist Management**: Create custom playlists stored locally in `~/.config/yttui/playlists.json`.
- **🔔 Channel Subscriptions**: Subscribe to your favorite creators and view their latest uploads directly via RSS feeds (`~/.config/yttui/subscriptions.json`).
- **📥 Background Downloads**: Download videos (best quality, 1080p, 720p) or convert them to MP3 using `yt-dlp` in the background with real-time speed, size, and ETA updates.
- **🎨 Premium Visuals**: Beautiful Gruvbox-themed layout with rich Nerd Font icon integration.

---

## 📋 Prerequisites

Before running `ytplay-tui`, make sure you have the following installed on your system and available in your `PATH`:

1. **Rust**: Cargo and Rust compiler (version 1.74+).
2. **mpv**: The media player used for background playback.
3. **yt-dlp**: Required for background downloading and stream parsing.
4. **Nerd Font**: Required to correctly display terminal icons (e.g., [FiraCode Nerd Font](https://github.com/ryanoasis/nerd-fonts)).

### Install Prerequisites (Linux/macOS)

#### Debian/Ubuntu
```bash
sudo apt update
sudo apt install mpv git-core
# Install yt-dlp (recommended to install latest from official repo or via pip)
sudo wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
sudo chmod a+rx /usr/local/bin/yt-dlp
```

#### Arch Linux
```bash
sudo pacman -S mpv yt-dlp
```

#### macOS (Homebrew)
```bash
brew install mpv yt-dlp
```

---

## 🚀 Installation

Build `ytplay-tui` from source:

```bash
# Clone the repository
git clone https://github.com/ucheema/ytplay-tui.git
cd ytplay-tui

# Build the release version
cargo build --release

# The compiled binary is located at:
# ./target/release/ytplay-tui
```

To install the binary globally:

```bash
cargo install --path .
```

### 🖥️ Desktop Integration (Linux)

You can install a desktop launcher and application icon to open `ytplay-tui` straight from your application menu:

```bash
# 1. Install the desktop launcher file
cp assets/ytplay-tui.desktop ~/.local/share/applications/

# 2. Install the application icon
mkdir -p ~/.local/share/icons/hicolor/scalable/apps/
cp assets/icon.svg ~/.local/share/icons/hicolor/scalable/apps/ytplay-tui.svg

# 3. Update desktop database (optional)
update-desktop-database ~/.local/share/applications/
```

*Note: Make sure your globally compiled cargo binary path (typically `~/.cargo/bin`) is added to your environment `PATH` variable so the launcher can find the `ytplay-tui` command.*

---

## 🎮 Keybindings & Controls

Press `?` inside the app at any time to open the interactive Help popup.

### General Controls
| Key | Action |
|---|---|
| `q` / `Ctrl+C` | Quit the application |
| `?` | Toggle Help Menu |
| `/` | Enter Search Mode |
| `Esc` | Clear Search / Return to Normal Mode |
| `Tab` | Switch Focus between main Panes |

### Navigation & Playback
| Key | Action |
|---|---|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Stream selected video |
| `m` | Stream selected video as **Audio-Only** |
| `a` | Add selected video to the playback Queue |
| `d` | Delete selected item (remove from queue/playlist, unsubscribe, etc.) |
| `Space` | Pause / Resume playback |
| `l` / `→` | Seek forward 5 seconds |
| `h` / `←` | Seek backward 5 seconds |
| `+` / `=` | Increase volume |
| `-` | Decrease volume |
| `N` | Skip to the next video in the queue |

### Views & Special Panels
| Key | Action |
|---|---|
| `P` | Toggle local Playlists pane |
| `S` | Toggle RSS Subscriptions pane |
| `D` | Show Download options for the selected video |
| `c` | Create a new local playlist |
| `s` | Add selected video to a playlist |
| `u` | Subscribe/Unsubscribe to the selected video's channel |
| `r` | Refresh feed (when Subscriptions pane is focused) |

*Note: Inside the **Subscriptions** pane, use `Tab` to toggle between the **Channels List** and the **Videos List**.*

---

## 📁 File Structure & Configuration

`ytplay-tui` stores user playlists and channel subscriptions locally as JSON files. They are automatically created upon running the app:

- **Playlists Storage**: `~/.config/yttui/playlists.json`
- **Subscriptions Storage**: `~/.config/yttui/subscriptions.json`
- **Downloads Location**: Videos are downloaded to your system's default `~/Downloads` folder.

---

## 🛠️ Architecture

- **Async Tasks**: YouTube scraping and video metadata fetching run on separate background worker threads to keep the UI buttery smooth.
- **mpv UNIX Socket Controller**: Controls `mpv` by spinning up a subprocess using an IPC server socket at `/tmp/yttui-mpv-[PID].sock`. This ensures clean resource management when quitting.
- **Fallback Stream Fetching**: If scraping fails or `mpv` cannot stream directly, `ytplay-tui` requests video details from a public Invidious API instance to obtain direct HTTP format streams.

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
