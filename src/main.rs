mod commands;
mod config;
mod constants;
pub mod database;
mod media_library;
mod mpris;
mod player;
mod playlist;
mod tray;
mod window;

use crate::commands::{Command, process_commands};
use crate::config::{Config, ConfigPtr};
use crate::constants::{APP_ID, FANCY_APP_NAME};
use crate::database::{Database, DatabasePtr, Scanner, ScannerPtr, SearchResult, SearchResultPtr};
use crate::media_library::{GroupingMode, GroupingModePtr, MediaLibraryUi};
use crate::mpris::mpris;
use crate::player::{PlaybackState, PlayerUi};
use crate::playlist::{ObjectIds, Playlist, PlaylistUi};
use crate::tray::run_tray;
use crate::window::Window;
use adw::glib::{Propagation, dgettext};
use adw::prelude::{
    ActionRowExt, AdwDialogExt, EntryRowExt, PreferencesDialogExt, PreferencesGroupExt,
    PreferencesPageExt, PreferencesRowExt,
};
use adw::{ButtonRow, EntryRow, PreferencesGroup, PreferencesPage, SwitchRow};
use async_channel::Sender;
use fluent_langneg::{LanguageIdentifier, NegotiationStrategy, negotiate_languages};
use fluent_zero::{set_lang, t};
use gettextrs::{
    LocaleCategory, bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain,
};
use gio::{ActionEntry, Menu};
use gtk::prelude::*;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::gdk::Display;
use gtk4::glib::clone;
use gtk4::{
    Button, CssProvider, DropDown, Expression, HeaderBar, MenuButton, Orientation, Paned,
    SearchEntry, StringList, StringObject,
};
use itertools::Itertools;
use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use sys_locale::get_locale;

include!(concat!(env!("OUT_DIR"), "/static_cache.rs"));

pub fn set_global_locale_gettext() {
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(
        "moniuszko",
        env!("CARGO_MANIFEST_DIR").to_owned() + "/assets/gettext",
    )
    .expect("Unable to bind the text domain");

    bind_textdomain_codeset("moniuszko", "UTF-8").expect("Unable to set text domain encoding");
    textdomain("moniuszko").expect("Unable to switch to the text domain");
}

fn main() -> glib::ExitCode {
    set_global_locale_gettext();

    gio::resources_register_include!("moniuszko.gresource").expect("Failed to register resources.");

    let config = Config::load().unwrap();

    let mut scanner: Scanner = if let Ok(scanner_file) = File::open(config.database_path()) {
        serde_json::from_reader(scanner_file).unwrap()
    } else {
        Default::default()
    };

    let database = scanner.make_database();

    let config_ptr = Arc::new(RwLock::new(config));
    let scanner_ptr = Arc::new(RwLock::new(scanner));
    let database_ptr = Arc::new(RwLock::new(database));

    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_startup(|_| load_css());
    application.connect_activate(move |a| build_ui(a, &database_ptr, &config_ptr, &scanner_ptr));

    application.run()
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(
    app: &adw::Application,
    database: &DatabasePtr,
    config: &ConfigPtr,
    scanner: &ScannerPtr,
) {
    let grouping_mode = Rc::new(Cell::new(GroupingMode::Album));
    let search_result = Rc::new(RefCell::new(SearchResult::default()));
    let (sender, receiver) = async_channel::unbounded();

    let window = Window::new(app, &config.read().unwrap());
    window.bind_data(
        database.clone(),
        search_result,
        grouping_mode,
        config.clone(),
        sender,
        scanner.clone(),
    );
    window.present();

    // TODO: present inside window, no need to pass
    let playlist = window.playlist();
    let playback = window.playback();
    let media_library = window.media_library();

    glib::spawn_future_local(process_commands(
        receiver,
        window.upcast(),
        playlist,
        playback,
        database.clone(),
        media_library,
    ));
}
//
// fn build_ui(
//     app: &adw::Application,
//     database: &DatabasePtr,
//     scanner: &ScannerPtr,
//     config: &ConfigPtr,
//     grouping_mode: &GroupingModePtr,
// ) {
//     let action_delete_selected = ActionEntry::builder("current-playlist-delete-selected")
//         .activate(clone!(
//             #[weak]
//             playlist_ui,
//             move |_, _, _| playlist_ui.request_delete_selected()
//         ))
//         .build();
//
//     let action_clear_playlist = ActionEntry::builder("current-playlist-clear")
//         .activate(clone!(
//             #[strong]
//             sender,
//             move |_, _, _| {
//                 sender.send_blocking(Command::ClearPlaylist).unwrap();
//             }
//         ))
//         .build();
//
//     let action_config = ActionEntry::builder("config")
//         .activate(clone!(
//             #[strong]
//             sender,
//             #[weak]
//             window,
//             #[weak]
//             config,
//             #[weak]
//             database,
//             #[weak]
//             scanner,
//             move |_, _, _| on_config_clicked(&window, &config, &database, &scanner, &sender)
//         ))
//         .build();
//
//     let action_quit = ActionEntry::builder("quit")
//         .activate(clone!(
//             #[strong]
//             sender,
//             move |_, _, _| {
//                 sender.send_blocking(Command::Quit).unwrap();
//             }
//         ))
//         .build();
//
//     app.add_action_entries([
//         action_clear_playlist,
//         action_delete_selected,
//         action_config,
//         action_quit,
//     ]);
//     app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
//
//     let config_clone = config.clone();
//     window.connect_close_request(move |window| {
//         let mut cfg = config_clone.write().unwrap();
//         cfg.window_width = window.width();
//         cfg.window_height = window.height();
//         cfg.window_maximized = window.is_maximized();
//         if let Err(e) = cfg.save() {
//             println!("Error saving config: {}", e);
//         }
//         Propagation::Proceed
//     });
//
//
//
//     glib::spawn_future_local(mpris(sender.clone(), playback_state, database.clone()));
//
//     if config.read().unwrap().tray_enabled {
//         glib::spawn_future_local(run_tray(sender));
//     }
// }
//
//
//

//
//
// fn clear_library(database: &DatabasePtr, scanner: &ScannerPtr, sender: &Sender<Command>) {
//     *database.write().unwrap() = Database::default();
//     *scanner.write().unwrap() = Scanner::default();
//     sender
//         .send_blocking(Command::RepopulateMediaLibrary)
//         .unwrap();
//     sender.send_blocking(Command::ClearPlaylist).unwrap();
// }
//
// fn on_config_clicked(
//     window: &ApplicationWindow,
//     config: &ConfigPtr,
//     database: &DatabasePtr,
//     scanner: &ScannerPtr,
//     sender: &Sender<Command>,
// ) {
//     let cfg = config.read().unwrap();
//
//     let media_path = EntryRow::new();
//     media_path.set_title(&t!("settings-media-path"));
//     media_path.set_text(&cfg.media_path.to_string_lossy());
//     media_path.set_show_apply_button(true);
//
//     let config_clone = config.clone();
//     media_path.connect_apply(move |entry| {
//         config_clone.write().unwrap().media_path = entry.text().into();
//     });
//
//     let full_rescan = ButtonRow::new();
//     full_rescan.set_title(&t!("settings-clear-database"));
//     full_rescan.set_end_icon_name(Some("view-refresh"));
//     let database_clone = database.clone();
//     let scanner_clone = scanner.clone();
//     let sender_clone = sender.clone();
//     full_rescan
//         .connect_activated(move |_| clear_library(&database_clone, &scanner_clone, &sender_clone));
//
//     let main_group = PreferencesGroup::new();
//     main_group.set_title(&t!("settings-main"));
//     main_group.add(&media_path);
//     main_group.add(&full_rescan);
//
//     let enable_tray = SwitchRow::new();
//     enable_tray.set_title(&t!("settings-enable-tray"));
//     enable_tray.set_subtitle(&t!("settings-requires-restart"));
//     enable_tray.set_active(cfg.tray_enabled);
//
//     let hide_on_close = SwitchRow::new();
//     hide_on_close.set_title(&t!("settings-hide-on-close"));
//     hide_on_close.set_active(cfg.hide_on_close);
//     hide_on_close.set_sensitive(cfg.tray_enabled);
//
//     enable_tray.connect_active_notify(clone!(
//         #[weak]
//         hide_on_close,
//         #[weak]
//         config,
//         move |this| {
//             if this.is_active() {
//                 hide_on_close.set_sensitive(true);
//                 config.write().unwrap().tray_enabled = true;
//             } else {
//                 hide_on_close.set_sensitive(false);
//                 hide_on_close.set_active(false);
//                 config.write().unwrap().tray_enabled = false;
//             }
//         }
//     ));
//
//     hide_on_close.connect_active_notify(clone!(
//         #[weak]
//         config,
//         #[weak]
//         window,
//         move |this| {
//             let mut cfg = config.write().unwrap();
//             cfg.hide_on_close = this.is_active();
//             window.set_hide_on_close(this.is_active());
//         }
//     ));
//
//     let tray_group = PreferencesGroup::new();
//     tray_group.set_title(&t!("settings-tray"));
//     tray_group.add(&enable_tray);
//     tray_group.add(&hide_on_close);
//
//     let page = PreferencesPage::new();
//     page.add(&main_group);
//     page.add(&tray_group);
//
//     let dialog = adw::PreferencesDialog::new();
//     dialog.add(&page);
//     dialog.present(Some(window));
// }
