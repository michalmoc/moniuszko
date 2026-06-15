use crate::db::database::DatabasePtr;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{Widget, glib};

glib::wrapper! {
    pub struct InfoPanel(ObjectSubclass<imp::InfoPanel>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl InfoPanel {
    pub fn bind_data(&self, database: DatabasePtr) {
        self.imp().database.replace(Some(database));
    }
}

mod imp {
    use crate::data::track::TrackId;
    use crate::db::database::DatabasePtr;
    use crate::ui::playlist_item::PlaylistItem;
    use adw::glib::subclass::InitializingObject;
    use anyhow::anyhow;
    use gtk4::gdk::Texture;
    use gtk4::glib::Properties;
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::{
        CompositeTemplateCallbacksClass, CompositeTemplateClass, DerivedObjectProperties,
        ObjectImpl, ObjectImplExt, ObjectSubclass, WidgetClassExt,
    };
    use gtk4::subclass::widget::{
        CompositeTemplateDisposeExt, CompositeTemplateInitializingExt, WidgetImpl,
    };
    use gtk4::{
        CompositeTemplate, Image, Label, Stack, TemplateChild, Widget, glib, template_callbacks,
    };
    use lofty::file::TaggedFileExt;
    use lofty::picture::PictureType;
    use lofty::probe::Probe;
    use lofty::tag::{Accessor, ItemKey};
    use std::cell::RefCell;
    use std::path::Path;

    #[derive(Properties, CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/info_panel.ui")]
    #[properties(wrapper_type = super::InfoPanel)]
    pub struct InfoPanel {
        #[property(get, set=Self::change_current, nullable)]
        pub current: RefCell<Option<PlaylistItem>>,

        #[template_child]
        pub info_stack: TemplateChild<Stack>,

        #[template_child]
        pub cover: TemplateChild<Image>,

        #[template_child]
        pub title: TemplateChild<Label>,

        #[template_child]
        pub album: TemplateChild<Label>,

        #[template_child]
        pub track_artists: TemplateChild<Label>,

        #[template_child]
        pub lyrics: TemplateChild<Label>,

        pub database: RefCell<Option<DatabasePtr>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoPanel {
        const NAME: &'static str = "InfoPanel";
        type Type = super::InfoPanel;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_layout_manager_type::<gtk4::BinLayout>();
            klass.set_css_name("info-panel");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for InfoPanel {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for InfoPanel {}

    impl InfoPanel {
        fn change_current(&self, new_current: Option<PlaylistItem>) {
            if let Some(item) = &new_current {
                self.info_stack.set_visible_child_name("main");
                self.update_data(item.stored_track());
            } else {
                self.info_stack.set_visible_child_name("placeholder");
            }

            self.current.replace(new_current);
        }

        fn update_data(&self, track_id: TrackId) {
            if let Some(database) = self.database.borrow().as_ref() {
                let database = database.read().unwrap();
                if let Some(track) = database.get_track(track_id) {
                    self.update_data_from_path(&track.path);
                }
            }
        }

        fn update_data_from_path(&self, path: &Path) {
            self.cover.set_paintable(None::<&Texture>);
            self.title.set_label("");
            self.album.set_label("");
            self.track_artists.set_label("");
            self.lyrics.set_label("");

            let Ok(probe) = Probe::open(path) else {
                return;
            };
            let Ok(tagged_file) = probe.read() else {
                return;
            };

            let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            else {
                return;
            };

            if let Some(pic) = tag
                .get_picture_type(PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
            {
                if let Ok(texture) = Texture::from_bytes(&glib::Bytes::from(pic.data())) {
                    self.cover.set_paintable(Some(&texture))
                }
            };

            if let Some(title) = tag.title() {
                self.title.set_label(&title);
            }
            if let Some(album) = tag.album() {
                self.album.set_label(&album);
            }
            if let Some(artist) = tag.artist() {
                self.track_artists.set_label(&artist);
            }
            if let Some(lyrics) = tag.get_string(ItemKey::Lyrics) {
                self.lyrics.set_label(&lyrics);
            }
        }
    }

    #[template_callbacks]
    impl InfoPanel {}
}
