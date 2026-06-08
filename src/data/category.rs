use crate::data::object_id::ObjectId;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
pub enum Category {
    Track,
    Album,
    Artist,
    Genre,
    Year,
}

impl Category {
    pub fn of(object: &ObjectId) -> Self {
        match object {
            ObjectId::None => Self::Track,
            ObjectId::TrackId(_) => Self::Track,
            ObjectId::AlbumId(_) => Self::Album,
            ObjectId::ArtistId(_) => Self::Artist,
            ObjectId::Genre(_) => Self::Genre,
            ObjectId::Year(_) => Self::Year,
        }
    }
}
