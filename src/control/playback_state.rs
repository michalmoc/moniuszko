use crate::data::playback_status::PlaybackStatus;
use gtk4::glib;
use gtk4::glib::Object;
use gtk4::prelude::MediaStreamExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

mod imp {
    use crate::data::playback_status::PlaybackStatus;
    use crate::data::repeat_mode::RepeatMode;
    use crate::ui::playlist_item::PlaylistItem;
    use adw::glib::subclass::Signal;
    use gtk4::glib::{Object, Properties, clone};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::{DerivedObjectProperties, ObjectImplExt, ObjectSubclassExt};
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use gtk4::{MediaFile, glib};
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::PlaybackState)]
    pub struct PlaybackState {
        #[property(get, set)]
        pub playing: Cell<bool>,

        #[property(get, set)]
        pub ended: Cell<bool>,

        #[property(get, set)]
        pub progress: Cell<i64>,

        #[property(get, set)]
        pub duration: Cell<i64>,

        #[property(get, set, default)]
        pub repeat_mode: Cell<RepeatMode>,

        #[property(get, set)]
        pub volume: Cell<f64>,

        #[property(get, set, default)]
        pub status: Cell<PlaybackStatus>,

        #[property(get, set=Self::change_current, nullable)]
        pub current: RefCell<Option<PlaylistItem>>,

        pub medium: RefCell<Option<MediaFile>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaybackState {
        const NAME: &'static str = "PlaybackState";
        type Type = super::PlaybackState;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaybackState {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.connect_ended_notify(|p| {
                if p.ended() {
                    p.emit_by_name::<()>("ended", &[])
                }
            });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("ended").build()])
        }
    }

    impl PlaybackState {
        fn change_current(&self, current: Option<PlaylistItem>) {
            if let Some(old) = self.current.replace(current) {
                old.set_is_playing(false);
                self.medium.take();
            }

            if let Some(current) = self.current.borrow().as_ref() {
                current.set_is_playing(true);
                self.medium
                    .replace(Some(MediaFile::for_filename(current.path())));
            }

            self.bind_medium();
            self.obj().compute_status();
        }

        fn bind_medium(&self) {
            let obj = self.obj();

            if let Some(medium) = self.medium.borrow().as_ref() {
                obj.bind_property("playing", medium, "playing")
                    .bidirectional()
                    .sync_create()
                    .build();
                medium
                    .bind_property("timestamp", obj.as_ref(), "progress")
                    .sync_create()
                    .build();
                medium
                    .bind_property("duration", obj.as_ref(), "duration")
                    .sync_create()
                    .build();
                medium
                    .bind_property("ended", obj.as_ref(), "ended")
                    .sync_create()
                    .build();
                obj.bind_property("volume", medium, "volume")
                    .bidirectional()
                    .sync_create()
                    .build();

                obj.connect_playing_notify(clone!(
                    #[weak]
                    obj,
                    move |_| obj.compute_status()
                ));
            }
        }
    }
}

glib::wrapper! {
    pub struct PlaybackState(ObjectSubclass<imp::PlaybackState>);
}

impl PlaybackState {
    pub fn new() -> Self {
        Object::builder().property("volume", 1.0).build()
    }

    pub fn seek(&self, progress: i64) {
        if let Some(medium) = self.imp().medium.borrow().as_ref() {
            medium.seek(progress);
        }
    }

    fn compute_status(&self) {
        if self.current().is_none() {
            if self.status() != PlaybackStatus::Stopped {
                self.set_status(PlaybackStatus::Stopped);
            }
        } else {
            if self.playing() {
                if self.status() != PlaybackStatus::Playing {
                    self.set_status(PlaybackStatus::Playing);
                }
            } else {
                if self.status() != PlaybackStatus::Paused {
                    self.set_status(PlaybackStatus::Paused);
                }
            }
        }
    }
}
