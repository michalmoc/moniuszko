use crate::player::PlaybackState;
use crate::playlist::PlaylistItem;
use adw::gtk;
use adw::prelude::CastNone;
use async_channel::Receiver;
use gio::prelude::ListModelExt;
use gtk4::prelude::GtkWindowExt;

pub enum Command {
    Raise,
    Quit,
    Next,
    PlayPause,
    Play(u32),
    Previous,
}

pub async fn process_commands(
    queue: Receiver<Command>,
    window: gtk::Window,
    playlist: gio::ListStore,
    playback_state: PlaybackState,
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
        }
    }
}

pub fn on_play(playlist: &gio::ListStore, playback_state: &PlaybackState, pos: u32) {
    if pos < playlist.n_items() {
        playback_state.set_current(playlist.item(pos).and_downcast::<PlaylistItem>());
        playback_state.set_playing(true);
    }
}

// TODO: wrap playlist in a struct
fn on_play_pause(playlist: &gio::ListStore, playback_state: &PlaybackState) {
    if playback_state.current().is_none() {
        if playlist.n_items() > 0 {
            let item = playlist.item(0).and_downcast::<PlaylistItem>().unwrap();
            playback_state.set_current(Some(item));
            playback_state.set_playing(true);
        }
    } else {
        playback_state.set_playing(!playback_state.playing());
    }
}

fn on_next(playlist: &gio::ListStore, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current() {
        if let Some(idx) = playlist.find(&current) {
            // playlist.n_items() != 0 because current present
            let next = playback_state.repeat_mode().next(idx, playlist.n_items());
            playback_state.set_current(playlist.item(next).and_downcast::<PlaylistItem>());
            playback_state.set_playing(true);
        } else {
            playback_state.set_current(None::<PlaylistItem>);
            on_play_pause(playlist, playback_state);
        }
    } else {
        on_play_pause(playlist, playback_state);
    }
}

fn on_previous(playlist: &gio::ListStore, playback_state: &PlaybackState) {
    if let Some(current) = playback_state.current() {
        if let Some(idx) = playlist.find(&current) {
            if playback_state.progress() * 10 > playback_state.duration() {
                playback_state.seek(0);
            } else {
                // playlist.n_items() != 0 because current present
                let next = playback_state
                    .repeat_mode()
                    .previous(idx, playlist.n_items());
                playback_state.set_current(playlist.item(next).and_downcast::<PlaylistItem>());
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
