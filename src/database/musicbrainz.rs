use musicbrainz_rs::Fetch;
use musicbrainz_rs::entity::artist::Artist as MbArtist;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Default, Deserialize, Serialize)]
pub struct MusicBrainz {
    name_cache: HashMap<Uuid, Ustr>,
}

impl MusicBrainz {
    pub fn get_artist_name(&mut self, uuid: &Uuid) -> Option<Ustr> {
        if let Some(cached) = self.name_cache.get(uuid) {
            Some(cached.clone())
        } else {
            let artist = MbArtist::fetch()
                .id(&uuid.to_string())
                .with_aliases()
                .execute()
                .ok()?;

            let name = artist.name.into();

            self.name_cache.insert(*uuid, name);
            Some(name)
        }
    }
}
