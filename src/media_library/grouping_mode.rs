use crate::database::ObjectId;
use fluent_zero::t;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum GroupingMode {
    None,
    Album,
    ArtistAlbum,
    ArtistYearAlbum,
    GenreAlbum,
    GenreYearAlbum,
    GenreArtistAlbum,
    YearAlbum,
}

impl GroupingMode {
    pub fn from_str(input: &str) -> Option<GroupingMode> {
        HashMap::from([
            (t!("none"), GroupingMode::None),
            (t!("album"), GroupingMode::Album),
            (t!("artist-album"), GroupingMode::ArtistAlbum),
            (t!("artist-year-album"), GroupingMode::ArtistYearAlbum),
            (t!("genre-album"), GroupingMode::GenreAlbum),
            (t!("genre-year-album"), GroupingMode::GenreYearAlbum),
            (t!("genre-artist-album"), GroupingMode::GenreArtistAlbum),
            (t!("year-album"), GroupingMode::YearAlbum),
        ])
        .get(input)
        .copied()
    }

    pub fn all_str() -> [Cow<'static, str>; 8] {
        [
            t!("none"),
            t!("album"),
            t!("artist-album"),
            t!("artist-year-album"),
            t!("genre-album"),
            t!("genre-year-album"),
            t!("genre-artist-album"),
            t!("year-album"),
        ]
    }

    pub fn top_category(&self) -> Category {
        match self {
            GroupingMode::None => Category::Track,
            GroupingMode::Album => Category::Album,
            GroupingMode::ArtistAlbum => Category::Artist,
            GroupingMode::ArtistYearAlbum => Category::Artist,
            GroupingMode::GenreAlbum => Category::Genre,
            GroupingMode::GenreYearAlbum => Category::Genre,
            GroupingMode::GenreArtistAlbum => Category::Genre,
            GroupingMode::YearAlbum => Category::Year,
        }
    }

    pub fn next_category(&self, category: Category) -> Category {
        match self {
            GroupingMode::None => Category::Track,
            GroupingMode::Album => Category::Track,
            GroupingMode::ArtistAlbum => match category {
                Category::Artist => Category::Album,
                _ => Category::Track,
            },
            GroupingMode::ArtistYearAlbum => match category {
                Category::Artist => Category::Year,
                Category::Year => Category::Album,
                _ => Category::Track,
            },
            GroupingMode::GenreAlbum => match category {
                Category::Genre => Category::Album,
                _ => Category::Track,
            },
            GroupingMode::GenreYearAlbum => match category {
                Category::Genre => Category::Year,
                Category::Year => Category::Album,
                _ => Category::Track,
            },
            GroupingMode::GenreArtistAlbum => match category {
                Category::Genre => Category::Artist,
                Category::Artist => Category::Album,
                _ => Category::Track,
            },
            GroupingMode::YearAlbum => match category {
                Category::Year => Category::Album,
                _ => Category::Track,
            },
        }
    }
}

pub type GroupingModePtr = Rc<Cell<GroupingMode>>;
