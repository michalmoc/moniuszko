use crate::database::{AlbumId, Database, ObjectId, TrackId};
use gtk4::glib;
use gtk4::glib::Object;

mod imp {
    use crate::database::ObjectId;
    use gtk4::glib::{Object, Properties};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::DerivedObjectProperties;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use gtk4::{gdk, glib};
    use std::cell::{Cell, RefCell};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::MediaListItem)]
    pub struct MediaListItem {
        #[property(get, set)]
        pub stored_object: Cell<ObjectId>,

        #[property(get, set)]
        pub name: RefCell<String>,

        #[property(get, set, nullable)]
        pub image: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaListItem {
        const NAME: &'static str = "MediaListItem";
        type Type = super::MediaListItem;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaListItem {}
}

glib::wrapper! {
    pub struct MediaListItem(ObjectSubclass<imp::MediaListItem>);
}

impl MediaListItem {
    pub fn new_track(track_id: TrackId, database: &Database) -> Self {
        Object::builder()
            .property("stored_object", ObjectId::from(track_id))
            .property("name", database[track_id].title.to_string())
            .build()
    }

    pub fn new_album(album_id: AlbumId, database: &Database) -> Self {
        Object::builder()
            .property("stored_object", ObjectId::from(album_id))
            .property("name", database[album_id].title.to_string())
            .build()
    }
}
