use crate::data::album::AlbumId;
use crate::data::artist::ArtistId;
use crate::data::playlist_entry_uuid::{PlaylistEntryUuid, PlaylistEntryUuids};
use crate::data::track::TrackId;
use adw::glib;
use std::ops::{Deref, DerefMut};
use ustr::Ustr;

#[derive(Default, Copy, Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ObjectId")]
pub enum ObjectId {
    #[default]
    None,
    TrackId(TrackId),
    AlbumId(AlbumId),
    ArtistId(ArtistId),
    Genre(Option<Ustr>),
    Year(Option<u16>),
}

impl From<TrackId> for ObjectId {
    fn from(value: TrackId) -> Self {
        Self::TrackId(value)
    }
}

impl From<AlbumId> for ObjectId {
    fn from(value: AlbumId) -> Self {
        Self::AlbumId(value)
    }
}

impl From<ArtistId> for ObjectId {
    fn from(value: ArtistId) -> Self {
        Self::ArtistId(value)
    }
}

impl From<Option<u16>> for ObjectId {
    fn from(value: Option<u16>) -> Self {
        Self::Year(value)
    }
}

// TODO split into object ids and dnd item
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
