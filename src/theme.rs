use ratatui::style::Color;

pub(crate) const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub(crate) const FINISH_ANIM: [&str; 4] = ["✨", "🌟", "💫", "⭐"];

pub(crate) struct Theme {
    pub(crate) accent: Color,
    pub(crate) accent_dim: Color,
    pub(crate) fg: Color,
    pub(crate) text_dim: Color,
    pub(crate) border_inactive: Color,
    pub(crate) modal_bg: Color,
    pub(crate) error: Color,
    pub(crate) warning: Color,
    pub(crate) success: Color,
}

impl Theme {
    pub(crate) fn gruvbox() -> Self {
        Theme {
            accent: Color::Rgb(254, 128, 25),         // Gruvbox Orange (#fe8019)
            accent_dim: Color::Rgb(80, 73, 69),       // Gruvbox Bg2 (#504945)
            fg: Color::Rgb(235, 219, 178),            // Gruvbox Fg (#ebdbb2)
            text_dim: Color::Rgb(168, 153, 132),      // Gruvbox Fg4 (#a89984)
            border_inactive: Color::Rgb(102, 92, 84), // Gruvbox Bg3 (#665c54)
            modal_bg: Color::Rgb(40, 40, 40),         // Gruvbox Bg0 (#282828)
            error: Color::Rgb(251, 73, 52),           // Gruvbox Red (#fb4934)
            warning: Color::Rgb(250, 189, 47),        // Gruvbox Yellow (#fabd2f)
            success: Color::Rgb(184, 187, 38),        // Gruvbox Green (#b8bb26)
        }
    }
}

// Nerd Font Icons
pub(crate) const ICON_SEARCH: &str = "";
pub(crate) const ICON_YOUTUBE: &str = "󰗃";
pub(crate) const ICON_QUEUE: &str = "󰺗";
pub(crate) const ICON_PLAY: &str = "󰐊";
pub(crate) const ICON_PAUSE: &str = "󰏤";
pub(crate) const ICON_VOLUME: &str = "󰕾";
pub(crate) const ICON_TIME: &str = "󰥔";
pub(crate) const ICON_VIEWS: &str = "󰈈";
pub(crate) const ICON_POINTER: &str = "󰁔";
pub(crate) const ICON_DOWNLOAD: &str = "󰇚";
pub(crate) const ICON_MUSIC: &str = "";
pub(crate) const ICON_PLAYLIST: &str = "󰕮";
pub(crate) const ICON_BELL: &str = "󰂚";
pub(crate) const ICON_FOLDER: &str = "";
pub(crate) const ICON_FOLDER_OPEN: &str = "";
