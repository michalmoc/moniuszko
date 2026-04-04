pub mod database;
mod media_library;
mod playlist;

use crate::database::{Database, DatabasePtr};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::{Orientation, Paned};
use std::sync::{Arc, RwLock};

fn main() -> glib::ExitCode {
    let mut database = Database::new();
    database.load("aaaaaaa".as_ref());
    let database = Arc::new(RwLock::new(database));

    let application = Application::builder()
        .application_id("com.example.Moniuszko")
        .build();

    application.connect_activate(move |a| build_ui(a, &database));

    application.run()
}

fn build_ui(app: &gtk::Application, database: &DatabasePtr) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Moniuszko")
        .default_width(350)
        .default_height(70)
        .build();

    let media_library = media_library::Ui::new(database);

    let playlist = playlist::Ui::new(database);
    let playlist_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(360)
        .child(&playlist.widget())
        .vexpand(true)
        .hexpand(true)
        .build();

    let box_ = gtk4::Box::new(Orientation::Vertical, 0);
    box_.append(&playlist_sw);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&media_library.widget()));
    paned.set_end_child(Some(&box_));

    window.set_child(Some(&paned));

    window.present();
}
