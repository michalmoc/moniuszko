mod config;
mod constants;
pub mod database;
mod media_library;
mod player;
mod playlist;

use crate::config::{Config, ConfigPtr};
use crate::constants::{APP_ID, APP_NAME};
use crate::database::{DatabasePtr, Scanner, ScannerPtr};
use adw::glib::Propagation;
use gtk::prelude::*;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::gdk::Display;
use gtk4::{Button, CssProvider, Orientation, Paned};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

// TODO: for 1.0
// * artists shown in playlist
// * library grouping modes
// * library search
// * volume control
// * app settings: media directory, full rescan

// TODO: for 1.1
// * translations
// * mpris
// * system tray
// * enable tray in app settings
// * right-click menu on playlist and library

// TODO: for 1.2
// * many playlists
// * save/load playlist
// * undo/redo playlist changes
// * random modes

// TODO: for 1.3
// * panel with details of current piece (including lyrics)
// * separate library for audiobooks
// * save last timestamp in audiobooks

fn main() -> glib::ExitCode {
    let config = Config::load().unwrap();

    let scanner: Scanner = if let Ok(scanner_file) = File::open(config.database_path()) {
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
    let config_clone = config_ptr.clone();
    application.connect_activate(move |a| build_ui(a, &database_ptr, &scanner_ptr, &config_clone));

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
    scanner: &ScannerPtr,
    config: &ConfigPtr,
) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(config.read().unwrap().window_width)
        .default_height(config.read().unwrap().window_height)
        .maximized(config.read().unwrap().window_maximized)
        .build();

    let playlist = playlist::Ui::new(database, config);
    let playlist_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(120)
        .child(&playlist.widget())
        .vexpand(true)
        .hexpand(true)
        .build();

    let player = player::Ui::new(playlist.store());

    let player_clone = player.clone();
    playlist.connect_activate(move |p| player_clone.play(p));

    let box_ = gtk4::Box::new(Orientation::Vertical, 0);
    box_.append(&playlist_sw);
    box_.append(&player.widget());

    let media_library = media_library::Ui::new(database);
    media_library.repopulate(database);
    media_library.widget().set_vexpand(true);
    let playlist_clone = playlist.clone();
    media_library.connect_activate(move |obj| {
        playlist_clone.append(obj);
    });

    let media_library_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(120)
        .child(&media_library.widget())
        .vexpand(true)
        .hexpand(false)
        .build();

    let refresh_button = Button::from_icon_name("view-refresh");
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let config_clone = config.clone();
    let media_library_clone = media_library.clone();
    let playlist_clone = playlist.clone();
    refresh_button.connect_clicked(move |button| {
        refresh_button_cb(
            button,
            &database_clone,
            &scanner_clone,
            &config_clone,
            &media_library_clone,
            &playlist_clone,
        )
    });

    let media_library_box = gtk4::Box::new(Orientation::Vertical, 0);
    media_library_box.append(&media_library_sw);
    media_library_box.append(&refresh_button);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&media_library_box));
    paned.set_end_child(Some(&box_));

    let config_clone = config.clone();
    window.connect_close_request(move |window| {
        let mut cfg = config_clone.write().unwrap();
        cfg.window_width = window.width();
        cfg.window_height = window.height();
        cfg.window_maximized = window.is_maximized();
        if let Err(e) = cfg.save() {
            println!("Error saving config: {}", e);
        }
        Propagation::Proceed
    });

    window.set_child(Some(&paned));

    window.present();
}

fn refresh_button_cb(
    button: &Button,
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    config: &ConfigPtr,
    media_library: &media_library::Ui,
    playlist: &playlist::Ui,
) {
    let database_clone = database.clone();
    let database_clone2 = database.clone();
    let scanner_clone = scanner.clone();
    let button_clone = button.clone();
    let config_clone = config.clone();
    let media_library_clone = media_library.clone();
    let playlist_clone = playlist.clone();

    glib::spawn_future_local(async move {
        button_clone.set_sensitive(false);

        let enable_button = gio::spawn_blocking(move || {
            let config = config_clone.read().unwrap();
            let mut scanner = scanner_clone.write().unwrap();
            scanner.scan(&config.media_path);
            let db = scanner.make_database();

            fs::create_dir_all(config.database_path().parent().unwrap()).unwrap();
            let file = File::create(config.database_path()).unwrap();
            serde_json::to_writer(file, scanner.deref()).unwrap();

            *database_clone.write().unwrap() = db;

            true
        })
        .await
        .expect("Task needs to finish successfully.");

        media_library_clone.repopulate(&database_clone2);
        playlist_clone.refresh(&database_clone2);

        button_clone.set_sensitive(enable_button);
    });
}
