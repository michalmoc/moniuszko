use gtk4::glib;
use rand::random_range;

#[derive(glib::Enum, Copy, Clone, Default, Debug)]
#[enum_type(name = "RepeatMode")]
pub enum RepeatMode {
    Single,
    #[default]
    All,
    Shuffle,
}

impl RepeatMode {
    pub fn next(&self, current: u32, n_items: u32) -> u32 {
        assert!(n_items > 0);
        match self {
            RepeatMode::Single => current,
            RepeatMode::All => (current + 1) % n_items,
            RepeatMode::Shuffle => random_range(0..n_items),
        }
    }

    pub fn previous(&self, current: u32, n_items: u32) -> u32 {
        assert!(n_items > 0);
        match self {
            RepeatMode::Single => current,
            RepeatMode::All => (current + n_items - 1) % n_items,
            RepeatMode::Shuffle => random_range(0..n_items),
        }
    }
}
