use crate::data::album::{Album, AlbumId};
use crate::data::artist::{Artist, ArtistId};
use crate::data::genre::Genre;
use crate::data::track::{Track, TrackId};
use crate::db::database::Subdatabase;
use crate::db::file_data::{AlbumIdentification, FileData};
use crate::db::musicbrainz::MusicBrainz;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Default)]
pub struct DatabaseBuilder {
    tracks: HashMap<TrackId, Track>,
    albums: HashMap<AlbumId, Album>,
    artists: HashMap<ArtistId, Artist>,
    genres: BTreeMap<Option<Ustr>, Genre>,
    years: BTreeMap<Option<u16>, HashSet<AlbumId>>,

    known_albums: HashMap<AlbumIdentification, AlbumId>,
    known_artist_uuids: HashMap<Uuid, ArtistId>,
    known_artist_names: HashMap<(Option<Ustr>, Option<Ustr>), ArtistId>,
}

#[derive(Default)]
struct FoundArtists {
    album: HashSet<ArtistId>,
    track: HashSet<ArtistId>,
}

impl DatabaseBuilder {
    pub fn build(self) -> Subdatabase {
        Subdatabase {
            tracks: self.tracks,
            albums: self.albums,
            years: self.years,
            artists: self.artists,
            genres: self.genres,
        }
    }

    pub fn add_file(
        &mut self,
        path: &Path,
        data: &FileData,
        music_brainz: &mut MusicBrainz,
        covers: &HashMap<AlbumIdentification, Option<Ustr>>,
    ) {
        let found_artists = self.find_artists(music_brainz, data);

        let album = self.find_album(&data, covers);

        self.find_position(data, &album);

        self.find_genres(&data, &found_artists, album);

        self.years.entry(data.year).or_default().insert(album);

        self.fill_artists(&found_artists, album);

        self.tracks.insert(
            data.track_id,
            Track {
                path: path.to_path_buf(),
                title: data.title,
                title_sort: data.title_sort,
                album,
                position: data.position,
                cd: data.cd,
                max_cd: data.max_cd,
                duration: data.duration,
                artists: data.track_artists,
                artist_ids: found_artists.track,
                genres: HashSet::from_iter(data.genres.iter().copied()),
                year: data.year,
            },
        );
    }

    fn fill_artists(&mut self, found_artists: &FoundArtists, album: AlbumId) {
        for artist_id in &found_artists.track {
            self.artists
                .get_mut(artist_id)
                .unwrap()
                .albums
                .insert(album);
        }

        for artist_id in &found_artists.album {
            self.artists
                .get_mut(artist_id)
                .unwrap()
                .albums
                .insert(album);
        }
    }

    fn find_genres(&mut self, data: &&FileData, found_artists: &FoundArtists, album: AlbumId) {
        if data.genres.is_empty() {
            let entry = self.genres.entry(None).or_default();
            entry.albums.insert(album);
            entry.artists.extend(&found_artists.album);
            entry.artists.extend(&found_artists.track);
        } else {
            for genre in &data.genres {
                let entry = self.genres.entry(Some(*genre)).or_default();
                entry.albums.insert(album);
                entry.artists.extend(&found_artists.album);
                entry.artists.extend(&found_artists.track);
            }
        }
    }

    fn find_position(&mut self, data: &FileData, album: &AlbumId) {
        if let Some(position) = data.position {
            let cd = data.cd.unwrap_or_default();
            self.albums
                .get_mut(&album)
                .unwrap()
                .tracks
                .insert((cd, position), data.track_id);
        } else {
            self.albums
                .get_mut(&album)
                .unwrap()
                .unordered_tracks
                .push(data.track_id);
        }
    }

    fn find_album(
        &mut self,
        data: &FileData,
        covers: &HashMap<AlbumIdentification, Option<Ustr>>,
    ) -> AlbumId {
        let album = if let Some(album_id) = self.known_albums.get(&data.album) {
            *album_id
        } else {
            let album_id = AlbumId::new();
            self.known_albums.insert(data.album.clone(), album_id);

            let cover = covers[&data.album];

            let new_album = match data.album {
                AlbumIdentification::None => Album {
                    title: Ustr::default(),
                    title_sort: Ustr::default(),
                    tracks: BTreeMap::new(),
                    unordered_tracks: Vec::new(),
                    year: None,
                    cover,
                },
                AlbumIdentification::MusicBrainz { title, sort, .. }
                | AlbumIdentification::Custom { title, sort } => Album {
                    title,
                    title_sort: sort,
                    tracks: BTreeMap::new(),
                    unordered_tracks: Vec::new(),
                    year: data.year,
                    cover,
                },
            };

            self.albums.insert(album_id, new_album);

            album_id
        };
        album
    }

    fn find_artists(&mut self, music_brainz: &mut MusicBrainz, data: &FileData) -> FoundArtists {
        let mut found_artists = FoundArtists::default();

        for uuid in &data.album_artists_uuids {
            if let Some(artist_id) = self.known_artist_uuids.get(uuid) {
                found_artists.album.insert(*artist_id);
            } else if let Some(name) = music_brainz.get_artist_name(uuid) {
                let new_id = ArtistId::new();

                self.known_artist_uuids.insert(*uuid, new_id);
                self.artists.insert(
                    new_id,
                    Artist {
                        uuid: *uuid,
                        name: Some(name.name),
                        sort: Some(name.sort),
                        albums: Default::default(),
                    },
                );

                found_artists.album.insert(new_id);
            };
        }

        for uuid in &data.track_artists_uuids {
            if let Some(artist_id) = self.known_artist_uuids.get(uuid) {
                found_artists.track.insert(*artist_id);
            } else if let Some(name) = music_brainz.get_artist_name(uuid) {
                let new_id = ArtistId::new();

                self.known_artist_uuids.insert(*uuid, new_id);
                self.artists.insert(
                    new_id,
                    Artist {
                        uuid: *uuid,
                        name: Some(name.name),
                        sort: Some(name.sort),
                        albums: Default::default(),
                    },
                );

                found_artists.track.insert(new_id);
            };
        }

        if found_artists.album.is_empty() {
            // cannot get artists by uuid, so try to use simple tag
            let name = data.album_artists;
            let sort = data.album_artists_sort.or(name);

            let artist_id = if let Some(artist_id) = self.known_artist_names.get(&(name, sort)) {
                *artist_id
            } else {
                let new_id = ArtistId::new();

                self.known_artist_names.insert((name, sort), new_id);
                self.artists.insert(
                    new_id,
                    Artist {
                        uuid: Uuid::nil(),
                        name,
                        sort,
                        albums: Default::default(),
                    },
                );

                new_id
            };
            found_artists.album.insert(artist_id);
        }

        if found_artists.track.is_empty() {
            // cannot get artists by uuid, so try to use simple tag
            let name = data.track_artists;
            let sort = data.track_artists_sort.or(name);

            let artist_id = if let Some(artist_id) = self.known_artist_names.get(&(name, sort)) {
                *artist_id
            } else {
                let new_id = ArtistId::new();

                self.known_artist_names.insert((name, sort), new_id);
                self.artists.insert(
                    new_id,
                    Artist {
                        uuid: Uuid::nil(),
                        name,
                        sort,
                        albums: Default::default(),
                    },
                );

                new_id
            };
            found_artists.track.insert(artist_id);
        }

        found_artists
    }
}
