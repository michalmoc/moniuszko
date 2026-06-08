use gtk4::glib;

#[derive(glib::Enum, Copy, Clone, Default, Debug, PartialEq, Eq)]
#[enum_type(name = "PlaybackStatus")]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Paused,
    Playing,
}

impl PlaybackStatus {
    pub fn to_str(&self) -> &str {
        match self {
            PlaybackStatus::Stopped => "Stopped",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Playing => "Playing",
        }
    }
}
