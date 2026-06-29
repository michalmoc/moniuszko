use crate::config::{Config, ConfigPtr, RandomMode};
use crate::control::commands_history::CommandsHistory;
use crate::control::random_data::RandomData;
use crate::data::playlist_uuid::PlaylistUuid;
use crate::data::track::TrackId;
use crate::db::database::Database;
use crate::ui::playlist_item::PlaylistItem;
use gio::prelude::{ListModelExt, ListModelExtManual};
use gtk4::glib;
use gtk4::glib::clone;
use gtk4::prelude::{Cast, CastNone, StaticType};
use log::warn;
use std::fs;

pub struct PlaylistStore {
    uuid: PlaylistUuid,
    history: CommandsHistory,
    random_data: RandomData,
    list: gio::ListStore,
}

impl PlaylistStore {
    // TODO: saving
    pub fn new(config: &ConfigPtr) -> Self {
        let uuid = PlaylistUuid::new();

        let store = gio::ListStore::with_type(PlaylistItem::static_type());

        Self {
            uuid,
            history: Default::default(),
            random_data: Self::make_random_data(&config.read().unwrap(), store.n_items()),
            list: store,
        }
    }

    pub fn load(database: &Database, config: &ConfigPtr) -> Self {
        let uuid = PlaylistUuid::new();
        let store = gio::ListStore::with_type(PlaylistItem::static_type());

        let path = config.read().unwrap().playlists_path();

        if let Ok(file) = fs::File::open(path) {
            let tracks: Vec<TrackId> = serde_json::from_reader(file).unwrap();
            for track_id in tracks {
                if database.any_has_track(track_id) {
                    store.append(&PlaylistItem::new(uuid, track_id, database));
                }
            }
        }

        Self {
            uuid,
            history: Default::default(),
            random_data: Self::make_random_data(&config.read().unwrap(), store.n_items()),
            list: store,
        }
    }

    pub fn inner(&self) -> &gio::ListStore {
        &self.list
    }

    pub fn uuid(&self) -> PlaylistUuid {
        self.uuid
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

    fn make_random_data(config: &Config, len: u32) -> RandomData {
        match config.random_mode {
            RandomMode::TrueRandom => RandomData::pcg(len),
            RandomMode::Permutation => RandomData::sattolo(len),
        }
    }

    pub fn reset_random_data(&mut self, config: &Config) {
        self.random_data = Self::make_random_data(config, self.len());
    }

    pub fn reset(&mut self, config: &Config) {
        self.history.clear();
        self.list.remove_all();
        self.reset_random_data(config);
    }

    pub fn random_data_mut(&mut self) -> &mut RandomData {
        &mut self.random_data
    }

    pub fn history_mut(&mut self) -> &mut CommandsHistory {
        &mut self.history
    }
}

fn save_playlist(store: &gio::ListStore, config: &ConfigPtr) {
    let config = config.read().unwrap();

    fs::create_dir_all(config.playlists_path().parent().unwrap()).unwrap();
    match fs::File::create(config.playlists_path()) {
        Err(e) => {
            warn!("Error creating playlist file: {}", e);
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
