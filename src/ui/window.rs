use crate::config::{Config, ConfigPtr};
use crate::control::commands::Command;
use crate::control::playback_state::PlaybackState;
use crate::control::playlist_store::PlaylistStore;
use crate::data::playlist_uuid::PlaylistUuid;
use crate::db::database::DatabasePtr;
use adw::subclass::prelude::ObjectSubclassIsExt;
use async_channel::Sender;
use glib::Object;
use gtk4::prelude::WidgetExt;
use gtk4::{gio, glib};
use std::cell::{Ref, RefMut};
use std::collections::HashMap;
use std::ops::Deref;

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

    pub fn bind_data(&self, database: DatabasePtr, config: ConfigPtr, commands: Sender<Command>) {
        self.imp().bind_data(database, config, commands);
    }

    pub fn current_playlist(&self) -> Option<PlaylistUuid> {
        self.imp().playlist_panel.current()
    }

    pub fn playlist(&self, uuid: PlaylistUuid) -> Option<Ref<'_, PlaylistStore>> {
        self.imp().playlist_panel.deref().get(uuid)
    }

    pub fn playlist_mut(&self, uuid: PlaylistUuid) -> Option<RefMut<'_, PlaylistStore>> {
        self.imp().playlist_panel.deref().get_mut(uuid)
    }

    pub fn playlists(&self) -> Ref<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        self.imp().playlist_panel.all()
    }

    pub fn playlists_mut(&self) -> RefMut<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        self.imp().playlist_panel.all_mut()
    }

    pub fn playback(&self) -> PlaybackState {
        self.imp().playback.get()
    }

    pub fn lock_refresh(&self, val: bool) {
        self.imp()
            .media_panel_music
            .refresh_button()
            .set_sensitive(!val);
        self.imp()
            .media_panel_books
            .refresh_button()
            .set_sensitive(!val);
    }

    pub fn repopulate_media_library(&self) {
        self.imp().media_panel_music.repopulate();
        self.imp().media_panel_books.repopulate()
    }
}

mod imp {
    use crate::config::ConfigPtr;
    use crate::control::commands::{Command, ModifyPlaylistCommand};
    use crate::control::playback_state::PlaybackState;
    use crate::data::object_id::{ObjectId, ObjectIds};
    use crate::db::database::DatabasePtr;
    use crate::ui::info_panel::InfoPanel;
    use crate::ui::media_panel::MediaPanel;
    use crate::ui::player::PlayerUi;
    use crate::ui::playlist_panel::PlaylistPanel;
    use crate::ui::preferences::Preferences;
    use adw::glib::{ControlFlow, timeout_add_local};
    use adw::prelude::AdwDialogExt;
    use adw::subclass::prelude::{AdwApplicationWindowImpl, ObjectSubclassIsExt};
    use async_channel::Sender;
    use gtk4::gdk::{Key, ModifierType};
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::glib::{Propagation, clone};
    use gtk4::prelude::{Cast, GtkWindowExt, ObjectExt, WidgetExt};
    use gtk4::subclass::prelude::ObjectSubclassExt;
    use gtk4::subclass::prelude::WidgetClassExt;
    use gtk4::subclass::prelude::{
        ApplicationWindowImpl, CompositeTemplateClass, ObjectImpl, ObjectSubclass,
    };
    use gtk4::subclass::widget::{
        CompositeTemplateCallbacksClass, CompositeTemplateInitializingExt, WidgetImpl,
    };
    use gtk4::subclass::window::WindowImpl;
    use gtk4::{CompositeTemplate, TemplateChild, glib, template_callbacks};
    use log::{error, info};
    use std::cell::RefCell;
    use std::time::Duration;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/window.ui")]
    pub struct Window {
        #[template_child]
        pub info_panel: TemplateChild<InfoPanel>,

        #[template_child]
        pub media_panel_music: TemplateChild<MediaPanel>,

        #[template_child]
        pub media_panel_books: TemplateChild<MediaPanel>,

        #[template_child]
        pub playlist_panel: TemplateChild<PlaylistPanel>,

        #[template_child]
        pub player: TemplateChild<PlayerUi>,

        #[template_child]
        pub playback: TemplateChild<PlaybackState>,

        pub bound_data: RefCell<Option<BoundData>>,
    }

    pub struct BoundData {
        pub commands: Sender<Command>,
        pub database: DatabasePtr,
        pub config: ConfigPtr,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "MyGtkAppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("current-playlist-delete-selected", None, |window, _, _| {
                window.imp().playlist_panel.request_delete_selected()
            });

            klass.install_action("current-playlist-clear", None, |window, _, _| {
                if let Some(uuid) = window.imp().playlist_panel.current() {
                    window
                        .imp()
                        .command(Command::ModifyPlaylist(uuid, ModifyPlaylistCommand::Clear))
                }
            });

            klass.install_action("config", None, |window, _, _| {
                let pref = Preferences::new();
                if let Some(bound_data) = window.imp().bound_data.borrow().as_ref() {
                    pref.bind_data(
                        bound_data.config.clone(),
                        bound_data.commands.clone(),
                        window.clone().upcast::<gtk4::Window>().downgrade(),
                    );
                }
                pref.present(Some(window))
            });

            klass.install_action("quit", None, |window, _, _| {
                window.imp().command(Command::Quit)
            });

            klass.install_action("current-playlist-undo", None, |window, _, _| {
                if let Some(uuid) = window.imp().playlist_panel.current() {
                    window.imp().command(Command::Undo(uuid))
                }
            });

            klass.install_action("current-playlist-redo", None, |window, _, _| {
                if let Some(uuid) = window.imp().playlist_panel.current() {
                    window.imp().command(Command::Redo(uuid))
                }
            });

            klass.install_action("playlist-new", None, |window, _, _| {
                if let Some(data) = window.imp().bound_data.borrow().as_ref() {
                    window
                        .imp()
                        .playlist_panel
                        .new_playlist(data.config.clone());
                }
            });

            klass.add_binding_action(Key::Q, ModifierType::CONTROL_MASK, "quit");
            klass.add_binding_action(Key::Z, ModifierType::CONTROL_MASK, "current-playlist-undo");
            klass.add_binding_action(
                Key::Z,
                ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK,
                "current-playlist-redo",
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {}

    impl Window {
        pub fn bind_data(
            &self,
            database: DatabasePtr,
            config: ConfigPtr,
            commands: Sender<Command>,
        ) {
            self.media_panel_music.bind_data(database.clone());
            self.media_panel_music.repopulate();
            self.media_panel_books.bind_data(database.clone());
            self.media_panel_books.repopulate();

            self.info_panel.bind_data(database.clone());

            self.bound_data.replace(Some(BoundData {
                commands: commands.clone(),
                database: database.clone(),
                config: config.clone(),
            }));

            self.playlist_panel.bind_data(commands);
            self.playlist_panel
                .load(&config.read().unwrap(), &database.read().unwrap());

            timeout_add_local(
                Duration::from_mins(5),
                clone!(
                    #[weak(rename_to=this)]
                    self,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move || {
                        this.save_data();
                        ControlFlow::Continue
                    }
                ),
            );
        }

        #[inline(always)]
        fn command(&self, command: Command) {
            if let Some(data) = self.bound_data.borrow().as_ref() {
                data.commands.send_blocking(command).unwrap()
            }
        }
    }

    impl Window {
        fn save_data(&self) {
            info!("saving data");

            self.save_config();

            if let Some(data) = self.bound_data.borrow().as_ref() {
                self.playlist_panel.save(&data.config.read().unwrap());
            }
        }

        fn save_config(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                let mut cfg = bound_data.config.write().unwrap();
                let obj = self.obj();

                cfg.window_width = obj.width();
                cfg.window_height = obj.height();
                cfg.window_maximized = obj.is_maximized();

                if let Err(e) = cfg.save() {
                    error!("Error saving config: {}", e);
                }
            }
        }
    }

    #[template_callbacks]
    impl Window {
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
        fn handle_library_activate(&self, obj: ObjectId) {
            if let Some(uuid) = self.playlist_panel.current() {
                self.command(Command::ModifyPlaylist(
                    uuid,
                    ModifyPlaylistCommand::Add(ObjectIds::single(obj), u32::MAX),
                ))
            }
        }

        #[template_callback]
        fn handle_refresh(&self) {
            self.command(Command::RefreshMediaLibrary)
        }

        #[template_callback]
        fn handle_close_request(&self) -> Propagation {
            self.save_data();
            Propagation::Proceed
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}
