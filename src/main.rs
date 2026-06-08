mod commands;
mod config;
mod constants;
pub mod database;
mod media_library;
mod mpris;
mod player;
mod playlist;
mod preferences;
mod tray;
mod window;

use crate::commands::process_commands;
use crate::config::{Config, ConfigPtr};
use crate::constants::APP_ID;
use crate::database::{DatabasePtr, Scanner, ScannerPtr, SearchResult};
use crate::media_library::GroupingMode;
use crate::mpris::mpris;
use crate::tray::run_tray;
use crate::window::Window;
use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4::CssProvider;
use gtk4::gdk::Display;
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

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
        sender.clone(),
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
        playback.clone(),
        database.clone(),
        media_library,
    ));

    glib::spawn_future_local(mpris(sender.clone(), playback, database.clone()));

    if config.read().unwrap().tray_enabled {
        glib::spawn_future_local(run_tray(sender));
    }
}
