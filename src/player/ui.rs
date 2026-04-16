use adw::ffi::AdwToggle;
use gio::prelude::Cast;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{Adjustment, Button, Label, Orientation, Scale, Widget};

#[derive(Clone)]
pub struct Ui {
    widget: gtk4::Box,
}

impl Ui {
    pub fn new() -> Self {
        let time_elapsed = Label::new(Some("00:00"));

        let adjustment = Adjustment::new(0.0, 0.0, 10.0, 0.1, 0.5, 1.0);
        let progress = Scale::new(Orientation::Horizontal, Some(&adjustment));
        progress.set_hexpand(true);

        let time_full = Label::new(Some("01:00"));

        let progress_box = gtk4::Box::new(Orientation::Horizontal, 5);
        progress_box.append(&time_elapsed);
        progress_box.append(&progress);
        progress_box.append(&time_full);

        let volume_button = Button::from_icon_name("multimedia-volume-control");

        let back_button = Button::from_icon_name("media-skip-backward");
        let play_button = Button::from_icon_name("media-playback-start");
        let forward_button = Button::from_icon_name("media-skip-forward");

        let player_control_box = gtk4::Box::new(Orientation::Horizontal, 0);
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
        repeat_choice.add(repeat_single);
        repeat_choice.add(repeat_all);
        repeat_choice.add(randomize);

        let control_box = gtk4::CenterBox::new();
        control_box.set_hexpand(true);
        control_box.set_start_widget(Some(&volume_button));
        control_box.set_center_widget(Some(&player_control_box));
        control_box.set_end_widget(Some(&repeat_choice));

        let main_box = gtk4::Box::new(Orientation::Vertical, 10);
        main_box.append(&progress_box);
        main_box.append(&control_box);

        Self { widget: main_box }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }
}
