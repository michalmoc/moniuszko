use crate::database::{Database, TrackId};
use gtk4::glib;
use gtk4::glib::Object;
use uuid::Uuid;

#[derive(glib::Boxed, Copy, Clone, Eq, PartialEq, Default, Hash)]
#[boxed_type(name = "PlaylistEntryUuid")]
pub struct PlaylistEntryUuid(Uuid);

mod imp {
    use crate::database::TrackId;
    use crate::playlist::ui_item::PlaylistEntryUuid;
    use gtk4::glib;
    use gtk4::glib::{Object, Properties};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::DerivedObjectProperties;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::PlaylistItem)]
    pub struct PlaylistItem {
        #[property(get, set)]
        uuid: Cell<PlaylistEntryUuid>,

        #[property(get, set)]
        stored_track: Cell<TrackId>,

        #[property(get, set)]
        is_playing: Cell<bool>,

        #[property(get, set)]
        path: RefCell<PathBuf>,

        #[property(get, set)]
        position: Cell<u32>,

        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set)]
        album: RefCell<String>,

        #[property(get, set)]
        artists: RefCell<String>,

        #[property(get, set)]
        duration: RefCell<String>,
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
    pub fn new(track_id: TrackId, database: &Database) -> Self {
        let obj: Self = Object::builder()
            .property("uuid", PlaylistEntryUuid(Uuid::new_v4()))
            .property("stored_track", track_id)
            .property("is_playing", false)
            .build();
        obj.set_data(database);
        obj
    }

    pub fn set_data(&self, database: &Database) {
        self.set_path(database[self.stored_track()].path.clone());
        self.set_position(database[self.stored_track()].position);
        self.set_name(database[self.stored_track()].title.to_string());
        self.set_album(
            database[database[self.stored_track()].album]
                .title
                .to_string(),
        );
        self.set_artists(
            database[self.stored_track()]
                .artists
                .map(|s| s.to_string())
                .unwrap_or_default(),
        );

        let duration = database[self.stored_track()].duration.as_secs();
        self.set_duration(format!("{:0>2}:{:0>2}", duration / 60, duration % 60));
    }
}
