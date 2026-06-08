use crate::data::album::AlbumId;
use crate::data::artist::ArtistId;
use std::collections::HashSet;

#[derive(Default)]
pub struct Genre {
    pub albums: HashSet<AlbumId>,
    pub artists: HashSet<ArtistId>,
}
