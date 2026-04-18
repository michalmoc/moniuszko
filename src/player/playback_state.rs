use crate::database::{AlbumId, Database, ObjectId, TrackId};
use crate::playlist::PlaylistItem;
use gtk4::glib::Object;
use gtk4::prelude::{MediaStreamExt, ObjectExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{MediaFile, glib};

mod imp {
    use crate::playlist::PlaylistItem;
    use gtk4::glib::{Object, Properties};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::{DerivedObjectProperties, ObjectImplExt, ObjectSubclassExt};
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use gtk4::{MediaFile, MediaStream, glib};
    use std::cell::{Cell, RefCell};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::PlaybackState)]
    pub struct PlaybackState {
        #[property(get, set)]
        pub is_playing: RefCell<bool>,

        #[property(get, set)]
        pub progress: RefCell<i64>,

        #[property(get, set)]
        pub duration: RefCell<i64>,

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
}

glib::wrapper! {
    pub struct PlaybackState(ObjectSubclass<imp::PlaybackState>);
}

impl PlaybackState {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_current(&self, current: Option<PlaylistItem>) {
        if let Some(old) = self.imp().current.replace(current) {
            old.set_is_playing(false);
            self.imp().medium.take();
        }

        if let Some(current) = self.imp().current.borrow().as_ref() {
            current.set_is_playing(true);

            let file = gio::File::for_path(current.path());
            let media = MediaFile::for_file(&file);
            self.imp().medium.replace(Some(media));
        }

        self.bind_medium();
    }

    pub fn current(&self) -> Option<PlaylistItem> {
        self.imp().current.borrow().clone()
    }

    pub fn seek(&self, progress: i64) {
        if let Some(medium) = self.imp().medium.borrow().as_ref() {
            medium.seek(progress);
        }
    }

    fn bind_medium(&self) {
        if let Some(medium) = self.imp().medium.borrow().as_ref() {
            self.bind_property("is_playing", medium, "playing")
                .sync_create()
                .build();
            medium
                .bind_property("timestamp", self, "progress")
                .sync_create()
                .build();
            medium
                .bind_property("duration", self, "duration")
                .sync_create()
                .build();
        }
    }
}
