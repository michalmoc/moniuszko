use gtk4::glib;
use gtk4::glib::Object;

mod imp {
    use crate::playlist::ui_item::PlaylistEntryUuid;
    use gtk4::glib;
    use gtk4::glib::Properties;
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::box_::BoxImpl;
    use gtk4::subclass::prelude::DerivedObjectProperties;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass, WidgetImpl};
    use std::cell::Cell;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::BoxWithPlaylistEntry)]
    pub struct BoxWithPlaylistEntry {
        #[property(get, set)]
        playlist: Cell<PlaylistEntryUuid>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BoxWithPlaylistEntry {
        const NAME: &'static str = "BoxWithPlaylistEntry";
        type Type = super::BoxWithPlaylistEntry;
        type ParentType = gtk4::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for BoxWithPlaylistEntry {}

    impl WidgetImpl for BoxWithPlaylistEntry {}

    impl BoxImpl for BoxWithPlaylistEntry {}
}

glib::wrapper! {
    pub struct BoxWithPlaylistEntry(ObjectSubclass<imp::BoxWithPlaylistEntry>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Orientable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl BoxWithPlaylistEntry {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
