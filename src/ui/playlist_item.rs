use crate::data::playlist_entry_uuid::PlaylistEntryUuid;
use crate::data::track::TrackId;
use crate::db::database::Database;
use gtk4::glib;
use gtk4::glib::Object;
use uuid::Uuid;

mod imp {
    use crate::data::playlist_entry_uuid::PlaylistEntryUuid;
    use crate::data::track::TrackId;
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

        #[property(get, set, nullable)]
        position: RefCell<Option<String>>,

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
            .property("uuid", PlaylistEntryUuid::new(Uuid::new_v4()))
            .property("stored_track", track_id)
            .property("is_playing", false)
            .build();
        obj.set_data(database);
        obj
    }

    pub fn set_data(&self, database: &Database) {
        let track = database.get_track(self.stored_track()).unwrap();

        let position = match (track.max_cd, track.cd, track.position) {
            (Some(max_cd), Some(cd), Some(position)) if max_cd > 1 => {
                Some(format!("{}.{}", cd, position))
            }
            (_, _, Some(position)) => Some(format!("{}", position)),
            (_, _, _) => None,
        };

        self.set_path(track.path.clone());
        self.set_position(position);
        self.set_name(track.title.to_string());
        self.set_album(database.get_album(track.album).unwrap().title.to_string());
        self.set_artists(track.artists.map(|s| s.to_string()).unwrap_or_default());

        let duration = track.duration.as_secs();
        self.set_duration(format!("{:0>2}:{:0>2}", duration / 60, duration % 60));
    }
}
