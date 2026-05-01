use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum Category {
    Track,
    Album,
    Artist,
    Genre,
    Year,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum GroupingMode {
    Album,
    ArtistAlbum,
    // ArtistYearAlbum,
    GenreAlbum,
    // GenreYearAlbum,
    // GenreArtistAlbum,
    YearAlbum,
}

impl GroupingMode {
    pub fn all() -> [GroupingMode; 4] {
        [
            GroupingMode::Album,
            GroupingMode::ArtistAlbum,
            GroupingMode::GenreAlbum,
            GroupingMode::YearAlbum,
            // GroupingMode::ArtistYearAlbum,
            // GroupingMode::GenreYearAlbum,
        ]
    }

    pub const fn to_str(&self) -> &'static str {
        match self {
            GroupingMode::Album => "Album",
            GroupingMode::ArtistAlbum => "Artist / Album",
            // GroupingMode::ArtistYearAlbum => "Artist / Year / Album",
            GroupingMode::GenreAlbum => "Genre / Album",
            // GroupingMode::GenreYearAlbum => "Genre / Year / Album",
            GroupingMode::YearAlbum => "Year / Album",
        }
    }

    pub fn from_str(input: &str) -> Option<GroupingMode> {
        match input {
            "Album" => Some(GroupingMode::Album),
            "Artist / Album" => Some(GroupingMode::ArtistAlbum),
            // "Artist / Year / Album" => Some(GroupingMode::ArtistYearAlbum),
            "Genre / Album" => Some(GroupingMode::GenreAlbum),
            // "Genre / Year / Album" => Some(GroupingMode::GenreYearAlbum),
            "Year / Album" => Some(GroupingMode::YearAlbum),
            _ => None,
        }
    }

    pub fn all_str() -> &'static [&'static str] {
        &[
            "Album",
            "Artist / Album",
            // "Artist / Year / Album",
            "Genre / Album",
            // "Genre / Year / Album",
            "Year / Album",
        ]
    }

    pub fn top_category(&self) -> Category {
        match self {
            GroupingMode::Album => Category::Album,
            GroupingMode::ArtistAlbum => Category::Artist,
            // GroupingMode::ArtistYearAlbum => Category::Artist,
            GroupingMode::GenreAlbum => Category::Genre,
            // GroupingMode::GenreYearAlbum => Category::Genre,
            GroupingMode::YearAlbum => Category::Year,
        }
    }

    pub fn next_category(&self, category: Category) -> Category {
        match self {
            GroupingMode::Album => Category::Track,
            GroupingMode::ArtistAlbum => match category {
                Category::Artist => Category::Album,
                _ => Category::Track,
            },
            // GroupingMode::ArtistYearAlbum => match category {
            //     Category::Artist => Category::Year,
            //     Category::Year => Category::Album,
            //     _ => Category::Track,
            // },
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
