use crate::commands::Command;
use crate::config::{Config, ConfigPtr};
use crate::database::{DatabasePtr, SearchResultPtr};
use crate::media_library::{GroupingModePtr, MediaLibraryUi};
use crate::player::{PlaybackState, PlaybackStatus};
use crate::playlist::Playlist;
use adw::subclass::prelude::ObjectSubclassIsExt;
use async_channel::Sender;
use glib::Object;
use gtk4::{gio, glib};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application, config: &Config) -> Self {
        Object::builder()
            .property("application", app)
            .property("default-width", config.window_width)
            .property("default-height", config.window_height)
            .property("maximized", config.window_maximized)
            .property("hide-on-close", config.hide_on_close)
            .build()
    }

    pub fn bind_data(
        &self,
        database: DatabasePtr,
        search_result: SearchResultPtr,
        grouping_mode: GroupingModePtr,
        config: ConfigPtr,
        commands: Sender<Command>,
    ) {
        let playlist = self.imp().playlist.playlist().unwrap();
        let playlist = Playlist::wrap_and_load(playlist, &database.read().unwrap(), config);
        self.imp().playlist_store.replace(Some(playlist));

        self.imp()
            .media_library
            .bind_data(database, search_result, grouping_mode);

        self.imp().media_library.repopulate();

        self.imp().commands.replace(Some(commands));
    }

    pub fn playlist(&self) -> Playlist {
        self.imp().playlist_store.borrow().clone().unwrap()
    }

    pub fn playback(&self) -> PlaybackState {
        self.imp().playback.get()
    }

    pub fn media_library(&self) -> MediaLibraryUi {
        self.imp().media_library.get()
    }
}

mod imp {
    use crate::commands::Command;
    use crate::media_library::{GroupingMode, MediaLibraryUi};
    use crate::player::{PlaybackState, PlayerUi};
    use crate::playlist::{ObjectIds, Playlist, PlaylistEntryUuids, PlaylistItem, PlaylistUi};
    use adw::subclass::prelude::AdwApplicationWindowImpl;
    use async_channel::Sender;
    use fluent_zero::{lookup_static, t};
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::prelude::StaticTypeExt;
    use gtk4::subclass::prelude::{
        ApplicationWindowImpl, CompositeTemplateCallbacks, CompositeTemplateClass, ObjectImpl,
        ObjectSubclass,
    };
    use gtk4::subclass::prelude::{ObjectImplExt, WidgetClassExt};
    use gtk4::subclass::widget::{
        CompositeTemplateCallbacksClass, CompositeTemplateInitializingExt, WidgetImpl,
    };
    use gtk4::subclass::window::WindowImpl;
    use gtk4::{CompositeTemplate, DropDown, StringList, TemplateChild, glib, template_callbacks};
    use std::borrow::Cow;
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/window.ui")]
    pub struct Window {
        #[template_child]
        pub grouping_mode: TemplateChild<DropDown>,

        #[template_child]
        pub media_library: TemplateChild<MediaLibraryUi>,

        #[template_child]
        pub playlist: TemplateChild<PlaylistUi>,

        #[template_child]
        pub player: TemplateChild<PlayerUi>,

        #[template_child]
        pub playback: TemplateChild<PlaybackState>,

        pub commands: RefCell<Option<Sender<Command>>>,
        pub playlist_store: RefCell<Option<Playlist>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "MyGtkAppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            PlaylistItem::ensure_type();
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            // TODO change into enum
            let grouping_mode_list = StringList::new(&[]);
            for e in GroupingMode::all_str() {
                grouping_mode_list.append(&e);
            }
            self.grouping_mode.set_model(Some(&grouping_mode_list));
            self.grouping_mode.set_selected(1);
        }
    }

    #[template_callbacks]
    impl Window {
        fn command(&self, command: Command) {
            self.commands
                .borrow()
                .as_ref()
                .unwrap()
                .send_blocking(command)
                .unwrap()
        }

        #[template_callback]
        fn handle_request_append_tracks(&self, objs: ObjectIds) {
            self.command(Command::AppendToPlaylist(objs))
        }

        #[template_callback]
        fn handle_request_insert_tracks(&self, objs: ObjectIds, pos: u32) {
            self.command(Command::InsertInPlaylist(objs, pos))
        }

        #[template_callback]
        fn handle_request_remove_tracks(&self, uuids: PlaylistEntryUuids) {
            self.command(Command::RemoveFromPlaylist(uuids))
        }

        #[template_callback]
        fn handle_ended(&self) {
            self.command(Command::Next)
        }

        #[template_callback]
        fn handle_next_track(&self) {
            self.command(Command::Next)
        }

        #[template_callback]
        fn handle_play_pause(&self) {
            self.command(Command::PlayPause)
        }

        #[template_callback]
        fn handle_previous_track(&self) {
            self.command(Command::Previous)
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}
