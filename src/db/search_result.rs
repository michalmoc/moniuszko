use crate::data::album::AlbumId;
use crate::data::artist::ArtistId;
use crate::data::track::TrackId;
use std::collections::HashSet;
use ustr::Ustr;

/// None means full set
#[derive(Default)]
pub struct SearchResult {
    pub tracks: Option<HashSet<TrackId>>,
    pub albums: Option<HashSet<AlbumId>>,
    pub years: Option<HashSet<Option<u16>>>,
    pub artists: Option<HashSet<ArtistId>>,
    pub genres: Option<HashSet<Option<Ustr>>>,
}

impl SearchResult {
    pub fn has_track(&self, id: TrackId) -> bool {
        self.tracks
            .as_ref()
            .map(|container| container.contains(&id))
            .unwrap_or(true)
    }

    pub fn has_album(&self, id: AlbumId) -> bool {
        self.albums
            .as_ref()
            .map(|container| container.contains(&id))
            .unwrap_or(true)
    }

    pub fn has_year(&self, id: Option<u16>) -> bool {
        self.years
            .as_ref()
            .map(|container| container.contains(&id))
            .unwrap_or(true)
    }

    pub fn has_artist(&self, id: ArtistId) -> bool {
        self.artists
            .as_ref()
            .map(|container| container.contains(&id))
            .unwrap_or(true)
    }

    pub fn has_genre(&self, id: Option<Ustr>) -> bool {
        self.genres
            .as_ref()
            .map(|container| container.contains(&id))
            .unwrap_or(true)
    }
}
