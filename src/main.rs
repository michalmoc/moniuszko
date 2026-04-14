mod config;
mod constants;
pub mod database;
mod media_library;
mod playlist;

use crate::config::{Config, ConfigPtr};
use crate::constants::{APP_ID, APP_NAME};
use crate::database::{DatabasePtr, Scanner, ScannerPtr};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::{Button, Orientation, Paned};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

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

    let application = Application::builder().application_id(APP_ID).build();

    application.connect_activate(move |a| build_ui(a, &database_ptr, &scanner_ptr, &config_ptr));

    application.run()
}

fn build_ui(app: &Application, database: &DatabasePtr, scanner: &ScannerPtr, config: &ConfigPtr) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(350)
        .default_height(70)
        .build();

    let media_library = media_library::Ui::new(database);
    media_library.repopulate(database);
    media_library.widget().set_vexpand(true);
    let media_library_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(360)
        .child(&media_library.widget())
        .vexpand(true)
        .hexpand(false)
        .build();

    let refresh_button = Button::from_icon_name("view-refresh");
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let config_clone = config.clone();
    let media_library_clone = media_library.clone();
    refresh_button.connect_clicked(move |button| {
        refresh_button_cb(
            button,
            &database_clone,
            &scanner_clone,
            &config_clone,
            &media_library_clone,
        )
    });

    let media_library_box = gtk4::Box::new(Orientation::Vertical, 0);
    media_library_box.append(&media_library_sw);
    media_library_box.append(&refresh_button);

    let playlist = playlist::Ui::new(database);
    let playlist_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(360)
        .child(&playlist.widget())
        .vexpand(true)
        .hexpand(true)
        .build();

    let box_ = gtk4::Box::new(Orientation::Vertical, 0);
    box_.append(&playlist_sw);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&media_library_box));
    paned.set_end_child(Some(&box_));

    window.set_child(Some(&paned));

    window.present();
}

fn refresh_button_cb(
    button: &Button,
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    config: &ConfigPtr,
    media_library: &media_library::Ui,
) {
    let database_clone = database.clone();
    let database_clone2 = database.clone();
    let scanner_clone = scanner.clone();
    let button_clone = button.clone();
    let config_clone = config.clone();
    let media_library_clone = media_library.clone();

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
        button_clone.set_sensitive(enable_button);
    });
}
