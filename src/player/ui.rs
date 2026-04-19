use crate::player::playback_state::PlaybackState;
use crate::playlist::PlaylistItem;
use adw::glib::Propagation;
use gio::prelude::{Cast, ListModelExt};
use gtk4::glib::Binding;
use gtk4::prelude::{BoxExt, ButtonExt, CastNone, ObjectExt, RangeExt, WidgetExt};
use gtk4::{Adjustment, Button, Label, Orientation, Scale, Widget};

#[derive(Clone)]
pub struct Ui {
    widget: gtk4::Box,
    playback_state: PlaybackState,
    store: gio::ListStore,
}

impl Ui {
    pub fn new(playlist: &gio::ListStore) -> Self {
        let playback_state = PlaybackState::new();
        let playlist_clone = playlist.clone();
        let playback_state_clone = playback_state.clone();
        playback_state.connect_ended_notify(move |_| {
            if playback_state_clone.ended() {
                on_next(&playlist_clone, &playback_state_clone)
            }
        });

        let time_elapsed = Label::new(Some("00:00"));
        time_elapsed.add_css_class("numeric");
        playback_state
            .bind_property("progress", &time_elapsed, "label")
            .transform_to(timestamp_to_text)
            .sync_create()
            .build();

        let adjustment = Adjustment::new(0.0, 0.0, 1.0, 1000000.0, 1000000.0, 1.0);
        let progress = Scale::new(Orientation::Horizontal, Some(&adjustment));
        progress.set_hexpand(true);
        playback_state
            .bind_property("progress", &progress.adjustment(), "value")
            .sync_create()
            .build();
        playback_state
            .bind_property("duration", &progress.adjustment(), "upper")
            .sync_create()
            .build();
        let playback_state_clone = playback_state.clone();
        progress.connect_change_value(move |_, _, value| {
            playback_state_clone.seek(value as i64);
            Propagation::Proceed
        });

        let time_full = Label::new(Some("00:00"));
        time_full.add_css_class("numeric");
        playback_state
            .bind_property("duration", &time_full, "label")
            .transform_to(timestamp_to_text)
            .sync_create()
            .build();

        let progress_box = gtk4::Box::new(Orientation::Horizontal, 5);
        progress_box.append(&time_elapsed);
        progress_box.append(&progress);
        progress_box.append(&time_full);

        let volume_button = Button::from_icon_name("multimedia-volume-control");
        volume_button.add_css_class("flat");
        volume_button.add_css_class("dimmed");

        let back_button = Button::from_icon_name("media-skip-backward");
        back_button.add_css_class("suggested-action");
        let playlist_clone = playlist.clone();
        let playback_state_clone = playback_state.clone();
        back_button.connect_clicked(move |_| on_previous(&playlist_clone, &playback_state_clone));

        let play_button = Button::new();
        play_button.add_css_class("suggested-action");
        let playlist_clone = playlist.clone();
        let playback_state_clone = playback_state.clone();
        play_button.connect_clicked(move |_| on_play(&playlist_clone, &playback_state_clone));
        playback_state
            .bind_property("playing", &play_button, "icon_name")
            .transform_to(|_, b: bool| {
                if b {
                    Some("media-playback-pause")
                } else {
                    Some("media-playback-start")
                }
            })
            .sync_create()
            .build();

        let forward_button = Button::from_icon_name("media-skip-forward");
        forward_button.add_css_class("suggested-action");
        let playlist_clone = playlist.clone();
        let playback_state_clone = playback_state.clone();
        forward_button.connect_clicked(move |_| on_next(&playlist_clone, &playback_state_clone));

        let player_control_box = gtk4::Box::new(Orientation::Horizontal, 0);
        player_control_box.add_css_class("linked");
        player_control_box.append(&back_button);
        player_control_box.append(&play_button);
        player_control_box.append(&forward_button);

        let repeat_single = adw::Toggle::new();
        repeat_single.set_label(Some("1"));

        let repeat_all = adw::Toggle::new();
        repeat_all.set_icon_name(Some("media-playlist-repeat"));

        let randomize = adw::Toggle::new();
        randomize.set_icon_name(Some("media-playlist-shuffle"));

        let repeat_choice = adw::ToggleGroup::new();
        repeat_choice.add_css_class("flat");
        repeat_choice.add_css_class("dimmed");
        repeat_choice.set_homogeneous(true);
        repeat_choice.add(repeat_single);
        repeat_choice.add(repeat_all);
        repeat_choice.add(randomize);
        repeat_choice.set_active(1);
        repeat_choice
            .bind_property("active", &playback_state, "repeat_mode")
            .bidirectional()
            .sync_create()
            .build();

        let control_box = gtk4::CenterBox::new();
        control_box.set_hexpand(true);
        control_box.set_start_widget(Some(&volume_button));
        control_box.set_center_widget(Some(&player_control_box));
        control_box.set_end_widget(Some(&repeat_choice));

        let main_box = gtk4::Box::new(Orientation::Vertical, 10);
        main_box.set_widget_name("player_control_main_box");
        main_box.append(&progress_box);
        main_box.append(&control_box);

        Self {
            widget: main_box,
            playback_state,
            store: playlist.clone(),
        }
    }

    pub fn play(&self, pos: u32) {
        if pos < self.store.n_items() {
            self.playback_state
                .set_current(self.store.item(pos).and_downcast());
            self.playback_state.set_playing(true);
        }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }
}

fn timestamp_to_text(_: &Binding, n: i64) -> Option<String> {
    Some(format!("{:0>2}:{:0>2}", n / 60000000, n / 1000000 % 60))
}

fn on_play(playlist: &gio::ListStore, playback_state: &PlaybackState) {
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
            playback_state.set_current(Some(playlist.item(next).and_downcast().unwrap()));
            playback_state.set_playing(true);
        } else {
            playback_state.set_current(None);
            on_play(playlist, playback_state);
        }
    } else {
        on_play(playlist, playback_state);
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
                playback_state.set_current(Some(playlist.item(next).and_downcast().unwrap()));
                playback_state.set_playing(true);
            }
        } else {
            playback_state.set_current(None);
            on_play(playlist, playback_state);
        }
    } else {
        on_play(playlist, playback_state);
    }
}
