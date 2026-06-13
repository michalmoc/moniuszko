use crate::data::track::TrackId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub enum AlbumIdentification {
    None,
    MusicBrainz { uuid: Uuid, title: Ustr, sort: Ustr },
    Custom { title: Ustr, sort: Ustr },
}

#[derive(Serialize, Deserialize)]
pub struct FileData {
    pub track_id: TrackId,

    pub title: Ustr,
    pub title_sort: Ustr,

    pub album: AlbumIdentification,
    pub cd: Option<u32>,
    pub position: Option<u32>,

    pub album_artists: Option<Ustr>,
    pub album_artists_sort: Option<Ustr>,
    pub album_artists_uuids: HashSet<Uuid>,
    pub track_artists: Option<Ustr>,
    pub track_artists_sort: Option<Ustr>,
    pub track_artists_uuids: HashSet<Uuid>,

    pub duration: Duration,
    pub year: Option<u16>,

    pub genres: Vec<Ustr>,
}
