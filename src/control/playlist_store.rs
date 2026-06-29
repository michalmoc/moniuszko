use crate::config::{Config, RandomMode};
use crate::control::commands_history::CommandsHistory;
use crate::control::random_data::RandomData;
use crate::data::playlist_uuid::PlaylistUuid;
use crate::data::track::TrackId;
use crate::db::database::Database;
use crate::ui::playlist_item::PlaylistItem;
use gio::prelude::{ListModelExt, ListModelExtManual};
use gtk4::prelude::{Cast, CastNone, StaticType};
use serde::{Serialize, Serializer};

pub struct PlaylistStore {
    uuid: PlaylistUuid,
    history: CommandsHistory,
    random_data: RandomData,
    list: gio::ListStore,
}

impl PlaylistStore {
    pub fn new(config: &Config) -> Self {
        let uuid = PlaylistUuid::new();

        let store = gio::ListStore::with_type(PlaylistItem::static_type());

        Self {
            uuid,
            history: Default::default(),
            random_data: Self::make_random_data(&config, store.n_items()),
            list: store,
        }
    }

    pub fn from(tracks: &Vec<TrackId>, database: &Database, config: &Config) -> Self {
        let uuid = PlaylistUuid::new();
        let store = gio::ListStore::with_type(PlaylistItem::static_type());

        for track_id in tracks {
            if database.any_has_track(*track_id) {
                store.append(&PlaylistItem::new(uuid, *track_id, database));
            }
        }

        Self {
            uuid,
            history: Default::default(),
            random_data: Self::make_random_data(config, store.n_items()),
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

impl Serialize for PlaylistStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let playlist: Vec<TrackId> = self.iter().map(|p| p.stored_track()).collect();
        playlist.serialize(serializer)
    }
}
