use gtk4::glib;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt::Display;
use uuid::Uuid;

#[derive(glib::Boxed, Copy, Clone, Eq, PartialEq, Default, Hash)]
#[boxed_type(name = "PlaylistEntryUuid")]
pub struct PlaylistEntryUuid(Uuid);

impl PlaylistEntryUuid {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Display for PlaylistEntryUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(glib::Boxed, Clone, Eq, PartialEq, Default)]
#[boxed_type(name = "PlaylistEntryUuids")]
pub struct PlaylistEntryUuids(HashSet<PlaylistEntryUuid>);

impl PlaylistEntryUuids {
    pub fn insert(&mut self, uuid: PlaylistEntryUuid) {
        self.0.insert(uuid);
    }

    pub fn contains(&self, uuid: &PlaylistEntryUuid) -> bool {
        self.0.contains(uuid)
    }
}

impl Borrow<HashSet<PlaylistEntryUuid>> for PlaylistEntryUuids {
    fn borrow(&self) -> &HashSet<PlaylistEntryUuid> {
        &self.0
    }
}
