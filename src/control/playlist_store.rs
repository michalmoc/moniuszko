use crate::config::ConfigPtr;
use crate::data::track::TrackId;
use crate::db::database::Database;
use crate::ui::playlist_item::PlaylistItem;
use gio::prelude::{ListModelExt, ListModelExtManual};
use gtk4::prelude::{Cast, CastNone};
use std::fs;

#[derive(Clone)]
pub struct PlaylistStore {
    list: gio::ListStore,
}

impl PlaylistStore {
    pub fn wrap_and_load(store: gio::ListStore, database: &Database, config: ConfigPtr) -> Self {
        let path = config.read().unwrap().playlists_path();

        if let Ok(file) = fs::File::open(path) {
            let tracks: Vec<TrackId> = serde_json::from_reader(file).unwrap();
            for track_id in tracks {
                if database.any_has_track(track_id) {
                    store.append(&PlaylistItem::new(track_id, database));
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

    pub fn remove(&self, pos: u32) -> PlaylistItem {
        let ret = self.list.item(pos).unwrap();
        self.list.remove(pos);
        ret.downcast().unwrap()
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

    pub fn connect_changed_listener<F: Fn(u32) + 'static>(&self, f: F) {
        self.list.connect_items_changed(move |_, pos, _, _| f(pos));
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
