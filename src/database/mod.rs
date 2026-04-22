mod scan;
mod traverse_files;

use gtk4::glib;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Index;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use ustr::Ustr;
use uuid::Uuid;

pub use scan::{Scanner, ScannerPtr};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize, glib::Boxed)]
#[boxed_type(name = "TrackId")]
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

#[derive(Default, Copy, Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ObjectId")]
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

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,

    pub title: Ustr,
    pub album: AlbumId,
    pub cd: u32,
    pub position: u32,
    pub artists: Ustr,
    pub duration: Duration,
}

pub struct Album {
    pub title: Ustr,
    pub tracks: BTreeMap<(u32, u32), TrackId>,
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
}

pub type DatabasePtr = Arc<RwLock<Database>>;
