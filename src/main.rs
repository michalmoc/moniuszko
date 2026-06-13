mod config;
mod constants;
pub mod control;
mod data;
mod db;
mod ui;

use crate::config::{Config, ConfigPtr};
use crate::constants::APP_ID;
use crate::control::commands::process_commands;
use crate::control::mpris::mpris;
use crate::control::tray::run_tray;
use crate::db::database::DatabasePtr;
use crate::db::scan::{Scanner, ScannerPtr};
use crate::ui::window::Window;
use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use gtk4::CssProvider;
use gtk4::gdk::Display;
use std::fs::File;
use std::sync::{Arc, RwLock};

pub fn set_global_locale_gettext() {
    let locale_dir =
        option_env!("LOCALE_DIR").unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/gettext"));

    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain("moniuszko", locale_dir).expect("Unable to bind the text domain");

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
    let (sender, receiver) = async_channel::unbounded();

    let window = Window::new(app, &config.read().unwrap());
    window.bind_data(database.clone(), config.clone(), sender.clone());
    window.present();

    glib::spawn_future_local(process_commands(
        receiver,
        window.clone(),
        database.clone(),
        config.clone(),
        scanner.clone(),
    ));
    glib::spawn_future_local(mpris(sender.clone(), window.playback(), database.clone()));

    if config.read().unwrap().tray_enabled {
        glib::spawn_future_local(run_tray(sender));
    }
}
