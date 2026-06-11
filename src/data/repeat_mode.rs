use crate::control::random_data::RandomData;
use gtk4::glib;

#[derive(glib::Enum, Copy, Clone, Default, Debug)]
#[enum_type(name = "RepeatMode")]
pub enum RepeatMode {
    Single,
    #[default]
    All,
    Shuffle,
}

impl RepeatMode {
    pub fn next(&self, current: u32, n_items: u32, rng: &mut RandomData) -> u32 {
        assert!(n_items > 0);
        match self {
            RepeatMode::Single => current,
            RepeatMode::All => (current + 1) % n_items,
            RepeatMode::Shuffle => rng.next(current),
        }
    }

    pub fn previous(&self, current: u32, n_items: u32, rng: &mut RandomData) -> u32 {
        assert!(n_items > 0);
        match self {
            RepeatMode::Single => current,
            RepeatMode::All => (current + n_items - 1) % n_items,
            RepeatMode::Shuffle => rng.prev(current),
        }
    }
}
