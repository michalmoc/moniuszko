use crate::database::TrackId;
use gio::subclass::prelude::ObjectSubclassIsExt;
use gtk4::glib;
use gtk4::glib::Object;
use uuid::Uuid;

mod imp {
    use crate::database::TrackId;
    use gtk4::glib;
    use gtk4::glib::{Object, Properties};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::DerivedObjectProperties;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use std::cell::{Cell, RefCell};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::PlaylistItem)]
    pub struct PlaylistItem {
        #[property(get, set)]
        uuid: RefCell<String>,

        pub stored_track: Cell<TrackId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistItem {
        const NAME: &'static str = "PlaylistItem";
        type Type = super::PlaylistItem;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistItem {}
}

glib::wrapper! {
    pub struct PlaylistItem(ObjectSubclass<imp::PlaylistItem>);
}

impl PlaylistItem {
    pub fn new(track_id: TrackId) -> Self {
        let obj: Self = Object::builder()
            .property("uuid", Uuid::new_v4().as_simple().to_string())
            .build();

        obj.imp().stored_track.set(track_id);

        obj
    }

    pub fn get_track_id(&self) -> TrackId {
        self.imp().stored_track.get()
    }
}
