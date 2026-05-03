use musicbrainz_rs::Fetch;
use musicbrainz_rs::entity::artist::Artist as MbArtist;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ArtistName {
    pub name: Ustr,
    pub sort: Ustr,
}

#[derive(Default, Deserialize, Serialize)]
pub struct MusicBrainz {
    name_cache: HashMap<Uuid, ArtistName>,
}

impl MusicBrainz {
    pub fn get_artist_name(&mut self, uuid: &Uuid) -> Option<ArtistName> {
        if let Some(cached) = self.name_cache.get(uuid) {
            Some(cached.clone())
        } else {
            let artist = MbArtist::fetch()
                .id(&uuid.to_string())
                .with_aliases()
                .execute()
                .ok()?;

            let name = ArtistName {
                name: artist.name.into(),
                sort: artist.sort_name.into(),
            };

            self.name_cache.insert(*uuid, name);
            Some(name)
        }
    }
}
