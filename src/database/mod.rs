mod musicbrainz;
mod scan;
mod traverse_files;

use gtk4::glib;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ArtistId(Uuid);

impl ArtistId {
    pub fn new() -> Self {
        ArtistId(Uuid::now_v7())
    }
}

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

pub struct Album {
    pub title: Ustr,
    pub title_sort: Ustr,

    pub year: Option<u16>, // TODO: allow multiple
    unordered_tracks: Vec<TrackId>,
    tracks: BTreeMap<(u32, u32), TrackId>,
}

pub struct Artist {
    pub uuid: Uuid,
    pub name: Option<Ustr>,
    pub sort: Option<Ustr>,
    albums: HashSet<AlbumId>,
}

#[derive(Default)]
pub struct Genre {
    albums: HashSet<AlbumId>,
    artists: HashSet<ArtistId>,
}

#[derive(Default)]
pub struct Database {
    tracks: HashMap<TrackId, Track>,
    albums: HashMap<AlbumId, Album>,
    years: BTreeMap<Option<u16>, HashSet<AlbumId>>,
    artists: HashMap<ArtistId, Artist>,
    genres: BTreeMap<Option<Ustr>, Genre>,
}

impl Database {
    pub fn has_track(&self, track_id: TrackId) -> bool {
        self.tracks.contains_key(&track_id)
    }

    pub fn sorted_tracks(&self) -> Vec<TrackId> {
        let mut tracks = self.tracks.iter().collect_vec();
        tracks.sort_by_key(|(_, t)| t.title_sort);
        tracks.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_tracks_of_album(&self, album_id: AlbumId) -> Vec<TrackId> {
        let mut tracks = self[album_id].unordered_tracks.clone();
        tracks.sort_by_key(|t| self[*t].title_sort);
        tracks.extend(self[album_id].tracks.values());

        tracks
    }

    pub fn sorted_albums(&self) -> Vec<AlbumId> {
        let mut albums = self.albums.iter().collect_vec();
        albums.sort_by_key(|(_, t)| t.title_sort);
        albums.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_albums_of_artist(&self, artist_id: ArtistId) -> Vec<AlbumId> {
        let mut albums = self[artist_id]
            .albums
            .iter()
            .map(|a| (*a, self[*a].title_sort))
            .collect_vec();
        albums.sort_by_key(|a| a.1);
        albums.into_iter().map(|a| a.0).collect_vec()
    }

    pub fn sorted_albums_of_year(&self, year: Option<u16>) -> Vec<AlbumId> {
        let mut albums = self.years[&year]
            .iter()
            .map(|a| (*a, self[*a].title_sort))
            .collect_vec();
        albums.sort_by_key(|a| a.1);
        albums.into_iter().map(|a| a.0).collect_vec()
    }

    pub fn sorted_albums_of_genre(&self, genre: Option<Ustr>) -> Vec<AlbumId> {
        let mut albums = self.genres[&genre]
            .albums
            .iter()
            .map(|a| (*a, self[*a].title_sort))
            .collect_vec();
        albums.sort_by_key(|a| a.1);
        albums.into_iter().map(|a| a.0).collect_vec()
    }

    pub fn sorted_artists(&self) -> Vec<ArtistId> {
        let mut artists = self.artists.iter().collect_vec();
        artists.sort_by_key(|(_, a)| a.sort);
        artists.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_artists_of_genre(&self, genre: Option<Ustr>) -> Vec<ArtistId> {
        let mut artists = self.genres[&genre]
            .artists
            .iter()
            .map(|a| (*a, self[*a].sort))
            .collect_vec();
        artists.sort_by_key(|a| a.1);
        artists.into_iter().map(|a| a.0).collect_vec()
    }

    pub fn sorted_years(&self) -> impl Iterator<Item = Option<u16>> {
        self.years.keys().copied()
    }

    pub fn sorted_years_of_artist(&self, artist_id: ArtistId) -> Vec<Option<u16>> {
        let mut years = self[artist_id]
            .albums
            .iter()
            .map(|a| self[*a].year)
            .collect_vec();

        years.sort_unstable();
        years.dedup();

        years
    }

    pub fn sorted_years_of_genre(&self, genre: Option<Ustr>) -> Vec<Option<u16>> {
        let mut years = self.genres[&genre]
            .albums
            .iter()
            .map(|a| self[*a].year)
            .collect_vec();

        years.sort_unstable();
        years.dedup();

        years
    }

    pub fn sorted_genres(&self) -> impl Iterator<Item = Option<Ustr>> {
        self.genres.keys().copied()
    }

    pub fn track_matches_filter(&self, track_id: TrackId, filter: &Vec<ObjectId>) -> bool {
        let track = &self[track_id];

        for object in filter {
            match object {
                ObjectId::None => {}
                ObjectId::TrackId(t) => {
                    if *t != track_id {
                        return false;
                    }
                }
                ObjectId::AlbumId(album_id) => {
                    if track.album != *album_id {
                        return false;
                    }
                }
                ObjectId::ArtistId(artist_id) => {
                    if !track.artist_ids.contains(&artist_id) {
                        return false;
                    }
                }
                ObjectId::Genre(genre) => {
                    if let Some(genre) = genre {
                        if !track.genres.contains(&genre) {
                            return false;
                        }
                    } else {
                        if !track.genres.is_empty() {
                            return false;
                        }
                    }
                }
                ObjectId::Year(year) => {
                    if track.year != *year {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn album_matches_filter(&self, album_id: AlbumId, filter: &Vec<ObjectId>) -> bool {
        let album = &self[album_id];

        for object in filter {
            match object {
                ObjectId::None => {}
                ObjectId::TrackId(t) => {
                    if self[*t].album != album_id {
                        return false;
                    }
                }
                ObjectId::AlbumId(a) => {
                    if *a != album_id {
                        return false;
                    }
                }
                ObjectId::ArtistId(artist_id) => {
                    if !self[*artist_id].albums.contains(&album_id) {
                        return false;
                    }
                }
                ObjectId::Genre(genre) => {
                    if !self.genres[genre].albums.contains(&album_id) {
                        return false;
                    }
                }
                ObjectId::Year(year) => {
                    if !self.years[year].contains(&album_id) {
                        return false;
                    }
                }
            }
        }

        true
    }
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

impl Index<ArtistId> for Database {
    type Output = Artist;

    fn index(&self, index: ArtistId) -> &Self::Output {
        self.artists.get(&index).unwrap()
    }
}

impl Database {
    pub fn new() -> Database {
        Default::default()
    }
}

pub type DatabasePtr = Arc<RwLock<Database>>;
