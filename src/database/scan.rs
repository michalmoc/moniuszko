use crate::config::Config;
use crate::database::musicbrainz::MusicBrainz;
use crate::database::traverse_files::FilesDatabase;
use crate::database::{Album, AlbumId, Artist, ArtistId, Database, Genre, Track, TrackId};
use adw::{gdk, glib};
use anyhow::anyhow;
use gtk4::prelude::TextureExt;
use image::ImageFormat;
use itertools::Itertools;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::create_dir_all;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
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

    genres: Vec<Ustr>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Scanner {
    files_database: FilesDatabase,
    music_brainz: MusicBrainz,
    files: HashMap<PathBuf, FileData>,
    covers: HashMap<AlbumIdentification, PathBuf>,
}

impl Scanner {
    fn ensure_cover(&mut self, album: &AlbumIdentification, file_path: &Path, config: &Config) {
        if !self.covers.contains_key(album) {
            self.covers
                .insert(album.clone(), make_cover(album, file_path, config));
        }
    }

    pub fn scan(&mut self, path: &Path, config: &Config) {
        create_dir_all(config.covers_path()).unwrap();

        let files = self.files_database.scan(path);
        println!(
            "unchanged: {}, modified: {}, deleted: {}, new: {}",
            files.unchanged.len(),
            files.modified.len(),
            files.deleted.len(),
            files.new.len()
        );

        for file in files.deleted {
            self.files.remove(&file);
        }

        for file in files.new {
            if let Ok(track) = scan_file(&file, None) {
                self.ensure_cover(&track.album, &file, config);
                self.files.insert(file, track);
            }
        }

        for file in files.modified {
            if let Some(track_id) = self.files.get(&file).map(|t| t.track_id) {
                if let Ok(track) = scan_file(&file, Some(track_id)) {
                    self.ensure_cover(&track.album, &file, config);
                    let old = self.files.insert(file, track);
                    debug_assert!(old.is_some());
                } else {
                    self.files.remove(&file);
                }
            } else {
                if let Ok(track) = scan_file(&file, None) {
                    self.ensure_cover(&track.album, &file, config);
                    self.files.insert(file, track);
                }
            }
        }
    }

    pub fn make_database(&mut self) -> Database {
        let mut tracks = HashMap::new();
        let mut albums = HashMap::new();
        let mut artists = HashMap::new();
        let mut genres = BTreeMap::<Option<Ustr>, Genre>::new();
        let mut years: BTreeMap<_, HashSet<_>> = BTreeMap::new();

        let mut known_albums = HashMap::new();
        let mut known_artist_uuids = HashMap::new();
        let mut known_artist_names = HashMap::new();

        for (path, data) in &self.files {
            let mut found_album_artists = HashSet::new();
            let mut found_track_artists = HashSet::new();

            for uuid in &data.album_artists_uuids {
                if let Some(artist_id) = known_artist_uuids.get(uuid) {
                    found_album_artists.insert(*artist_id);
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

                    found_album_artists.insert(new_id);
                };
            }

            for uuid in &data.track_artists_uuids {
                if let Some(artist_id) = known_artist_uuids.get(uuid) {
                    found_track_artists.insert(*artist_id);
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

                    found_track_artists.insert(new_id);
                };
            }

            if found_album_artists.is_empty() {
                // cannot get artists by uuid, so try to use simple tag
                let name = data.album_artists;
                let sort = data.album_artists_sort.or(name);

                let artist_id = if let Some(artist_id) = known_artist_names.get(&(name, sort)) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert((name, sort), new_id);
                    artists.insert(
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
                found_album_artists.insert(artist_id);
            }

            if found_track_artists.is_empty() {
                // cannot get artists by uuid, so try to use simple tag
                let name = data.track_artists;
                let sort = data.track_artists_sort.or(name);

                let artist_id = if let Some(artist_id) = known_artist_names.get(&(name, sort)) {
                    *artist_id
                } else {
                    let new_id = ArtistId::new();

                    known_artist_names.insert((name, sort), new_id);
                    artists.insert(
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
                found_track_artists.insert(artist_id);
            }

            let album = if let Some(album_id) = known_albums.get(&data.album) {
                *album_id
            } else {
                let album_id = AlbumId::new();
                known_albums.insert(data.album.clone(), album_id);

                let cover = self.covers.remove(&data.album).unwrap();

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

                albums.insert(album_id, new_album);

                album_id
            };

            if let Some(position) = data.position {
                let cd = data.cd.unwrap_or_default();
                albums
                    .get_mut(&album)
                    .unwrap()
                    .tracks
                    .insert((cd, position), data.track_id);
            } else {
                albums
                    .get_mut(&album)
                    .unwrap()
                    .unordered_tracks
                    .push(data.track_id);
            }

            if data.genres.is_empty() {
                let entry = genres.entry(None).or_default();
                entry.albums.insert(album);
                entry.artists.extend(&found_album_artists);
                entry.artists.extend(&found_track_artists);
            } else {
                for genre in &data.genres {
                    let entry = genres.entry(Some(*genre)).or_default();
                    entry.albums.insert(album);
                    entry.artists.extend(&found_album_artists);
                    entry.artists.extend(&found_track_artists);
                }
            }

            years.entry(data.year).or_default().insert(album);

            for artist_id in &found_track_artists {
                artists.get_mut(artist_id).unwrap().albums.insert(album);
            }

            for artist_id in &found_album_artists {
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
                    artist_ids: found_track_artists,
                    genres: HashSet::from_iter(data.genres.iter().copied()),
                    year: data.year,
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

fn make_cover(album: &AlbumIdentification, some_file: &Path, config: &Config) -> PathBuf {
    let cover_name = match album {
        AlbumIdentification::None => {
            "no cover".to_string()
            // TODO: return placeholder
        }
        AlbumIdentification::MusicBrainz { uuid, .. } => uuid.to_string(),
        AlbumIdentification::Custom { title, sort } => {
            let h1 = title.precomputed_hash();
            let h2 = sort.precomputed_hash();
            format!("{};;;{};;;{}", h1, h2, title)
        }
    };
    let cover_path = config.covers_path().join(cover_name);

    let result = (|| {
        let tagged_file = Probe::open(some_file)?.read()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
            .ok_or(anyhow!("no tag found"))?;

        let pic = tag
            .get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
            .ok_or(anyhow!("no pictures found"))?;

        let texture = image::load_from_memory(pic.data())?;

        texture
            .thumbnail(128, 128)
            .save_with_format(&cover_path, ImageFormat::Png)
            .map_err(anyhow::Error::from)
    })();

    if let Err(e) = result {
        println!("{}", e);
    }

    // TODO if !result return placeholder
    cover_path
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
                        track_id: id.unwrap_or_else(|| TrackId::new()),
                        title: stem.to_string_lossy().into(),
                        title_sort: stem.to_string_lossy().into(),
                        album: AlbumIdentification::None,
                        cd: Default::default(),
                        position: Default::default(),
                        album_artists: Default::default(),
                        album_artists_sort: Default::default(),
                        album_artists_uuids: Default::default(),
                        track_artists: Default::default(),
                        track_artists_sort: Default::default(),
                        duration,
                        year: None,
                        genres: Default::default(),
                        track_artists_uuids: Default::default(),
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

    let album_title = tag.album().map(|s| Ustr::from(&s));
    let album_sort = tag
        .get_string(ItemKey::AlbumTitleSortOrder)
        .map(|s| Ustr::from(&s))
        .or(album_title);

    let album = if let Some(uuid) = album_uuid {
        AlbumIdentification::MusicBrainz {
            uuid,
            title: album_title.unwrap_or_default(),
            sort: album_sort.unwrap_or_default(),
        }
    } else if let Some(title) = album_title {
        AlbumIdentification::Custom {
            title,
            sort: album_sort.unwrap(), // because .or(album_title)
        }
    } else {
        AlbumIdentification::None
    };

    let position = tag.track();
    let cd = tag.disk();

    let album_artists = tag.get_string(ItemKey::AlbumArtist).map(|s| Ustr::from(&s));
    let album_artists_sort = tag
        .get_string(ItemKey::AlbumArtistSortOrder)
        .map(|s| Ustr::from(&s))
        .or(album_artists);
    let album_artists_uuids = tag
        .get_strings(ItemKey::MusicBrainzReleaseArtistId)
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();
    let track_artists = tag.artist().map(|s| Ustr::from(&s));
    let track_artists_sort = tag
        .get_string(ItemKey::AlbumArtistSortOrder)
        .map(|s| Ustr::from(&s))
        .or(track_artists);
    let track_artists_uuids = tag
        .get_strings(ItemKey::MusicBrainzArtistId)
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
        album,
        cd,
        position,
        album_artists,
        album_artists_sort,
        album_artists_uuids,
        track_artists,
        track_artists_sort,
        track_artists_uuids,
        duration,
        year,
        genres,
    })
}

pub type ScannerPtr = Arc<RwLock<Scanner>>;
