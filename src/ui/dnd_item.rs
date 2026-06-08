use crate::data::object_id::{ObjectId, ObjectIds};
use crate::data::playlist_entry_uuid::{PlaylistEntryUuid, PlaylistEntryUuids};
use adw::glib;

#[derive(Default, Clone, glib::Boxed)]
#[boxed_type(name = "DndItem")]
pub struct DndItem(ObjectIds, PlaylistEntryUuids);

impl DndItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_object(&mut self, object_id: ObjectId) {
        self.0.push(object_id)
    }

    pub fn mark_entry_for_removal(&mut self, uuid: PlaylistEntryUuid) {
        self.1.insert(uuid);
    }

    pub fn entries_to_remove(self) -> PlaylistEntryUuids {
        self.1
    }

    pub fn objects(self) -> ObjectIds {
        self.0
    }
}
