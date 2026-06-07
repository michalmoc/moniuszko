use crate::commands::Command;
use crate::config::{Config, ConfigPtr};
use crate::database::{DatabasePtr, ScannerPtr, SearchResultPtr};
use crate::media_library::{GroupingModePtr, MediaLibraryUi};
use crate::player::PlaybackState;
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
}

mod imp {
    use crate::commands::Command;
    use crate::config::ConfigPtr;
    use crate::database::{DatabasePtr, ObjectId, Scanner, ScannerPtr, SearchResultPtr};
    use crate::media_library::{GroupingMode, GroupingModePtr, MediaLibraryUi};
    use crate::player::{PlaybackState, PlayerUi};
    use crate::playlist::{ObjectIds, Playlist, PlaylistEntryUuids, PlaylistItem, PlaylistUi};
    use adw::subclass::prelude::{AdwApplicationWindowImpl, ObjectSubclassIsExt};
    use async_channel::Sender;
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::glib::{Object, clone, closure_local};
    use gtk4::prelude::{CastNone, EditableExt, StaticTypeExt, WidgetExt};
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
    use std::fs;
    use std::fs::File;
    use std::ops::Deref;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/window.ui")]
    pub struct Window {
        #[template_child]
        search_entry: TemplateChild<SearchEntry>,

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
            glib::spawn_future_local(clone!(
                #[weak(rename_to=this)]
                self,
                async move {
                    if let Some(bound_data) = this.bound_data.borrow().as_ref() {
                        this.refresh_button.set_sensitive(false);

                        // TODO: move somehow to commands

                        gio::spawn_blocking(clone!(
                            #[weak(rename_to=config)]
                            bound_data.config,
                            #[weak(rename_to=scanner)]
                            bound_data.scanner,
                            #[weak(rename_to=database)]
                            bound_data.database,
                            move || {
                                let config = config.read().unwrap();
                                let mut scanner = scanner.write().unwrap();
                                scanner.scan(&config.media_path, &config);
                                let db = scanner.make_database();

                                fs::create_dir_all(config.database_path().parent().unwrap())
                                    .unwrap();
                                let file = File::create(config.database_path()).unwrap();
                                serde_json::to_writer(file, scanner.deref()).unwrap();

                                *database.write().unwrap() = db;
                            }
                        ))
                        .await
                        .expect("Task needs to finish successfully.");

                        this.command(Command::RepopulateMediaLibrary);
                        this.command(Command::RefreshPlaylist);

                        this.refresh_button.set_sensitive(true);
                    }
                }
            ));
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
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}
