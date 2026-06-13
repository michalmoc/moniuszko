use crate::data::category::Category;
use gettextrs::gettext;
use gtk4::glib;
use std::collections::HashMap;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Default, glib::Enum)]
#[enum_type(name = "GroupingMode")]
pub enum GroupingMode {
    None,
    #[default]
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
