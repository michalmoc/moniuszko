use crate::config::Config;
use crate::data::track::TrackId;
use crate::db::database::Database;
use crate::db::database_builder::DatabaseBuilder;
use crate::db::file_data::{AlbumIdentification, FileData};
use crate::db::musicbrainz::MusicBrainz;
use crate::db::traverse_files::FilesDatabase;
use anyhow::anyhow;
use itertools::Itertools;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use ustr::Ustr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default)]
pub struct Scanner {
    files_database: FilesDatabase,
    music_brainz: MusicBrainz,
    files: HashMap<PathBuf, FileData>,
    #[serde(with = "scanner_serde")]
    covers: HashMap<AlbumIdentification, Ustr>,
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
        info!(
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

    // TODO: remove mut
    pub fn make_database(&mut self) -> Database {
        let mut music_db = DatabaseBuilder::default();
        let mut books_db = DatabaseBuilder::default();

        for (path, data) in &self.files {
            if data
                .genres
                .iter()
                .any(|s| s.to_ascii_lowercase() == "audiobook")
            {
                books_db.add_file(path, data, &mut self.music_brainz, &self.covers);
            } else {
                music_db.add_file(path, data, &mut self.music_brainz, &self.covers);
            }
        }

        Database {
            music: music_db.build(),
            books: books_db.build(),
        }
    }
}

fn make_cover(album: &AlbumIdentification, some_file: &Path, config: &Config) -> Ustr {
    let cover_name = match album {
        AlbumIdentification::None => {
            return config.album_placeholder_path().to_string_lossy().into();
        }
        AlbumIdentification::MusicBrainz { uuid, .. } => uuid.to_string(),
        AlbumIdentification::Custom { title, sort } => {
            let h1 = title.precomputed_hash();
            let h2 = sort.precomputed_hash();
            format!("{};;;{};;;{}", h1, h2, title)
        }
    };
    let mut cover_path = config.covers_path().join(cover_name);
    cover_path.add_extension("png");

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
            .save(&cover_path)
            .map_err(anyhow::Error::from)
    })();

    if result.is_err() {
        config.album_placeholder_path().to_string_lossy().into()
    } else {
        cover_path.to_string_lossy().into()
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

mod scanner_serde {
    use super::AlbumIdentification;
    use serde::{Deserializer, Serializer};
    use ustr::Ustr;

    type Attr = std::collections::HashMap<AlbumIdentification, Ustr>;

    pub(super) fn serialize<S: Serializer>(attr: &Attr, ser: S) -> Result<S::Ok, S::Error> {
        let attr: Vec<_> = attr.iter().collect();
        serde::Serialize::serialize(&attr, ser)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(des: D) -> Result<Attr, D::Error> {
        let attr: Vec<_> = serde::Deserialize::deserialize(des)?;
        Ok(attr.into_iter().collect())
    }
}

pub type ScannerPtr = Arc<RwLock<Scanner>>;
