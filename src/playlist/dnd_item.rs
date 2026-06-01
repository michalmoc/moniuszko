use crate::database::ObjectId;
use crate::playlist::ui_item::{PlaylistEntryUuid, PlaylistEntryUuids};
use gio::glib;
use std::ops::{Deref, DerefMut};

// TODO: move to data

#[derive(Default, Clone, glib::Boxed)]
#[boxed_type(name = "ObjectIds")]
pub struct ObjectIds(Vec<ObjectId>, PlaylistEntryUuids);

impl ObjectIds {
    pub fn new() -> Self {
        ObjectIds::default()
    }

    pub fn single(obj: ObjectId) -> Self {
        ObjectIds(vec![obj], PlaylistEntryUuids::default())
    }

    pub fn mark_entry_for_removal(&mut self, uuid: PlaylistEntryUuid) {
        self.1.insert(uuid);
    }

    pub fn entries_to_remove(self) -> PlaylistEntryUuids {
        self.1
    }
}

impl IntoIterator for ObjectIds {
    type Item = ObjectId;
    type IntoIter = <Vec<ObjectId> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for ObjectIds {
    type Target = Vec<ObjectId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ObjectIds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
