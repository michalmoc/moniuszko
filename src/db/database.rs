use crate::data::album::{Album, AlbumId};
use crate::data::artist::{Artist, ArtistId};
use crate::data::genre::Genre;
use crate::data::object_id::ObjectId;
use crate::data::track::{Track, TrackId};
use crate::db::search_result::SearchResult;
use crate::languages::COLLATOR;
use gtk4::glib;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Index;
use std::sync::{Arc, RwLock};
use ustr::Ustr;

#[derive(Default)]
pub struct Subdatabase {
    pub tracks: HashMap<TrackId, Track>,
    pub albums: HashMap<AlbumId, Album>,
    pub years: BTreeMap<Option<u16>, HashSet<AlbumId>>,
    pub artists: HashMap<ArtistId, Artist>,
    pub genres: BTreeMap<Option<Ustr>, Genre>,
}

fn cmp(a: &str, b: &str) -> Ordering {
    COLLATOR.wait().compare(a, b)
}

fn cmp_o<T: AsRef<str>>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => cmp(a.as_ref(), b.as_ref()),
    }
}

impl Subdatabase {
    pub fn has_track(&self, track_id: TrackId) -> bool {
        self.tracks.contains_key(&track_id)
    }

    pub fn sorted_tracks(&self) -> Vec<TrackId> {
        let mut tracks = self.tracks.iter().collect_vec();
        tracks.sort_by(|(_, t1), (_, t2)| cmp(&t1.title_sort, &t2.title_sort));
        tracks.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_tracks_of_album(&self, album_id: AlbumId) -> Vec<TrackId> {
        if let Some(album) = self.albums.get(&album_id) {
            let mut tracks = album.unordered_tracks.clone();
            tracks.sort_by(|t1, t2| {
                COLLATOR
                    .wait()
                    .compare(&self[*t1].title_sort, &self[*t2].title_sort)
            });
            tracks.extend(self[album_id].tracks.values());
            tracks
        } else {
            Vec::new()
        }
    }

    pub fn sorted_albums(&self) -> Vec<AlbumId> {
        let mut albums = self.albums.iter().collect_vec();
        albums.sort_by(|(_, t1), (_, t2)| cmp(&t1.title_sort, &t2.title_sort));
        albums.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_albums_of_artist(&self, artist_id: ArtistId) -> Vec<AlbumId> {
        if let Some(artist) = self.artists.get(&artist_id) {
            let mut albums = artist
                .albums
                .iter()
                .map(|a| (*a, self[*a].title_sort))
                .collect_vec();
            albums.sort_by(|a, b| cmp(&a.1, &b.1));
            albums.into_iter().map(|a| a.0).collect_vec()
        } else {
            Vec::new()
        }
    }

    pub fn sorted_albums_of_year(&self, year: Option<u16>) -> Vec<AlbumId> {
        if let Some(year) = self.years.get(&year) {
            let mut albums = year.iter().map(|a| (*a, self[*a].title_sort)).collect_vec();
            albums.sort_by(|a, b| cmp(&a.1, &b.1));
            albums.into_iter().map(|a| a.0).collect_vec()
        } else {
            Vec::new()
        }
    }

    pub fn sorted_albums_of_genre(&self, genre: Option<Ustr>) -> Vec<AlbumId> {
        if let Some(genre) = self.genres.get(&genre) {
            let mut albums = genre
                .albums
                .iter()
                .map(|a| (*a, self[*a].title_sort))
                .collect_vec();
            albums.sort_by(|a, b| cmp(&a.1, &b.1));
            albums.into_iter().map(|a| a.0).collect_vec()
        } else {
            Vec::new()
        }
    }

    pub fn sorted_artists(&self) -> Vec<ArtistId> {
        let mut artists = self.artists.iter().collect_vec();
        artists.sort_by(|(_, a), (_, b)| cmp_o(&a.sort, &b.sort));
        artists.into_iter().map(|(id, _)| *id).collect()
    }

    pub fn sorted_artists_of_genre(&self, genre: Option<Ustr>) -> Vec<ArtistId> {
        if let Some(genre) = self.genres.get(&genre) {
            let mut artists = genre
                .artists
                .iter()
                .map(|a| (*a, self[*a].sort))
                .collect_vec();
            artists.sort_by(|a, b| cmp_o(&a.1, &b.1));
            artists.into_iter().map(|a| a.0).collect_vec()
        } else {
            Vec::new()
        }
    }

    pub fn sorted_years(&self) -> impl Iterator<Item = Option<u16>> {
        self.years.keys().copied()
    }

    pub fn sorted_years_of_artist(&self, artist_id: ArtistId) -> Vec<Option<u16>> {
        if let Some(artist) = self.artists.get(&artist_id) {
            let mut years = artist.albums.iter().map(|a| self[*a].year).collect_vec();

            years.sort_unstable();
            years.dedup();

            years
        } else {
            Vec::new()
        }
    }

    pub fn sorted_years_of_genre(&self, genre: Option<Ustr>) -> Vec<Option<u16>> {
        if let Some(genre) = self.genres.get(&genre) {
            let mut years = genre.albums.iter().map(|a| self[*a].year).collect_vec();

            years.sort_unstable();
            years.dedup();

            years
        } else {
            Vec::new()
        }
    }

    pub fn sorted_genres(&self) -> Vec<Option<Ustr>> {
        let mut genres = self.genres.keys().copied().collect_vec();
        genres.sort_by(cmp_o);
        genres
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

    fn keyword_in_track(&self, keyword: &str, track: &Track) -> bool {
        if track.title.contains(keyword) {
            return true;
        }

        if let Some(artists) = track.artists
            && artists.contains(keyword)
        {
            return true;
        }

        if self[track.album].title.contains(keyword) {
            return true;
        }

        false
    }

    pub fn search(&self, text: &str) -> SearchResult {
        let keywords = text.split_whitespace().collect_vec();

        if keywords.is_empty() {
            return SearchResult::default();
        }

        let mut tracks = HashSet::new();
        let mut albums = HashSet::new();
        let mut artists = HashSet::new();
        let mut genres = HashSet::new();
        let mut years = HashSet::new();

        for (track_id, track) in &self.tracks {
            if keywords.iter().all(|k| self.keyword_in_track(k, track)) {
                tracks.insert(*track_id);
                albums.insert(track.album);
                artists.extend(&track.artist_ids);
                years.insert(track.year);

                if track.genres.is_empty() {
                    genres.insert(None);
                } else {
                    genres.extend(track.genres.iter().copied().map(Some));
                }
            }
        }

        SearchResult {
            tracks: Some(tracks),
            albums: Some(albums),
            years: Some(years),
            artists: Some(artists),
            genres: Some(genres),
        }
    }
}

impl Index<TrackId> for Subdatabase {
    type Output = Track;

    fn index(&self, index: TrackId) -> &Self::Output {
        self.tracks.get(&index).unwrap()
    }
}

impl Index<AlbumId> for Subdatabase {
    type Output = Album;

    fn index(&self, index: AlbumId) -> &Self::Output {
        self.albums.get(&index).unwrap()
    }
}

impl Index<ArtistId> for Subdatabase {
    type Output = Artist;

    fn index(&self, index: ArtistId) -> &Self::Output {
        self.artists.get(&index).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Default, Copy, Clone, glib::Enum)]
#[enum_type(name = "AvailableDatabases")]
pub enum AvailableDatabases {
    #[default]
    Music,
    Books,
}

#[derive(Default)]
pub struct Database {
    pub music: Subdatabase,
    pub books: Subdatabase,
}

impl Database {
    pub fn get_subdb(&self, db: AvailableDatabases) -> &Subdatabase {
        match db {
            AvailableDatabases::Music => &self.music,
            AvailableDatabases::Books => &self.books,
        }
    }

    pub fn any_has_track(&self, track_id: TrackId) -> bool {
        self.music.has_track(track_id) || self.books.has_track(track_id)
    }

    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.music
            .tracks
            .get(&track_id)
            .or_else(|| self.books.tracks.get(&track_id))
    }

    pub fn get_artist(&self, artist_id: ArtistId) -> Option<&Artist> {
        self.music
            .artists
            .get(&artist_id)
            .or_else(|| self.books.artists.get(&artist_id))
    }

    pub fn get_album(&self, album_id: AlbumId) -> Option<&Album> {
        self.music
            .albums
            .get(&album_id)
            .or_else(|| self.books.albums.get(&album_id))
    }

    pub fn all_sorted_tracks_of_album(&self, album_id: AlbumId) -> Vec<TrackId> {
        let mut ret = self.music.sorted_tracks_of_album(album_id);
        ret.append(&mut self.books.sorted_tracks_of_album(album_id));
        ret
    }

    pub fn all_sorted_albums_of_artist(&self, artist_id: ArtistId) -> Vec<AlbumId> {
        let mut ret = self.music.sorted_albums_of_artist(artist_id);
        ret.append(&mut self.books.sorted_albums_of_artist(artist_id));
        ret
    }

    pub fn all_sorted_albums_of_year(&self, year: Option<u16>) -> Vec<AlbumId> {
        let mut ret = self.music.sorted_albums_of_year(year);
        ret.append(&mut self.books.sorted_albums_of_year(year));
        ret
    }

    pub fn all_sorted_albums_of_genre(&self, genre: Option<Ustr>) -> Vec<AlbumId> {
        let mut ret = self.music.sorted_albums_of_genre(genre);
        ret.append(&mut self.books.sorted_albums_of_genre(genre));
        ret
    }
}

pub type DatabasePtr = Arc<RwLock<Database>>;
