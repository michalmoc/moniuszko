use crate::database::musicbrainz::MusicBrainz;
use crate::database::traverse_files::FilesDatabase;
use crate::database::{Album, AlbumId, Artist, ArtistId, Database, Track, TrackId};
use itertools::Itertools;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct FileData {
    pub track_id: TrackId,

    pub title: Ustr,
    pub title_sort: Ustr,

    pub album_uuid: Option<Uuid>,
    pub album: Option<Ustr>,
    pub album_sort: Option<Ustr>,
    pub cd: u32,
    pub position: u32,

    pub album_artists: Option<Ustr>,
    pub album_artists_sort: Option<Ustr>,
    pub track_artists: Option<Ustr>,
    pub track_artists_sort: Option<Ustr>,
    pub artists_uuids: HashSet<Uuid>,

    pub duration: Duration,
    pub year: Option<u16>,

    genres: Vec<Ustr>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Scanner {
    files_database: FilesDatabase,
    music_brainz: MusicBrainz,
    files: HashMap<PathBuf, FileData>,
}

impl Scanner {
    pub fn scan(&mut self, path: &Path) {
        let files = self.files_database.scan(path);
        println!(
            "unchanged: {}, moved: {}, modified: {}, deleted: {}, new: {}",
            files.unchanged.len(),
            files.moved.len(),
            files.modified.len(),
            files.deleted.len(),
            files.new.len()
        );

        for file in files.deleted {
            self.files.remove(&file);
        }

        for (old, new) in files.moved {
            if let Some(track) = self.files.remove(&old) {
                self.files.insert(new, track);
            }
        }

        for file in files.new {
            if let Ok(track) = scan_file(&file, None) {
                self.files.insert(file, track);
            }
        }

        for file in files.modified {
            if let Some(track) = self.files.get_mut(&file) {
                if let Ok(t) = scan_file(&file, Some(track.track_id)) {
                    *track = t;
                } else {
                    self.files.remove(&file);
                }
            } else {
                if let Ok(track) = scan_file(&file, None) {
                    self.files.insert(file, track);
                }
            }
        }
    }

    pub fn make_database(&mut self) -> Database {
        let mut tracks = HashMap::new();
        let mut albums = HashMap::new();
        let mut artists = HashMap::new();
        let mut genres = BTreeMap::<Option<Ustr>, HashSet<AlbumId>>::new();
        let mut years: BTreeMap<_, HashSet<_>> = BTreeMap::new();

        let mut known_albums = HashMap::new();
        let mut known_artist_uuids = HashMap::new();
        let mut known_artist_names = HashMap::new();

        for (path, data) in &self.files {
            let mut found_artists = {
                let mut found_artists = HashSet::new();

                for uuid in &data.artists_uuids {
                    if let Some(artist_id) = known_artist_uuids.get(uuid) {
                        found_artists.insert(*artist_id);
                    } else if let Some(name) = self.music_brainz.get_artist_name(uuid) {
                        let new_id = ArtistId::new();

                        known_artist_uuids.insert(*uuid, new_id);
                        artists.insert(
                            new_id,
                            Artist {
                                uuid: *uuid,
                                name: Some(name.name),
                                sort: Some(name.sort),
                                albums: Default::default(),
                            },
                        );

                        found_artists.insert(new_id);
                    };
                }

                found_artists
            };

            if found_artists.is_empty() {
                // cannot get artists by uuid, so try to use simple tag
                let name1 = data.album_artists;
                let sort1 = data.album_artists_sort;
                let name2 = data.track_artists;
                let sort2 = data.track_artists_sort;

                let artist_id1 = if let Some(artist_id) = known_artist_names.get(&(name1, sort1)) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert((name1, sort1), new_id);
                    artists.insert(
                        new_id,
                        Artist {
                            uuid: Uuid::nil(),
                            name: name1,
                            sort: sort1,
                            albums: Default::default(),
                        },
                    );

                    new_id
                };
                found_artists.insert(artist_id1);

                let artist_id2 = if let Some(artist_id) = known_artist_names.get(&(name2, sort2)) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert((name2, sort2), new_id);
                    artists.insert(
                        new_id,
                        Artist {
                            uuid: Uuid::nil(),
                            name: name2,
                            sort: sort2,
                            albums: Default::default(),
                        },
                    );

                    new_id
                };
                found_artists.insert(artist_id2);
            }

            let album = if let Some(album_id) = known_albums.get(&(data.album_uuid, data.album)) {
                *album_id
            } else {
                let album_id = AlbumId::new();
                known_albums.insert((data.album_uuid, data.album), album_id);
                albums.insert(
                    album_id,
                    Album {
                        title: data.album.unwrap_or_default(),
                        title_sort: data.album_sort.unwrap_or_default(),
                        tracks: BTreeMap::new(),
                        year: data.year,
                    },
                );

                album_id
            };

            albums
                .get_mut(&album)
                .unwrap()
                .tracks
                .insert((data.cd, data.position), data.track_id);

            if data.genres.is_empty() {
                genres.entry(None).or_default().insert(album);
            } else {
                for genre in &data.genres {
                    genres.entry(Some(*genre)).or_default().insert(album);
                }
            }

            years.entry(data.year).or_default().insert(album);

            for artist_id in &found_artists {
                artists.get_mut(artist_id).unwrap().albums.insert(album);
            }

            tracks.insert(
                data.track_id,
                Track {
                    path: path.clone(),
                    title: data.title,
                    title_sort: data.title_sort,
                    album,
                    position: data.position,
                    cd: data.cd,
                    duration: data.duration,
                    artists: data.track_artists,
                },
            );
        }

        Database {
            tracks,
            albums,
            years,
            artists,
            genres,
        }
    }
}
fn scan_file(path: &Path, id: Option<TrackId>) -> anyhow::Result<FileData> {
    let tagged_file = Probe::open(path)?.read()?;

    let duration = tagged_file.properties().duration();

    let tag = match tagged_file.primary_tag() {
        Some(primary_tag) => primary_tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => {
                if let Some(stem) = path.file_stem() {
                    return Ok(FileData {
                        // TODO: cd X position is not unique, and therefore they overwrite each other
                        track_id: id.unwrap_or_else(|| TrackId::new()),
                        title: stem.to_string_lossy().into(),
                        title_sort: stem.to_string_lossy().into(),
                        album_uuid: None,
                        album: None,
                        album_sort: None,
                        cd: Default::default(),
                        position: Default::default(),
                        album_artists: Default::default(),
                        album_artists_sort: Default::default(),
                        track_artists: Default::default(),
                        track_artists_sort: Default::default(),
                        artists_uuids: Default::default(),
                        duration,
                        year: None,
                        genres: Default::default(),
                    });
                } else {
                    anyhow::bail!("cannot tag file");
                }
            }
        },
    };

    let title = if let Some(title) = tag.title() {
        title.into()
    } else if let Some(stem) = path.file_stem() {
        stem.to_string_lossy().into()
    } else {
        anyhow::bail!("file has no possible title");
    };
    let title_sort = tag
        .get_string(ItemKey::TrackTitleSortOrder)
        .map(|s| Ustr::from(s))
        .unwrap_or(title);

    let album_uuid = tag
        .get_string(ItemKey::MusicBrainzReleaseId)
        .and_then(|s| Uuid::parse_str(&s).ok());

    let album = tag.album().map(|s| Ustr::from(&s));
    let album_sort = tag
        .get_string(ItemKey::AlbumTitleSortOrder)
        .map(|s| Ustr::from(&s))
        .or(album);

    let position = tag.track().unwrap_or_default();
    let cd = tag.disk().unwrap_or_default();

    let album_artists = tag.get_string(ItemKey::AlbumArtist).map(|s| Ustr::from(&s));
    let album_artists_sort = tag
        .get_string(ItemKey::AlbumArtistSortOrder)
        .map(|s| Ustr::from(&s))
        .or(album_artists);
    let track_artists = tag.artist().map(|s| Ustr::from(&s));
    let track_artists_sort = tag
        .get_string(ItemKey::AlbumArtistSortOrder)
        .map(|s| Ustr::from(&s))
        .or(track_artists);
    let artists_uuids = tag
        .get_strings(ItemKey::MusicBrainzArtistId)
        .chain(tag.get_strings(ItemKey::MusicBrainzReleaseArtistId))
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();

    let year = tag.date().map(|t| t.year);

    let genres = tag
        .get_strings(ItemKey::Genre)
        .map(|s| Ustr::from(s))
        .collect_vec();

    Ok(FileData {
        track_id: id.unwrap_or_else(|| TrackId::new()),
        title,
        title_sort,
        album_uuid,
        album,
        album_sort,
        cd,
        position,
        album_artists,
        album_artists_sort,
        track_artists,
        track_artists_sort,
        artists_uuids,
        duration,
        year,
        genres,
    })
}

pub type ScannerPtr = Arc<RwLock<Scanner>>;
