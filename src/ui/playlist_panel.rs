use crate::config::{Config, ConfigPtr};
use crate::control::commands::Command;
use crate::control::playlist_store::PlaylistStore;
use crate::data::playlist_uuid::PlaylistUuid;
use crate::db::database::Database;
use crate::ui::playlist::PlaylistUi;
use async_channel::Sender;
use gtk4::prelude::Cast;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{Widget, glib};
use std::cell::{Ref, RefMut};
use std::collections::HashMap;

glib::wrapper! {
    pub struct PlaylistPanel(ObjectSubclass<imp::PlaylistPanel>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl PlaylistPanel {
    pub fn current(&self) -> Option<PlaylistUuid> {
        self.imp().current_playlist()
    }

    pub fn request_delete_selected(&self) {
        if let Some(page) = self.imp().tab_view.selected_page() {
            page.child()
                .downcast::<PlaylistUi>()
                .unwrap()
                .request_delete_selected()
        }
    }

    pub fn rename_from_menu(&self) {
        if let Some(page) = self.imp().page_for_menu.borrow().as_ref()
            && let Some(page) = page.upgrade()
        {
            self.imp().new_playlist_name(move |_, text| {
                page.set_title(text);
            });
        }
    }

    pub fn close_from_menu(&self) {
        if let Some(page) = self.imp().page_for_menu.borrow().as_ref()
            && let Some(page) = page.upgrade()
        {
            self.imp().tab_view.close_page(&page);
        }
    }

    pub fn get(&self, uuid: PlaylistUuid) -> Option<Ref<'_, PlaylistStore>> {
        Ref::filter_map(self.imp().playlists.borrow(), |d| d.get(&uuid)).ok()
    }

    pub fn get_mut(&self, uuid: PlaylistUuid) -> Option<RefMut<'_, PlaylistStore>> {
        RefMut::filter_map(self.imp().playlists.borrow_mut(), |d| d.get_mut(&uuid)).ok()
    }

    pub fn all(&self) -> Ref<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        self.imp().playlists.borrow()
    }

    pub fn all_mut(&self) -> RefMut<'_, HashMap<PlaylistUuid, PlaylistStore>> {
        self.imp().playlists.borrow_mut()
    }

    pub fn load(&self, config: &Config, database: &Database) {
        self.imp().load(config, database);
    }

    pub fn save(&self, config: &Config) {
        self.imp().save(config)
    }

    pub fn new_playlist(&self, config: ConfigPtr) {
        self.imp().new_playlist(config)
    }

    pub fn bind_data(&self, commands: Sender<Command>) {
        self.imp().commands.replace(Some(commands));
    }
}

mod imp {
    use crate::config::{Config, ConfigPtr};
    use crate::control::commands::{Command, ModifyPlaylistCommand};
    use crate::control::playlist_store::PlaylistStore;
    use crate::data::object_id::ObjectIds;
    use crate::data::playlist_entry_uuid::PlaylistEntryUuids;
    use crate::data::playlist_uuid::PlaylistUuid;
    use crate::data::track::TrackId;
    use crate::db::database::Database;
    use crate::ui::playlist::PlaylistUi;
    use adw::glib::subclass::InitializingObject;
    use adw::glib::{Propagation, WeakRef, clone};
    use adw::prelude::{AdwDialogExt, AlertDialogExt};
    use adw::{AlertDialog, TabPage, TabView};
    use async_channel::Sender;
    use gettextrs::gettext;
    use gtk4::glib::Properties;
    use gtk4::prelude::{Cast, CastNone, EditableExt, EntryExt, ObjectExt, WidgetExt};
    use gtk4::subclass::prelude::{
        CompositeTemplateCallbacksClass, CompositeTemplateClass, DerivedObjectProperties,
        ObjectImpl, ObjectImplExt, ObjectSubclass, ObjectSubclassExt, WidgetClassExt,
    };
    use gtk4::subclass::widget::{CompositeTemplateInitializingExt, WidgetImpl};
    use gtk4::{CompositeTemplate, Entry, TemplateChild, Widget, glib, template_callbacks};
    use itertools::Itertools;
    use log::warn;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fs;
    use std::fs::File;

    #[derive(Properties, CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/playlist_panel.ui")]
    #[properties(wrapper_type = super::PlaylistPanel)]
    pub struct PlaylistPanel {
        #[template_child]
        pub tab_view: TemplateChild<TabView>,

        pub page_for_menu: RefCell<Option<WeakRef<TabPage>>>,
        pub playlists: RefCell<HashMap<PlaylistUuid, PlaylistStore>>,

        pub commands: RefCell<Option<Sender<Command>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistPanel {
        const NAME: &'static str = "PlaylistPanel";
        type Type = super::PlaylistPanel;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_layout_manager_type::<gtk4::BinLayout>();

            klass.install_action("playlist-rename", None, |panel, _, _| {
                panel.rename_from_menu();
            });

            klass.install_action("playlist-close", None, |panel, _, _| {
                panel.close_from_menu();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistPanel {
        fn constructed(&self) {
            self.parent_constructed();

            self.tab_view.connect_setup_menu(clone!(
                #[weak(rename_to=this)]
                self,
                move |_, page| {
                    this.page_for_menu.replace(page.map(|p| p.downgrade()));
                }
            ));
        }
    }

    impl WidgetImpl for PlaylistPanel {}

    impl PlaylistPanel {
        pub fn current_playlist(&self) -> Option<PlaylistUuid> {
            self.tab_view
                .selected_page()
                .map(|p| p.child().downcast::<PlaylistUi>().unwrap().uuid())
        }

        pub fn new_playlist(&self, config: ConfigPtr) {
            self.new_playlist_name(move |this, text| {
                let store = PlaylistStore::new(&config.read().unwrap());
                this.append_playlist(store, &text);
            });
        }

        pub fn new_playlist_name<F>(&self, f: F)
        where
            F: Fn(&Self, &str) + 'static,
        {
            let entry = Entry::new();

            let dialog = AlertDialog::new(Some(&gettext("new-playlist-name")), None);
            dialog.set_extra_child(Some(&entry));
            dialog.add_response("yes", &gettext("ok"));
            dialog.add_response("no", &gettext("cancel"));
            dialog.set_close_response("no");

            entry.connect_activate(clone!(
                #[weak]
                dialog,
                move |_| {
                    dialog.set_close_response("yes");
                    dialog.close();
                }
            ));

            dialog.connect_response(
                None,
                clone!(
                    #[weak(rename_to=this)]
                    self,
                    move |dialog, response| {
                        if response == "yes" {
                            let text = dialog.extra_child().and_downcast::<Entry>().unwrap().text();
                            f(&this, &text);
                        }
                    }
                ),
            );

            dialog.present(Some(&self.obj().clone()));
            entry.grab_focus();
        }

        pub fn append_playlist(&self, playlist: PlaylistStore, name: &str) {
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

            let page = self.tab_view.append(&playlist_ui);
            page.set_title(name);
            self.tab_view.set_selected_page(&page);

            self.playlists
                .borrow_mut()
                .insert(playlist.uuid(), playlist);
        }

        #[inline(always)]
        fn command(&self, command: Command) {
            if let Some(commands) = self.commands.borrow().as_ref() {
                commands.send_blocking(command).unwrap()
            }
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

        pub fn load(&self, config: &Config, database: &Database) {
            if let Ok(playlists_file) = File::open(config.playlists_path())
                && let Ok(tracklists) =
                    serde_json::from_reader::<_, Vec<(String, Vec<TrackId>)>>(playlists_file)
            {
                for (title, tracks) in tracklists {
                    let playlist = PlaylistStore::from(&tracks, database, config);
                    self.append_playlist(playlist, &title);
                }
            } else {
                let playlist = PlaylistStore::new(config);
                self.append_playlist(playlist, &gettext("playlist-default-name"));
            }
        }

        pub fn save(&self, config: &Config) {
            let path = config.playlists_path();

            let playlists = self.playlists.borrow();

            let to_save = (0..self.tab_view.n_pages())
                .map(|i| self.tab_view.nth_page(i))
                .map(|p| {
                    (
                        p.title().to_string(),
                        p.child().downcast::<PlaylistUi>().unwrap().uuid(),
                    )
                })
                .filter_map(|(title, uuid)| playlists.get(&uuid).map(|playlist| (title, playlist)))
                .collect_vec();

            let file = fs::create_dir_all(path.parent().unwrap()).and_then(|_| File::create(path));
            match file {
                Err(e) => {
                    warn!("Error creating playlist file: {}", e);
                }
                Ok(file) => {
                    serde_json::to_writer(file, &to_save).unwrap();
                }
            }
        }
    }

    #[template_callbacks]
    impl PlaylistPanel {
        #[template_callback]
        fn handle_playlist_close(&self, page: &TabPage) -> Propagation {
            let dialog = AlertDialog::new(Some(&gettext("sure-delete-playlist")), None);
            dialog.add_response("yes", &gettext("ok"));
            dialog.add_response("no", &gettext("cancel"));
            dialog.set_close_response("no");

            dialog.connect_response(
                None,
                clone!(
                    #[weak(rename_to=this)]
                    self,
                    #[weak]
                    page,
                    move |_, response| {
                        if response == "yes" {
                            let uuid = page.child().downcast::<PlaylistUi>().unwrap().uuid();
                            this.playlists.borrow_mut().remove(&uuid);
                            this.tab_view.close_page_finish(&page, true)
                        } else {
                            this.tab_view.close_page_finish(&page, false)
                        }
                    }
                ),
            );

            dialog.present(Some(&self.obj().clone()));
            Propagation::Stop
        }

        #[template_callback]
        fn handle_new_playlist(&self) {
            self.obj().activate_action("playlist-new", None).unwrap();
        }
    }
}
