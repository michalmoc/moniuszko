use crate::database::DatabasePtr;
use crate::media_library;
use crate::player::PlaybackState;
use crate::playlist::{Playlist, PlaylistItem};
use adw::gtk;
use async_channel::Receiver;
use gtk4::prelude::GtkWindowExt;

pub enum Command {
    Raise,
    Quit,

    Next,
    PlayPause,
    Play(u32),
    Previous,

    RefreshPlaylist,
    ClearPlaylist,

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
                // TODO
            }
            Command::Quit => {
                window.close();
            }
            Command::Next => {
                on_next(&playlist, &playback_state);
            }
            Command::PlayPause => {
                on_play_pause(&playlist, &playback_state);
            }
            Command::Previous => {
                on_previous(&playlist, &playback_state);
            }
            Command::Play(pos) => {
                on_play(&playlist, &playback_state, pos);
            }
            Command::RefreshPlaylist => refresh_playlist(&playlist, &database),
            Command::ClearPlaylist => clear_playlist(&playlist),
            Command::RepopulateMediaLibrary => media_library.repopulate(),
        }
    }
}

pub fn on_play(playlist: &Playlist, playback_state: &PlaybackState, pos: u32) {
    if pos < playlist.len() {
        playback_state.set_current(playlist.get(pos));
        playback_state.set_playing(true);
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

fn refresh_playlist(playlist: &Playlist, database: &DatabasePtr) {
    let db = database.read().unwrap();
    for item in playlist.iter() {
        item.set_data(&db);
    }
}

fn clear_playlist(playlist: &Playlist) {
    playlist.remove_all();
}
