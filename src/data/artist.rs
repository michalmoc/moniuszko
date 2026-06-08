use crate::data::album::AlbumId;
use std::collections::HashSet;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ArtistId(Uuid);

impl ArtistId {
    pub fn new() -> Self {
        ArtistId(Uuid::now_v7())
    }
}

pub struct Artist {
    pub uuid: Uuid,
    pub name: Option<Ustr>,
    pub sort: Option<Ustr>,
    pub albums: HashSet<AlbumId>,
}
