use gtk4::glib;
use uuid::Uuid;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, glib::Boxed)]
#[boxed_type(name = "PlaylistUuid")]
pub struct PlaylistUuid(Uuid);

impl PlaylistUuid {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
