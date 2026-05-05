use crate::database::{AlbumId, ArtistId, Database, ObjectId, TrackId};
use gtk4::glib;
use gtk4::glib::Object;
use ustr::Ustr;

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
        let s = database[album_id].title.to_string();
        let name = if !s.is_empty() {
            s
        } else {
            String::from("[no album]")
        };

        Object::builder()
            .property("stored_object", ObjectId::from(album_id))
            .property("name", name)
            .build()
    }

    pub fn new_artist(artist_id: ArtistId, database: &Database) -> Self {
        Object::builder()
            .property("stored_object", ObjectId::from(artist_id))
            .property(
                "name",
                database[artist_id]
                    .name
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "[unknown artist]".to_string()),
            )
            .build()
    }

    pub fn new_genre(genre: Option<Ustr>) -> Self {
        Object::builder()
            .property("stored_object", ObjectId::Genre(genre))
            .property(
                "name",
                genre
                    .map(|genre| genre.to_string())
                    .unwrap_or("[no genre]".to_string()),
            )
            .build()
    }

    pub fn new_year(year: Option<u16>) -> Self {
        Object::builder()
            .property("stored_object", ObjectId::from(year))
            .property(
                "name",
                year.map(|x| x.to_string())
                    .unwrap_or("[unknown year]".to_string()),
            )
            .build()
    }
}
