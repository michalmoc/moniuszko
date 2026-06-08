use crate::data::track::TrackId;
use std::collections::BTreeMap;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct AlbumId(Uuid);

impl AlbumId {
    pub fn new() -> Self {
        AlbumId(Uuid::now_v7())
    }
}

pub struct Album {
    pub title: Ustr,
    pub title_sort: Ustr,

    pub year: Option<u16>, // TODO: allow multiple
    pub unordered_tracks: Vec<TrackId>,
    pub tracks: BTreeMap<(u32, u32), TrackId>,

    pub cover: Ustr,
}
