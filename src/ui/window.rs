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

    pub fn playlist(&self, uuid: PlaylistUuid) -> Option<Ref<'_, PlaylistStore>> {
        Ref::filter_map(self.imp().bound_data.borrow(), |d| {
            d.as_ref().and_then(|d| d.playlists.get(&uuid))
        })
        .ok()
    }

    pub fn playlist_mut(&self, uuid: PlaylistUuid) -> Option<RefMut<'_, PlaylistStore>> {
        RefMut::filter_map(self.imp().bound_data.borrow_mut(), |d| {
            d.as_mut().and_then(|d| d.playlists.get_mut(&uuid))
        })
        .ok()
    }

    pub fn playlists(&self) -> Ref<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        Ref::map(self.imp().bound_data.borrow(), |d| {
            &d.as_ref().unwrap().playlists
        })
    }

    pub fn playlists_mut(&self) -> RefMut<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        RefMut::map(self.imp().bound_data.borrow_mut(), |d| {
            &mut d.as_mut().unwrap().playlists
        })
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

    pub fn current_playlist(&self) -> Option<PlaylistUuid> {
        self.imp().current_playlist()
    }
}

mod imp {
    use crate::config::ConfigPtr;
    use crate::control::commands::{Command, ModifyPlaylistCommand};
    use crate::control::playback_state::PlaybackState;
    use crate::control::playlist_store::PlaylistStore;
    use crate::data::object_id::{ObjectId, ObjectIds};
    use crate::data::playlist_entry_uuid::PlaylistEntryUuids;
    use crate::data::playlist_uuid::PlaylistUuid;
    use crate::db::database::DatabasePtr;
    use crate::ui::info_panel::InfoPanel;
    use crate::ui::media_panel::MediaPanel;
    use crate::ui::player::PlayerUi;
    use crate::ui::playlist::PlaylistUi;
    use crate::ui::playlist_item::PlaylistItem;
    use crate::ui::preferences::Preferences;
    use adw::TabView;
    use adw::prelude::AdwDialogExt;
    use adw::subclass::prelude::{AdwApplicationWindowImpl, ObjectSubclassIsExt};
    use async_channel::Sender;
    use gtk4::gdk::{Key, ModifierType};
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::glib::{Propagation, clone};
    use gtk4::prelude::{Cast, GtkWindowExt, ObjectExt, StaticType, WidgetExt};
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
    use log::error;
    use std::cell::RefCell;
    use std::collections::HashMap;

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
        pub playlist_tab_view: TemplateChild<TabView>,

        #[template_child]
        pub player: TemplateChild<PlayerUi>,

        #[template_child]
        pub playback: TemplateChild<PlaybackState>,

        pub bound_data: RefCell<Option<BoundData>>,
    }

    pub struct BoundData {
        pub commands: Sender<Command>,
        pub playlists: HashMap<PlaylistUuid, PlaylistStore>,
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
                if let Some(page) = window.imp().playlist_tab_view.selected_page() {
                    page.child()
                        .downcast::<PlaylistUi>()
                        .unwrap()
                        .request_delete_selected()
                }
            });

            klass.install_action("current-playlist-clear", None, |window, _, _| {
                if let Some(uuid) = window.current_playlist() {
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
                if let Some(uuid) = window.current_playlist() {
                    window.imp().command(Command::Undo(uuid))
                }
            });

            klass.install_action("current-playlist-redo", None, |window, _, _| {
                if let Some(uuid) = window.current_playlist() {
                    window.imp().command(Command::Redo(uuid))
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
            let playlist = gio::ListStore::with_type(PlaylistItem::static_type()); // TODO: self.playlist.playlist().unwrap();
            let playlist =
                PlaylistStore::wrap_and_load(playlist, &database.read().unwrap(), config.clone());

            let playlist_ui = PlaylistUi::new(&playlist);
            playlist_ui.connect_request_add_tracks(clone!(
                #[weak(rename_to=this)]
                self,
                move |p, o, n| this.handle_request_add_tracks(p, o, n)
            ));
            playlist_ui.connect_request_move_tracks(clone!(
                #[weak(rename_to=this)]
                self,
                move |p, o, n| this.handle_request_move_tracks(p, o, n)
            ));
            playlist_ui.connect_request_remove_tracks(clone!(
                #[weak(rename_to=this)]
                self,
                move |p, o| this.handle_request_remove_tracks(p, o)
            ));
            playlist_ui.connect_activate(clone!(
                #[weak(rename_to=this)]
                self,
                move |p, n| this.handle_playlist_activate(p, n)
            ));

            self.playlist_tab_view.append(&playlist_ui);

            self.media_panel_music.bind_data(database.clone());
            self.media_panel_music.repopulate();
            self.media_panel_books.bind_data(database.clone());
            self.media_panel_books.repopulate();

            self.info_panel.bind_data(database.clone());

            self.bound_data.replace(Some(BoundData {
                commands,
                playlists: [(playlist.uuid(), playlist)].into(),
                database,
                config,
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
    }

    #[template_callbacks]
    impl Window {
        pub fn current_playlist(&self) -> Option<PlaylistUuid> {
            self.playlist_tab_view
                .selected_page()
                .map(|p| p.child().downcast::<PlaylistUi>().unwrap().uuid())
        }

        fn handle_request_add_tracks(&self, playlist: PlaylistUi, objs: ObjectIds, pos: u32) {
            self.command(Command::ModifyPlaylist(
                playlist.uuid(),
                ModifyPlaylistCommand::Add(objs, pos),
            ))
        }

        fn handle_request_move_tracks(
            &self,
            playlist: PlaylistUi,
            entries: PlaylistEntryUuids,
            pos: u32,
        ) {
            self.command(Command::ModifyPlaylist(
                playlist.uuid(),
                ModifyPlaylistCommand::Move(entries, pos),
            ))
        }

        fn handle_request_remove_tracks(&self, playlist: PlaylistUi, uuids: PlaylistEntryUuids) {
            self.command(Command::ModifyPlaylist(
                playlist.uuid(),
                ModifyPlaylistCommand::Remove(uuids),
            ))
        }

        fn handle_playlist_activate(&self, playlist: PlaylistUi, pos: u32) {
            self.command(Command::PlayFromPlaylist(playlist.uuid(), pos))
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
        fn handle_library_activate(&self, obj: ObjectId) {
            if let Some(uuid) = self.current_playlist() {
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

            Propagation::Proceed
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}
