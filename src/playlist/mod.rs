mod box_with_playlist_entry;
mod dnd_item;
mod ui;
mod ui_item;

use crate::config::ConfigPtr;
use crate::database::{DatabasePtr, TrackId};
use crate::playlist::ui_item::PlaylistEntryUuid;
pub use dnd_item::ObjectIds;
use gio::prelude::{ListModelExt, ListModelExtManual};
use gtk4::prelude::{Cast, CastNone};
use std::fs;
pub use ui::Ui;
pub use ui_item::PlaylistItem;

#[derive(Clone)]
pub struct Playlist {
    list: gio::ListStore,
}

impl Playlist {
    pub fn load_or_new(database: &DatabasePtr, config: ConfigPtr) -> Self {
        let path = config.read().unwrap().playlists_path();
        let store = gio::ListStore::new::<PlaylistItem>();

        if let Ok(file) = fs::File::open(path) {
            let tracks: Vec<TrackId> = serde_json::from_reader(file).unwrap();
            let db = database.read().unwrap();
            for track_id in tracks {
                if db.has_track(track_id) {
                    store.append(&PlaylistItem::new(track_id, &db));
                }
            }
        }

        store.connect_items_changed(move |list, _, _, _| save_playlist(list, &config));

        Self { list: store }
    }

    pub fn inner(&self) -> &gio::ListStore {
        &self.list
    }

    pub fn append(&self, item: &PlaylistItem) {
        self.list.append(item);
    }

    pub fn insert(&self, pos: u32, item: &PlaylistItem) {
        self.list.insert(pos, item);
    }

    pub fn iter(&self) -> impl Iterator<Item = PlaylistItem> + '_ {
        self.list.iter().map(|i| i.unwrap())
    }

    pub fn remove_all(&self) {
        self.list.remove_all();
    }

    pub fn len(&self) -> u32 {
        self.list.n_items()
    }

    pub fn find_uuid(&self, uuid: PlaylistEntryUuid) -> Option<u32> {
        self.list
            .find_with_equal_func(|o| o.downcast_ref::<PlaylistItem>().unwrap().uuid() == uuid)
    }

    pub fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&PlaylistItem) -> bool,
    {
        self.list.retain(move |o| f(o.downcast_ref().unwrap()))
    }

    pub fn get(&self, pos: u32) -> Option<PlaylistItem> {
        if pos < self.len() {
            self.list.item(pos).and_downcast()
        } else {
            None
        }
    }

    pub fn find(&self, item: &PlaylistItem) -> Option<u32> {
        self.list.find(item)
    }
}

fn save_playlist(store: &gio::ListStore, config: &ConfigPtr) {
    let config = config.read().unwrap();

    fs::create_dir_all(config.playlists_path().parent().unwrap()).unwrap();
    match fs::File::create(config.playlists_path()) {
        Err(e) => {
            println!("Error creating playlist file: {}", e);
        }
        Ok(file) => {
            let playlist: Vec<TrackId> = store
                .iter::<PlaylistItem>()
                .filter_map(|i| i.ok())
                .map(|p| p.stored_track())
                .collect();

            serde_json::to_writer(file, &playlist).unwrap();
        }
    }
}
