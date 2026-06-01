use crate::database::{Database, DatabasePtr, ObjectId, TrackId};
use crate::media_library;
use crate::player::PlaybackState;
use crate::playlist::{ObjectIds, Playlist, PlaylistEntryUuid, PlaylistEntryUuids, PlaylistItem};
use adw::gtk;
use async_channel::Receiver;
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use std::borrow::Borrow;
use std::collections::HashSet;

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
}

pub async fn process_commands(
    queue: Receiver<Command>,
    window: gtk::Window,
    playlist: Playlist,
    playback_state: PlaybackState,
    database: DatabasePtr,
    media_library: media_library::Ui,
) {
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
            Command::RepopulateMediaLibrary => media_library.repopulate(),
        }
    }
}

pub fn on_play_from_playlist(playlist: &Playlist, playback_state: &PlaybackState, pos: u32) {
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

fn on_play_pause(playlist: &Playlist, playback_state: &PlaybackState) {
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

fn on_next(playlist: &Playlist, playback_state: &PlaybackState) {
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

fn on_previous(playlist: &Playlist, playback_state: &PlaybackState) {
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

fn refresh_playlist(playlist: &Playlist, database: &Database) {
    for item in playlist.iter() {
        item.set_data(&database);
    }
}

fn clear_playlist(playlist: &Playlist) {
    playlist.remove_all();
}

fn remove_from_playlist(playlist: &Playlist, to_remove: &HashSet<PlaylistEntryUuid>) {
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

pub fn append_to_playlist(playlist: &Playlist, database: &Database, object_ids: ObjectIds) {
    for item in object_ids {
        for track in get_tracks(&database, item) {
            playlist.append(&PlaylistItem::new(track, &database));
        }
    }
}

pub fn insert_in_playlist(
    playlist: &Playlist,
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
