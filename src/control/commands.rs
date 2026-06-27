use crate::config::{Config, ConfigPtr};
use crate::control::modify_playlist_action::ModifyPlaylistAction;
use crate::control::playback_state::PlaybackState;
use crate::control::playlist_store::PlaylistStore;
use crate::data::object_id::{ObjectId, ObjectIds};
use crate::data::playlist_entry_uuid::PlaylistEntryUuids;
use crate::data::playlist_uuid::PlaylistUuid;
use crate::data::track::TrackId;
use crate::db::database::{Database, DatabasePtr};
use crate::db::scan::{Scanner, ScannerPtr};
use crate::ui::playlist_item::PlaylistItem;
use crate::ui::window::Window;
use adw::glib;
use adw::glib::clone;
use async_channel::Receiver;
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use log::warn;
use std::fs;
use std::fs::File;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub enum ModifyPlaylistCommand {
    Clear,
    Remove(PlaylistEntryUuids),
    Add(ObjectIds, u32),
    Move(PlaylistEntryUuids, u32),
}

impl ModifyPlaylistCommand {
    fn interpret(self, playlist: &PlaylistStore, database: &Database) -> ModifyPlaylistAction {
        match self {
            ModifyPlaylistCommand::Clear => ModifyPlaylistAction::Remove(
                playlist
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| (idx as u32, item.stored_track()))
                    .collect(),
            ),
            ModifyPlaylistCommand::Remove(entries) => ModifyPlaylistAction::Remove(
                playlist
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, item)| {
                        entries
                            .contains(&item.uuid())
                            .then_some((idx as u32, item.stored_track()))
                    })
                    .collect(),
            ),
            ModifyPlaylistCommand::Add(to_add, pos) => {
                let pos = pos.min(playlist.len());

                ModifyPlaylistAction::Insert(
                    to_add
                        .iter()
                        .flat_map(|i| get_tracks(&database, *i))
                        .enumerate()
                        .map(|(idx, t)| (pos + idx as u32, t))
                        .collect(),
                )
            }
            ModifyPlaylistCommand::Move(to_move, pos) => {
                let pos = pos.min(playlist.len());
                ModifyPlaylistAction::Move(
                    playlist
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, item)| {
                            to_move.contains(&item.uuid()).then_some(idx as u32)
                        })
                        .collect(),
                    pos,
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Raise,
    HideShow,
    Quit,

    Next,
    Pause,
    PlayPause,
    Stop,
    Play,
    PlayFromPlaylist(PlaylistUuid, u32),
    Previous,
    Seek(i64),

    ModifyPlaylist(PlaylistUuid, ModifyPlaylistCommand),
    Undo(PlaylistUuid),
    Redo(PlaylistUuid),

    RefreshMediaLibrary,
    ClearMediaLibrary,

    ResetRandomData,
}

pub async fn process_commands(
    queue: Receiver<Command>,
    window: Window,
    database: DatabasePtr,
    config: ConfigPtr,
    scanner: ScannerPtr,
) {
    let playback_state = window.playback();

    while let Ok(command) = queue.recv().await {
        match command {
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
                on_next(&window, &playback_state);
            }
            Command::PlayPause => {
                on_play_pause(&window, &playback_state);
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
                on_previous(&window, &playback_state);
            }
            Command::Seek(pos) => {
                on_seek(&playback_state, pos);
            }
            Command::PlayFromPlaylist(uuid, pos) => {
                if let Some(playlist) = window.playlist(uuid) {
                    on_play_from_playlist(&playlist, &playback_state, pos);
                }
            }
            Command::ModifyPlaylist(uuid, subcommand) => {
                if let Some(mut playlist) = window.playlist_mut(uuid) {
                    let action = subcommand.interpret(&playlist, &database.read().unwrap());
                    action.apply(&playlist, &database.read().unwrap());
                    playlist.history_mut().push(action);
                    playlist.reset_random_data(&config.read().unwrap());
                }
            }
            Command::Undo(uuid) => {
                if let Some(mut playlist) = window.playlist_mut(uuid) {
                    if let Some(cmd) = playlist.history_mut().undo() {
                        cmd.unapply(&playlist, &database.read().unwrap());
                        playlist.reset_random_data(&config.read().unwrap());
                    }
                }
            }
            Command::Redo(uuid) => {
                if let Some(mut playlist) = window.playlist_mut(uuid) {
                    if let Some(cmd) = playlist.history_mut().redo() {
                        cmd.apply(&playlist, &database.read().unwrap());
                        playlist.reset_random_data(&config.read().unwrap());
                    }
                }
            }
            Command::RefreshMediaLibrary => refresh_library(
                window.clone(),
                database.clone(),
                config.clone(),
                scanner.clone(),
            ),
            Command::ClearMediaLibrary => {
                clear_media_library(
                    &window,
                    &mut database.write().unwrap(),
                    &mut scanner.write().unwrap(),
                    &config.read().unwrap(),
                );

                let cfg = config.read().unwrap();
                for (_, playlist) in window.playlists_mut().iter_mut() {
                    playlist.reset_random_data(&cfg);
                }
            }
            Command::ResetRandomData => {
                let cfg = config.read().unwrap();
                for (_, playlist) in window.playlists_mut().iter_mut() {
                    playlist.reset_random_data(&cfg);
                }
            }
        }
    }

    warn!("command queue exited");
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

fn on_play_pause(window: &Window, playback_state: &PlaybackState) {
    if playback_state.current().is_none() {
        if let Some(current_playlist) = window.current_playlist()
            && let Some(playlist) = window.playlist(current_playlist)
            && playlist.len() > 0
        {
            let item = playlist.get(0).unwrap();
            playback_state.set_current(Some(item));
            playback_state.set_playing(true);
        }
    } else {
        playback_state.set_playing(!playback_state.playing());
    }
}

fn on_next(window: &Window, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current()
        && let Some(mut playlist) = window.playlist_mut(current.playlist())
        && let Some(idx) = playlist.find(&current)
    {
        let next =
            playback_state
                .repeat_mode()
                .next(idx, playlist.len(), playlist.random_data_mut());
        playback_state.set_current(playlist.get(next));
        playback_state.set_playing(true);
    } else {
        playback_state.set_current(None::<PlaylistItem>);
        on_play_pause(window, playback_state);
    }
}

fn on_previous(window: &Window, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current()
        && let Some(mut playlist) = window.playlist_mut(current.playlist())
        && let Some(idx) = playlist.find(&current)
    {
        if playback_state.progress() * 10 > playback_state.duration() {
            playback_state.seek(0);
        } else {
            let next = playback_state.repeat_mode().previous(
                idx,
                playlist.len(),
                playlist.random_data_mut(),
            );
            playback_state.set_current(playlist.get(next));
            playback_state.set_playing(true);
        }
    } else {
        playback_state.set_current(None::<PlaylistItem>);
        on_play_pause(window, playback_state);
    }
}

fn refresh_playlist(playlist: &PlaylistStore, database: &Database) {
    for item in playlist.iter() {
        item.set_data(&database);
    }
}

fn get_tracks(database: &Database, item: ObjectId) -> Vec<TrackId> {
    match item {
        ObjectId::None => {
            vec![]
        }
        ObjectId::TrackId(track_id) => {
            vec![track_id]
        }
        ObjectId::AlbumId(album_id) => database.all_sorted_tracks_of_album(album_id),
        ObjectId::ArtistId(artist) => {
            let albums = database.all_sorted_albums_of_artist(artist);
            albums
                .into_iter()
                .flat_map(|a| database.all_sorted_tracks_of_album(a))
                .collect()
        }
        ObjectId::Genre(genre) => {
            let albums = database.all_sorted_albums_of_genre(genre);
            albums
                .into_iter()
                .flat_map(|a| database.all_sorted_tracks_of_album(a))
                .collect()
        }
        ObjectId::Year(year) => {
            let albums = database.all_sorted_albums_of_year(year);
            albums
                .into_iter()
                .flat_map(|a| database.all_sorted_tracks_of_album(a))
                .collect()
        }
    }
}

pub fn refresh_library(
    window: Window,
    database: DatabasePtr,
    config: ConfigPtr,
    scanner: ScannerPtr,
) {
    glib::spawn_future_local(async move {
        window.lock_refresh(true);

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

        {
            let db = database.read().unwrap();
            for (_, playlist) in window.playlists().iter() {
                refresh_playlist(playlist, &db);
            }
        }

        window.lock_refresh(false);
    });
}

pub fn clear_media_library(
    window: &Window,
    database: &mut Database,
    scanner: &mut Scanner,
    config: &Config,
) {
    for (_, playlist) in window.playlists_mut().iter_mut() {
        playlist.reset(&config);
    }

    *database = Database::default();
    *scanner = Scanner::default();
    window.repopulate_media_library();
}
