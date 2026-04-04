use gtk4::glib;
use std::collections::HashMap;
use std::ops::Index;
use std::path::Path;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct TrackId(Uuid);

impl TrackId {
    pub fn new() -> Self {
        TrackId(Uuid::now_v7())
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct AlbumId(Uuid);

impl AlbumId {
    pub fn new() -> Self {
        AlbumId(Uuid::now_v7())
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub enum ObjectId {
    #[default]
    None,
    TrackId(TrackId),
    AlbumId(AlbumId),
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

pub struct Track {
    pub title: String,
    pub album: AlbumId,
}

pub struct Album {
    pub title: String,
    pub tracks: Vec<TrackId>,
}

#[derive(Default)]
pub struct Database {
    pub tracks: HashMap<TrackId, Track>,
    pub albums: HashMap<AlbumId, Album>,
    // authors: HashMap<Uuid, Author>,
    // genres: HashMap<String, Genre>,
}

impl Index<TrackId> for Database {
    type Output = Track;

    fn index(&self, index: TrackId) -> &Self::Output {
        self.tracks.get(&index).unwrap()
    }
}

impl Index<AlbumId> for Database {
    type Output = Album;

    fn index(&self, index: AlbumId) -> &Self::Output {
        self.albums.get(&index).unwrap()
    }
}

impl Database {
    pub fn new() -> Database {
        Default::default()
    }

    pub fn load(&mut self, _: &Path) {
        self.tracks.clear();
        self.albums.clear();

        let track_id_1 = TrackId::new();
        let track_id_2 = TrackId::new();
        let track_id_3 = TrackId::new();

        let album_id = AlbumId::new();
        self.albums.insert(
            album_id,
            Album {
                title: "Some album".to_string(),
                tracks: vec![track_id_1, track_id_2, track_id_3],
            },
        );

        self.tracks.insert(
            track_id_1,
            Track {
                title: "A track 1".to_string(),
                album: album_id,
            },
        );
        self.tracks.insert(
            track_id_2,
            Track {
                title: "A track 2".to_string(),
                album: album_id,
            },
        );
        self.tracks.insert(
            track_id_3,
            Track {
                title: "A track 3".to_string(),
                album: album_id,
            },
        );
    }
}

pub type DatabasePtr = Arc<RwLock<Database>>;
