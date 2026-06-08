use crate::commands::Command;
use crate::config::ConfigPtr;
use crate::database::{DatabasePtr, ScannerPtr};
use adw::glib;
use adw::glib::{Object, WeakRef};
use async_channel::Sender;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct Preferences(ObjectSubclass<imp::Preferences>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::ShortcutManager;
}

impl Preferences {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind_data(
        &self,
        config: ConfigPtr,
        database: DatabasePtr,
        scanner: ScannerPtr,
        commands: Sender<Command>,
        window: WeakRef<gtk4::Window>,
    ) {
        self.imp()
            .bind_data(config, database, scanner, commands, window);
    }
}

mod imp {
    use crate::commands::Command;
    use crate::config::ConfigPtr;
    use crate::database::{Database, DatabasePtr, Scanner, ScannerPtr};
    use adw::glib::subclass::InitializingObject;
    use adw::subclass::prelude::{
        AdwDialogImpl, ObjectImpl, ObjectSubclass, PreferencesDialogImpl,
    };
    use adw::{EntryRow, SwitchRow, glib};
    use async_channel::Sender;
    use gio::glib::WeakRef;
    use gtk4::prelude::{EditableExt, GtkWindowExt, WidgetExt};
    use gtk4::subclass::prelude::WidgetClassExt;
    use gtk4::subclass::prelude::WidgetImpl;
    use gtk4::subclass::widget::{
        CompositeTemplateCallbacksClass, CompositeTemplateClass, CompositeTemplateInitializingExt,
    };
    use gtk4::{CompositeTemplate, TemplateChild, template_callbacks};
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/preferences.ui")]
    pub struct Preferences {
        #[template_child]
        media_path: TemplateChild<EntryRow>,

        #[template_child]
        enable_tray: TemplateChild<SwitchRow>,

        #[template_child]
        hide_on_close: TemplateChild<SwitchRow>,

        bound_data: RefCell<Option<BoundData>>,
    }

    pub struct BoundData {
        config: ConfigPtr,
        database: DatabasePtr,
        scanner: ScannerPtr,
        commands: Sender<Command>,
        window: WeakRef<gtk4::Window>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Preferences {
        const NAME: &'static str = "Preferences";
        type Type = super::Preferences;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Preferences {}

    impl WidgetImpl for Preferences {}

    impl AdwDialogImpl for Preferences {}

    impl PreferencesDialogImpl for Preferences {}

    #[template_callbacks]
    impl Preferences {
        pub fn bind_data(
            &self,
            config: ConfigPtr,
            database: DatabasePtr,
            scanner: ScannerPtr,
            commands: Sender<Command>,
            window: WeakRef<gtk4::Window>,
        ) {
            {
                let cfg = config.read().unwrap();
                self.media_path.set_text(cfg.media_path.to_str().unwrap());

                self.enable_tray.set_active(cfg.tray_enabled);

                self.hide_on_close.set_active(cfg.hide_on_close);
                self.hide_on_close.set_sensitive(cfg.tray_enabled);
            }

            self.bound_data.replace(Some(BoundData {
                config,
                database,
                scanner,
                commands,
                window,
            }));
        }

        #[template_callback]
        fn handle_media_path(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                bound_data.config.write().unwrap().media_path = self.media_path.text().into();
            }
        }

        #[template_callback]
        fn handle_clear_database(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                *bound_data.database.write().unwrap() = Database::default();
                *bound_data.scanner.write().unwrap() = Scanner::default();
                bound_data
                    .commands
                    .send_blocking(Command::RepopulateMediaLibrary)
                    .unwrap();
                bound_data
                    .commands
                    .send_blocking(Command::ClearPlaylist)
                    .unwrap();
            }
        }

        #[template_callback]
        fn handle_enable_tray(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                if self.enable_tray.is_active() {
                    self.hide_on_close.set_sensitive(true);
                    bound_data.config.write().unwrap().tray_enabled = true;
                } else {
                    self.hide_on_close.set_sensitive(false);
                    self.hide_on_close.set_active(false);
                    bound_data.config.write().unwrap().tray_enabled = false;
                }
            }
        }

        #[template_callback]
        fn handle_hide_on_close(&self) {
            if let Some(bound_data) = self.bound_data.borrow().as_ref() {
                bound_data.config.write().unwrap().hide_on_close = self.hide_on_close.is_active();
                if let Some(window) = bound_data.window.upgrade() {
                    window.set_hide_on_close(self.hide_on_close.is_active());
                }
            }
        }
    }
}
