use crate::commands::Command;
use crate::config::{Config, ConfigPtr};
use crate::database::{DatabasePtr, ScannerPtr, SearchResultPtr};
use crate::media_library::{GroupingModePtr, MediaLibraryUi};
use crate::player::PlaybackState;
use crate::playlist::Playlist;
use adw::subclass::prelude::ObjectSubclassIsExt;
use async_channel::Sender;
use glib::Object;
use gtk4::{Button, gio, glib};

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
        scanner: ScannerPtr,
    ) {
        self.imp().bind_data(
            database,
            search_result,
            grouping_mode,
            config,
            commands,
            scanner,
        );
    }

    pub fn playlist(&self) -> Playlist {
        self.imp()
            .bound_data
            .borrow()
            .as_ref()
            .unwrap()
            .playlist
            .clone()
    }

    pub fn playback(&self) -> PlaybackState {
        self.imp().playback.get()
    }

    pub fn media_library(&self) -> MediaLibraryUi {
        self.imp().media_library.get()
    }

    pub fn refresh_button(&self) -> Button {
        self.imp().refresh_button.get()
    }
}

mod imp {
    use crate::commands::Command;
    use crate::config::ConfigPtr;
    use crate::database::{DatabasePtr, ObjectId, ScannerPtr, SearchResultPtr};
    use crate::media_library::{GroupingMode, GroupingModePtr, MediaLibraryUi};
    use crate::player::{PlaybackState, PlayerUi};
    use crate::playlist::{ObjectIds, Playlist, PlaylistEntryUuids, PlaylistItem, PlaylistUi};
    use crate::preferences::Preferences;
    use adw::prelude::AdwDialogExt;
    use adw::subclass::prelude::{AdwApplicationWindowImpl, ObjectSubclassIsExt};
    use async_channel::Sender;
    use gtk4::gdk::{Key, ModifierType};
    use gtk4::glib::Propagation;
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::prelude::{
        Cast, CastNone, EditableExt, GtkWindowExt, ObjectExt, StaticTypeExt, WidgetExt,
    };
    use gtk4::subclass::prelude::ObjectSubclassExt;
    use gtk4::subclass::prelude::{
        ApplicationWindowImpl, CompositeTemplateClass, ObjectImpl, ObjectSubclass,
    };
    use gtk4::subclass::prelude::{ObjectImplExt, WidgetClassExt};
    use gtk4::subclass::widget::{
        CompositeTemplateCallbacksClass, CompositeTemplateInitializingExt, WidgetImpl,
    };
    use gtk4::subclass::window::WindowImpl;
    use gtk4::{
        Button, CompositeTemplate, DropDown, SearchEntry, StringList, StringObject, TemplateChild,
        glib, template_callbacks,
    };
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/window.ui")]
    pub struct Window {
        #[template_child]
        pub search_entry: TemplateChild<SearchEntry>,

        #[template_child]
        pub grouping_mode: TemplateChild<DropDown>,

        #[template_child]
        pub refresh_button: TemplateChild<Button>,

        #[template_child]
        pub media_library: TemplateChild<MediaLibraryUi>,

        #[template_child]
        pub playlist: TemplateChild<PlaylistUi>,

        #[template_child]
        pub player: TemplateChild<PlayerUi>,

        #[template_child]
        pub playback: TemplateChild<PlaybackState>,

        pub bound_data: RefCell<Option<BoundData>>,
    }

    pub struct BoundData {
        pub commands: Sender<Command>,
        pub playlist: Playlist,
        pub grouping_mode: GroupingModePtr,
        pub database: DatabasePtr,
        pub config: ConfigPtr,
        pub scanner: ScannerPtr,
        pub search_result: SearchResultPtr,
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

            klass.install_action("current-playlist-delete-selected", None, |window, _, _| {
                window.imp().playlist.request_delete_selected()
            });

            klass.install_action("current-playlist-clear", None, |window, _, _| {
                window.imp().command(Command::ClearPlaylist)
            });

            klass.install_action("config", None, |window, _, _| {
                let pref = Preferences::new();
                if let Some(bound_data) = window.imp().bound_data.borrow().as_ref() {
                    pref.bind_data(
                        bound_data.config.clone(),
                        bound_data.database.clone(),
                        bound_data.scanner.clone(),
                        bound_data.commands.clone(),
                        window.clone().upcast::<gtk4::Window>().downgrade(),
                    );
                }
                pref.present(Some(window))
            });

            klass.install_action("quit", None, |window, _, _| {
                window.imp().command(Command::Quit)
            });

            klass.add_binding_action(Key::Q, ModifierType::CONTROL_MASK, "quit");
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
        pub fn bind_data(
            &self,
            database: DatabasePtr,
            search_result: SearchResultPtr,
            grouping_mode: GroupingModePtr,
            config: ConfigPtr,
            commands: Sender<Command>,
            scanner: ScannerPtr,
        ) {
            let playlist = self.playlist.playlist().unwrap();
            let playlist =
                Playlist::wrap_and_load(playlist, &database.read().unwrap(), config.clone());

            self.media_library.bind_data(
                database.clone(),
                search_result.clone(),
                grouping_mode.clone(),
            );
            self.media_library.repopulate();

            self.bound_data.replace(Some(BoundData {
                commands,
                playlist,
                grouping_mode,
                database,
                config,
                scanner,
                search_result,
            }));
        }

        #[inline(always)]
        fn command(&self, command: Command) {
            self.bound_data
                .borrow()
                .as_ref()
                .unwrap()
                .commands
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

        #[template_callback]
        fn handle_playlist_activate(&self, pos: u32) {
            self.command(Command::PlayFromPlaylist(pos))
        }

        #[template_callback]
        fn handle_library_activate(&self, obj: ObjectId) {
            self.command(Command::AppendToPlaylist(ObjectIds::single(obj)))
        }

        #[template_callback]
        fn handle_grouping_mode_change(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                let selected = self
                    .grouping_mode
                    .selected_item()
                    .and_downcast::<StringObject>()
                    .unwrap()
                    .string();
                bound_data
                    .grouping_mode
                    .set(GroupingMode::from_str(&selected).unwrap());
                self.command(Command::RepopulateMediaLibrary)
            }
        }

        #[template_callback]
        fn handle_refresh(&self) {
            self.command(Command::RefreshMediaLibrary)
        }

        #[template_callback]
        fn handle_search_changed(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                let result = bound_data
                    .database
                    .read()
                    .unwrap()
                    .search(&self.search_entry.text());
                bound_data.search_result.replace(result);
                self.command(Command::RepopulateMediaLibrary)
            }
        }

        #[template_callback]
        fn handle_close_request(&self) -> Propagation {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                let mut cfg = bound_data.config.write().unwrap();
                let obj = self.obj();

                cfg.window_width = obj.width();
                cfg.window_height = obj.height();
                cfg.window_maximized = obj.is_maximized();

                if let Err(e) = cfg.save() {
                    println!("Error saving config: {}", e);
                }
            }

            Propagation::Proceed
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}
