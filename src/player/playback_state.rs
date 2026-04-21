use gtk4::glib;
use gtk4::glib::Object;
use gtk4::prelude::MediaStreamExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

mod imp {
    use crate::player::repeat_mode::RepeatMode;
    use crate::playlist::PlaylistItem;
    use gtk4::glib::{Object, Properties};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::{DerivedObjectProperties, ObjectSubclassExt};
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use gtk4::{MediaFile, glib};
    use std::cell::{Cell, RefCell};

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
    impl ObjectImpl for PlaybackState {}

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
            }
        }
    }
}

glib::wrapper! {
    pub struct PlaybackState(ObjectSubclass<imp::PlaybackState>);
}

impl PlaybackState {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn seek(&self, progress: i64) {
        if let Some(medium) = self.imp().medium.borrow().as_ref() {
            medium.seek(progress);
        }
    }
}
