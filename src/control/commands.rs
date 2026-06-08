use crate::config::ConfigPtr;
use crate::control::playback_state::PlaybackState;
use crate::control::playlist_store::PlaylistStore;
use crate::data::object_id::{ObjectId, ObjectIds};
use crate::data::playlist_entry_uuid::{PlaylistEntryUuid, PlaylistEntryUuids};
use crate::data::track::TrackId;
use crate::db::database::{Database, DatabasePtr};
use crate::db::scan::{Scanner, ScannerPtr};
use crate::ui::playlist_item::PlaylistItem;
use crate::ui::window::Window;
use adw::glib;
use adw::glib::clone;
use async_channel::Receiver;
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::ops::Deref;

pub enum Command {
    Raise,
    HideShow,
    Quit,

    Next,
    Pause,
    PlayPause,
    Stop,
    Play,
    PlayFromPlaylist(u32),
    Previous,
    Seek(i64),

    RefreshPlaylist,
    ClearPlaylist,
    RemoveFromPlaylist(PlaylistEntryUuids),
    AppendToPlaylist(ObjectIds),
    InsertInPlaylist(ObjectIds, u32),

    RepopulateMediaLibrary,
    RefreshMediaLibrary,
    ClearMediaLibrary,
}

pub async fn process_commands(
    queue: Receiver<Command>,
    window: Window,
    database: DatabasePtr,
    config: ConfigPtr,
    scanner: ScannerPtr,
) {
    let playlist = window.playlist();
    let playback_state = window.playback();

    loop {
        match queue.recv().await.unwrap() {
            Command::Raise => {
                window.present();
            }
            Command::Quit => {
                window.set_hide_on_close(false);
                window.close();
            }
            Command::HideShow => {
                window.set_visible(!window.is_visible());
            }
            Command::Next => {
                on_next(&playlist, &playback_state);
            }
            Command::PlayPause => {
                on_play_pause(&playlist, &playback_state);
            }
            Command::Pause => {
                on_pause(&playback_state);
            }
            Command::Stop => {
                on_stop(&playback_state);
            }
            Command::Play => {
                on_play(&playback_state);
            }
            Command::Previous => {
                on_previous(&playlist, &playback_state);
            }
            Command::Seek(pos) => {
                on_seek(&playback_state, pos);
            }
            Command::PlayFromPlaylist(pos) => {
                on_play_from_playlist(&playlist, &playback_state, pos);
            }
            Command::RefreshPlaylist => refresh_playlist(&playlist, &database.read().unwrap()),
            Command::ClearPlaylist => clear_playlist(&playlist),
            Command::RemoveFromPlaylist(to_remove) => {
                remove_from_playlist(&playlist, to_remove.borrow())
            }
            Command::AppendToPlaylist(obj) => {
                append_to_playlist(&playlist, &database.read().unwrap(), obj)
            }
            Command::InsertInPlaylist(obj, pos) => {
                insert_in_playlist(&playlist, &database.read().unwrap(), obj, pos)
            }
            Command::RepopulateMediaLibrary => window.repopulate_media_library(),
            Command::RefreshMediaLibrary => refresh_library(
                window.clone(),
                database.clone(),
                config.clone(),
                scanner.clone(),
            ),
            Command::ClearMediaLibrary => {
                clear_media_library(&window, database.clone(), &playlist, scanner.clone())
            }
        }
    }
}

pub fn on_play_from_playlist(playlist: &PlaylistStore, playback_state: &PlaybackState, pos: u32) {
    if pos < playlist.len() {
        playback_state.set_current(playlist.get(pos));
        playback_state.set_playing(true);
    }
}

fn on_pause(playback_state: &PlaybackState) {
    if playback_state.current().is_some() {
        playback_state.set_playing(false);
    }
}

fn on_stop(playback_state: &PlaybackState) {
    if playback_state.current().is_some() {
        playback_state.set_playing(false);
        playback_state.seek(0);
    }
}

fn on_play(playback_state: &PlaybackState) {
    if playback_state.current().is_some() {
        playback_state.set_playing(true);
    }
}

fn on_seek(playback_state: &PlaybackState, offset: i64) {
    if playback_state.current().is_some() {
        let current = playback_state.progress();
        playback_state.seek(current + offset);
    }
}

fn on_play_pause(playlist: &PlaylistStore, playback_state: &PlaybackState) {
    if playback_state.current().is_none() {
        if playlist.len() > 0 {
            let item = playlist.get(0).unwrap();
            playback_state.set_current(Some(item));
            playback_state.set_playing(true);
        }
    } else {
        playback_state.set_playing(!playback_state.playing());
    }
}

fn on_next(playlist: &PlaylistStore, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current() {
        if let Some(idx) = playlist.find(&current) {
            // playlist.n_items() != 0 because current present
            let next = playback_state.repeat_mode().next(idx, playlist.len());
            playback_state.set_current(playlist.get(next));
            playback_state.set_playing(true);
        } else {
            playback_state.set_current(None::<PlaylistItem>);
            on_play_pause(playlist, playback_state);
        }
    } else {
        on_play_pause(playlist, playback_state);
    }
}

fn on_previous(playlist: &PlaylistStore, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current() {
        if let Some(idx) = playlist.find(&current) {
            if playback_state.progress() * 10 > playback_state.duration() {
                playback_state.seek(0);
            } else {
                // playlist.n_items() != 0 because current present
                let next = playback_state.repeat_mode().previous(idx, playlist.len());
                playback_state.set_current(playlist.get(next));
                playback_state.set_playing(true);
            }
        } else {
            playback_state.set_current(None::<PlaylistItem>);
            on_play_pause(playlist, playback_state);
        }
    } else {
        on_play_pause(playlist, playback_state);
    }
}

fn refresh_playlist(playlist: &PlaylistStore, database: &Database) {
    for item in playlist.iter() {
        item.set_data(&database);
    }
}

fn clear_playlist(playlist: &PlaylistStore) {
    playlist.remove_all();
}

fn remove_from_playlist(playlist: &PlaylistStore, to_remove: &HashSet<PlaylistEntryUuid>) {
    playlist.retain(|item| !to_remove.contains(&item.uuid()));
}

fn get_tracks(database: &Database, item: ObjectId) -> Vec<TrackId> {
    match item {
        ObjectId::None => {
            vec![]
        }
        ObjectId::TrackId(track_id) => {
            vec![track_id]
        }
        ObjectId::AlbumId(album_id) => database.sorted_tracks_of_album(album_id),
        ObjectId::ArtistId(artist) => {
            let albums = database.sorted_albums_of_artist(artist);
            albums
                .into_iter()
                .flat_map(|a| database.sorted_tracks_of_album(a))
                .collect()
        }
        ObjectId::Genre(genre) => {
            let albums = database.sorted_albums_of_genre(genre);
            albums
                .into_iter()
                .flat_map(|a| database.sorted_tracks_of_album(a))
                .collect()
        }
        ObjectId::Year(year) => {
            let albums = database.sorted_albums_of_year(year);
            albums
                .into_iter()
                .flat_map(|a| database.sorted_tracks_of_album(a))
                .collect()
        }
    }
}

pub fn append_to_playlist(playlist: &PlaylistStore, database: &Database, object_ids: ObjectIds) {
    for item in object_ids {
        for track in get_tracks(&database, item) {
            playlist.append(&PlaylistItem::new(track, &database));
        }
    }
}

pub fn insert_in_playlist(
    playlist: &PlaylistStore,
    database: &Database,
    object_ids: ObjectIds,
    pos: u32,
) {
    let tracks = object_ids
        .into_iter()
        .flat_map(|o| get_tracks(&database, o));

    for track in tracks.rev() {
        playlist.insert(pos, &PlaylistItem::new(track, &database));
    }
}

pub fn refresh_library(
    window: Window,
    database: DatabasePtr,
    config: ConfigPtr,
    scanner: ScannerPtr,
) {
    glib::spawn_future_local(async move {
        window.refresh_button().set_sensitive(false);

        gio::spawn_blocking(clone!(
            #[weak]
            scanner,
            #[weak]
            config,
            #[weak]
            database,
            move || {
                let mut scanner = scanner.write().unwrap();
                scanner.scan(&config.read().unwrap().media_path, &config.read().unwrap());
                let db = scanner.make_database();

                fs::create_dir_all(config.read().unwrap().database_path().parent().unwrap())
                    .unwrap();
                let file = File::create(config.read().unwrap().database_path()).unwrap();
                serde_json::to_writer(file, scanner.deref()).unwrap();

                *database.write().unwrap() = db;
            }
        ))
        .await
        .expect("Task needs to finish successfully.");

        window.repopulate_media_library();
        refresh_playlist(&window.playlist(), &database.read().unwrap());

        window.refresh_button().set_sensitive(true);
    });
}

pub fn clear_media_library(
    window: &Window,
    database: DatabasePtr,
    playlist_store: &PlaylistStore,
    scanner: ScannerPtr,
) {
    *database.write().unwrap() = Database::default();
    *scanner.write().unwrap() = Scanner::default();
    window.repopulate_media_library();
    clear_playlist(&playlist_store)
}
