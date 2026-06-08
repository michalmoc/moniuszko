use crate::data::album::AlbumId;
use crate::data::artist::ArtistId;
use adw::glib;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize, glib::Boxed)]
#[boxed_type(name = "TrackId")]
pub struct TrackId(Uuid);

impl TrackId {
    pub fn new() -> Self {
        TrackId(Uuid::now_v7())
    }
}

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,

    pub title: Ustr,
    pub title_sort: Ustr,

    pub album: AlbumId,
    pub cd: Option<u32>,
    pub position: Option<u32>,

    pub artists: Option<Ustr>,
    pub artist_ids: HashSet<ArtistId>,

    pub duration: Duration,
    pub year: Option<u16>,
    pub genres: HashSet<Ustr>,
}
