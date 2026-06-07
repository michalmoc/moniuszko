use crate::database::ObjectId;
use gettextrs::gettext;
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
            (gettext("none"), GroupingMode::None),
            (gettext("album"), GroupingMode::Album),
            (gettext("artist-album"), GroupingMode::ArtistAlbum),
            (gettext("artist-year-album"), GroupingMode::ArtistYearAlbum),
            (gettext("genre-album"), GroupingMode::GenreAlbum),
            (gettext("genre-year-album"), GroupingMode::GenreYearAlbum),
            (
                gettext("genre-artist-album"),
                GroupingMode::GenreArtistAlbum,
            ),
            (gettext("year-album"), GroupingMode::YearAlbum),
        ])
        .get(input)
        .copied()
    }

    pub fn all_str() -> [String; 8] {
        [
            gettext("none"),
            gettext("album"),
            gettext("artist-album"),
            gettext("artist-year-album"),
            gettext("genre-album"),
            gettext("genre-year-album"),
            gettext("genre-artist-album"),
            gettext("year-album"),
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
