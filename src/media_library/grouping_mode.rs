use crate::database::ObjectId;
use std::cell::Cell;
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
    // GenreYearAlbum,
    // GenreArtistAlbum,
    YearAlbum,
}

impl GroupingMode {
    pub fn from_str(input: &str) -> Option<GroupingMode> {
        match input {
            "None" => Some(GroupingMode::None),
            "Album" => Some(GroupingMode::Album),
            "Artist / Album" => Some(GroupingMode::ArtistAlbum),
            "Artist / Year / Album" => Some(GroupingMode::ArtistYearAlbum),
            "Genre / Album" => Some(GroupingMode::GenreAlbum),
            // "Genre / Year / Album" => Some(GroupingMode::GenreYearAlbum),
            "Year / Album" => Some(GroupingMode::YearAlbum),
            _ => None,
        }
    }

    pub fn all_str() -> &'static [&'static str] {
        &[
            "None",
            "Album",
            "Artist / Album",
            "Artist / Year / Album",
            "Genre / Album",
            // "Genre / Year / Album",
            "Year / Album",
        ]
    }

    pub fn top_category(&self) -> Category {
        match self {
            GroupingMode::None => Category::Track,
            GroupingMode::Album => Category::Album,
            GroupingMode::ArtistAlbum => Category::Artist,
            GroupingMode::ArtistYearAlbum => Category::Artist,
            GroupingMode::GenreAlbum => Category::Genre,
            // GroupingMode::GenreYearAlbum => Category::Genre,
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
            // GroupingMode::GenreYearAlbum => match category {
            //     Category::Genre => Category::Year,
            //     Category::Year => Category::Album,
            //     _ => Category::Track,
            // },
            GroupingMode::YearAlbum => match category {
                Category::Year => Category::Album,
                _ => Category::Track,
            },
        }
    }
}

pub type GroupingModePtr = Rc<Cell<GroupingMode>>;
