use crate::data::object_id::ObjectIds;
use crate::data::playlist_entry_uuid::PlaylistEntryUuids;
use adw::glib;

#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "DndItem")]
pub enum DndItem {
    Add(ObjectIds),
    Move(PlaylistEntryUuids),
}
