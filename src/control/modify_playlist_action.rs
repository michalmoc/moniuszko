use crate::control::playlist_store::PlaylistStore;
use crate::data::track::TrackId;
use crate::db::database::Database;
use crate::ui::playlist_item::PlaylistItem;
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone)]
pub enum ModifyPlaylistAction {
    Remove(BTreeMap<u32, TrackId>),
    Insert(BTreeMap<u32, TrackId>),
    Move(BTreeSet<u32>, u32),
}

impl ModifyPlaylistAction {
    pub fn apply(&self, playlist: &PlaylistStore, database: &Database) {
        match self {
            ModifyPlaylistAction::Remove(entries) => {
                Self::remove(playlist, &entries);
            }
            ModifyPlaylistAction::Insert(entries) => {
                Self::insert(playlist, database, entries);
            }
            ModifyPlaylistAction::Move(entries, pos) => {
                let mut to_add = Vec::new();
                for idx in entries.iter().rev() {
                    to_add.push(playlist.remove(*idx));
                }

                let pos = Self::calculate_pos_after_removal(entries, *pos);

                for item in to_add {
                    playlist.insert(pos, &item)
                }
            }
        }
    }

    pub fn unapply(&self, playlist: &PlaylistStore, database: &Database) {
        match self {
            ModifyPlaylistAction::Remove(entries) => {
                Self::insert(playlist, database, entries);
            }
            ModifyPlaylistAction::Insert(entries) => {
                Self::remove(playlist, &entries);
            }
            ModifyPlaylistAction::Move(entries, pos) => {
                let pos = Self::calculate_pos_after_removal(entries, *pos);

                let mut to_add = Vec::new();
                for idx in (pos..(pos + entries.len() as u32)).rev() {
                    to_add.push(playlist.remove(idx));
                }

                for (item, idx) in to_add.into_iter().rev().zip(entries) {
                    playlist.insert(*idx, &item);
                }
            }
        }
    }

    #[inline(always)]
    fn remove(playlist: &PlaylistStore, to_remove: &BTreeMap<u32, TrackId>) {
        let to_remove: HashSet<_> = to_remove
            .keys()
            .map(|i| playlist.get(*i).unwrap().uuid())
            .collect();
        playlist.retain(|item| !to_remove.contains(&item.uuid()));
    }

    #[inline(always)]
    fn insert(playlist: &PlaylistStore, database: &Database, entries: &BTreeMap<u32, TrackId>) {
        for (i, track) in entries.iter() {
            playlist.insert(*i, &PlaylistItem::new(playlist.uuid(), *track, database));
        }
    }

    #[inline(always)]
    fn calculate_pos_after_removal(entries: &BTreeSet<u32>, old_pos: u32) -> u32 {
        let offset = entries.range(0..old_pos).count();
        old_pos - offset as u32
    }
}
