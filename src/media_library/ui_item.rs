use crate::database::{AlbumId, ObjectId, TrackId};
use gio::subclass::prelude::ObjectSubclassIsExt;
use gtk4::glib;
use gtk4::glib::Object;

mod imp {
    use crate::database::ObjectId;
    use gtk4::glib;
    use gtk4::glib::Object;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use std::cell::Cell;

    #[derive(Default)]
    pub struct MediaListItem {
        pub stored_object: Cell<ObjectId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaListItem {
        const NAME: &'static str = "MediaListItem";
        type Type = super::MediaListItem;
        type ParentType = Object;
    }

    impl ObjectImpl for MediaListItem {}
}

glib::wrapper! {
    pub struct MediaListItem(ObjectSubclass<imp::MediaListItem>);
}

impl MediaListItem {
    pub fn new_track(track_id: TrackId) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().stored_object.set(track_id.into());
        obj
    }

    pub fn new_album(album_id: AlbumId) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().stored_object.set(album_id.into());
        obj
    }

    pub fn get_object_id(&self) -> ObjectId {
        self.imp().stored_object.get()
    }
}
