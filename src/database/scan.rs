use crate::database::musicbrainz::MusicBrainz;
use crate::database::traverse_files::FilesDatabase;
use crate::database::{Album, AlbumId, Artist, ArtistId, Database, Track, TrackId};
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

    pub album_uuid: Uuid,
    pub album: Ustr,
    pub cd: u32,
    pub position: u32,

    pub album_artists: Ustr,
    pub track_artists: Ustr,
    pub artists_uuids: HashSet<Uuid>,

    pub duration: Duration,
    pub year: Option<u16>,
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
                                name,
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
                let name2 = data.track_artists;

                let artist_id1 = if let Some(artist_id) = known_artist_names.get(&name1) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert(name1, new_id);
                    artists.insert(
                        new_id,
                        Artist {
                            uuid: Uuid::nil(),
                            name: name1,
                            albums: Default::default(),
                        },
                    );

                    new_id
                };
                found_artists.insert(artist_id1);

                let artist_id2 = if let Some(artist_id) = known_artist_names.get(&name2) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert(name2, new_id);
                    artists.insert(
                        new_id,
                        Artist {
                            uuid: Uuid::nil(),
                            name: name2,
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
                        title: data.album,
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

            years.entry(data.year).or_default().insert(album);

            for artist_id in &found_artists {
                artists.get_mut(artist_id).unwrap().albums.insert(album);
            }

            tracks.insert(
                data.track_id,
                Track {
                    path: path.clone(),
                    title: data.title,
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
                        album_uuid: Default::default(),
                        album: Default::default(),
                        cd: Default::default(),
                        position: Default::default(),
                        album_artists: Default::default(),
                        track_artists: Default::default(),
                        artists_uuids: Default::default(),
                        duration,
                        year: None,
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

    let album_uuid = tag
        .get_string(ItemKey::MusicBrainzReleaseId)
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or_default();

    let album = tag.album().map(|s| Ustr::from(&s)).unwrap_or_default();

    let position = tag.track().unwrap_or_default();
    let cd = tag.disk().unwrap_or_default();

    let album_artists = tag
        .get_string(ItemKey::AlbumArtist)
        .map(|s| Ustr::from(&s))
        .unwrap_or_default();
    let track_artists = tag.artist().map(|s| Ustr::from(&s)).unwrap_or_default();
    let artists_uuids = tag
        .get_strings(ItemKey::MusicBrainzArtistId)
        .chain(tag.get_strings(ItemKey::MusicBrainzReleaseArtistId))
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();

    let year = tag.date().map(|t| t.year);

    Ok(FileData {
        track_id: id.unwrap_or_else(|| TrackId::new()),
        title,
        album_uuid,
        album,
        cd,
        position,
        album_artists,
        track_artists,
        artists_uuids,
        duration,
        year,
    })
}

pub type ScannerPtr = Arc<RwLock<Scanner>>;
