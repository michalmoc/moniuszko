use crate::control::playback_state::PlaybackState;
use adw::glib::closure_local;
use adw::prelude::ObjectExt;
use gtk4::{Widget, glib};

glib::wrapper! {
    pub struct PlayerUi(ObjectSubclass<imp::PlayerUi>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl PlayerUi {
    pub fn new(playback_state: PlaybackState) -> Self {
        glib::Object::builder()
            .property("playback_state", playback_state)
            .build()
    }

    pub fn connect_previous_track<F: Fn() + 'static>(&self, f: F) {
        self.connect_closure("previous-track", false, closure_local!(move |_: Self| f()));
    }

    pub fn connect_play_pause<F: Fn() + 'static>(&self, f: F) {
        self.connect_closure("play-pause", false, closure_local!(move |_: Self| f()));
    }

    pub fn connect_next_track<F: Fn() + 'static>(&self, f: F) {
        self.connect_closure("next-track", false, closure_local!(move |_: Self| f()));
    }
}

mod imp {
    use crate::control::playback_state::PlaybackState;
    use adw::ToggleGroup;
    use adw::glib::{Binding, Propagation};
    use adw::prelude::{Cast, ObjectExt};
    use gtk4::glib::subclass::Signal;
    use gtk4::glib::{Properties, clone};
    use gtk4::prelude::{BoxExt, ButtonExt, CastNone, OrientableExt, RangeExt, WidgetExt};
    use gtk4::subclass::prelude::*;
    use gtk4::{Adjustment, Button, Label, Orientation, Scale, ScaleButton, Widget, glib};
    use log::info;
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::PlayerUi)]
    pub struct PlayerUi {
        #[property(get, construct_only)]
        playback_state: RefCell<Option<PlaybackState>>,

        progress_box: RefCell<Option<Widget>>,
        control_box: RefCell<Option<Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerUi {
        const NAME: &'static str = "PlayerUi";
        type Type = super::PlayerUi;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk4::BoxLayout>();
            klass.set_css_name("player-control");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlayerUi {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let layout_manager = obj
                .layout_manager()
                .and_downcast::<gtk4::BoxLayout>()
                .unwrap();
            layout_manager.set_spacing(10);
            layout_manager.set_orientation(Orientation::Vertical);

            let playback_state_ref = self.playback_state.borrow();
            let playback_state = playback_state_ref.as_ref().unwrap();

            let progress_box = new_progress_box(playback_state);
            progress_box.set_parent(&*obj);
            self.progress_box.replace(Some(progress_box.upcast()));

            let control_box = new_control_box(&obj, playback_state);
            control_box.set_parent(&*obj);
            self.control_box.replace(Some(control_box.upcast()));
        }

        fn dispose(&self) {
            if let Some(child) = self.progress_box.borrow_mut().take() {
                child.unparent();
            }

            if let Some(child) = self.control_box.borrow_mut().take() {
                child.unparent();
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("previous-track").build(),
                    Signal::builder("play-pause").build(),
                    Signal::builder("next-track").build(),
                ]
            })
        }
    }

    impl WidgetImpl for PlayerUi {}

    fn new_progress_box(playback_state: &PlaybackState) -> Widget {
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

        progress_box.upcast()
    }

    fn timestamp_to_text(_: &Binding, n: i64) -> Option<String> {
        Some(format!("{:0>2}:{:0>2}", n / 60000, n / 1000 % 60))
    }

    fn new_control_box(this: &super::PlayerUi, playback_state: &PlaybackState) -> Widget {
        let volume_button = ScaleButton::new(0.0, 1.0, 0.01, &["multimedia-volume-control"]);
        volume_button.add_css_class("flat");
        volume_button.add_css_class("dimmed");
        playback_state
            .bind_property("volume", &volume_button, "value")
            .bidirectional()
            .sync_create()
            .build();

        let player_control_box = new_player_control_box(this, &playback_state);

        let repeat_choice = new_repeat_choice(&playback_state);

        let control_box = gtk4::CenterBox::new();
        control_box.set_hexpand(true);
        control_box.set_start_widget(Some(&volume_button));
        control_box.set_center_widget(Some(&player_control_box));
        control_box.set_end_widget(Some(&repeat_choice));

        control_box.upcast()
    }

    fn new_repeat_choice(playback_state: &PlaybackState) -> ToggleGroup {
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
            .bind_property("active", playback_state, "repeat_mode")
            .bidirectional()
            .sync_create()
            .build();
        repeat_choice
    }

    fn new_player_control_box(this: &super::PlayerUi, playback_state: &PlaybackState) -> Widget {
        let back_button = Button::from_icon_name("media-skip-backward");
        back_button.add_css_class("suggested-action");
        back_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| this.emit_by_name::<()>("previous-track", &[])
        ));

        let play_button = Button::new();
        play_button.add_css_class("suggested-action");
        play_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| this.emit_by_name::<()>("play-pause", &[])
        ));
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
        forward_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| this.emit_by_name::<()>("next-track", &[])
        ));

        let player_control_box = gtk4::Box::new(Orientation::Horizontal, 0);
        player_control_box.add_css_class("linked");
        player_control_box.append(&back_button);
        player_control_box.append(&play_button);
        player_control_box.append(&forward_button);

        player_control_box.upcast()
    }
}
