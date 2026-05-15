mod commands;
mod config;
mod constants;
pub mod database;
mod media_library;
mod mpris;
mod player;
mod playlist;

use crate::commands::{Command, process_commands};
use crate::config::{Config, ConfigPtr};
use crate::constants::{APP_ID, APP_NAME};
use crate::database::{Database, DatabasePtr, Scanner, ScannerPtr, SearchResult, SearchResultPtr};
use crate::media_library::{GroupingMode, GroupingModePtr};
use crate::mpris::mpris;
use crate::player::PlaybackState;
use adw::glib::Propagation;
use adw::prelude::{
    AdwDialogExt, EntryRowExt, PreferencesDialogExt, PreferencesGroupExt, PreferencesPageExt,
    PreferencesRowExt,
};
use adw::{ButtonRow, EntryRow, PreferencesGroup, PreferencesPage};
use gtk::prelude::*;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::gdk::Display;
use gtk4::{
    Button, CssProvider, DropDown, Expression, HeaderBar, Orientation, Paned, SearchEntry,
    StringList, StringObject,
};
use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

fn main() -> glib::ExitCode {
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
    let grouping_mode_ptr = Rc::new(Cell::new(GroupingMode::Album));

    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_startup(|_| load_css());
    let config_clone = config_ptr.clone();
    application.connect_activate(move |a| {
        build_ui(
            a,
            &database_ptr,
            &scanner_ptr,
            &config_clone,
            &grouping_mode_ptr,
        )
    });

    let result = application.run();

    result
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
    grouping_mode: &GroupingModePtr,
) {
    let (sender, receiver) = async_channel::unbounded();
    glib::spawn_future_local(mpris(sender.clone()));

    let config_button = Button::from_icon_name("applications-system");

    let titlebar = HeaderBar::new();
    titlebar.pack_start(&config_button);

    let window = ApplicationWindow::builder()
        .application(app)
        .titlebar(&titlebar)
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

    let playback_state = PlaybackState::new();
    let player = player::new(playback_state.clone(), sender.clone());

    let sender_clone = sender.clone();
    playlist.connect_activate(move |p| sender_clone.send_blocking(Command::Play(p)).unwrap());

    let box_ = gtk4::Box::new(Orientation::Vertical, 0);
    box_.append(&playlist_sw);
    box_.append(&player);

    let search = SearchEntry::new();
    let search_result = Rc::new(RefCell::new(SearchResult::default()));

    let media_library = media_library::Ui::new(
        database.clone(),
        grouping_mode.clone(),
        search_result.clone(),
    );
    media_library.repopulate();
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

    let grouping_mode_list = StringList::new(GroupingMode::all_str());
    let grouping_mode_choice = DropDown::new(Some(grouping_mode_list), None::<Expression>);
    grouping_mode_choice.set_selected(1);
    grouping_mode_choice.set_hexpand(true);
    let media_library_clone = media_library.clone();
    let grouping_mode_clone = grouping_mode.clone();
    grouping_mode_choice.connect_selected_item_notify(move |d| {
        on_grouping_mode_change(
            d.selected_item()
                .and_downcast::<StringObject>()
                .unwrap()
                .string()
                .as_str(),
            &grouping_mode_clone,
            &media_library_clone,
        )
    });

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

    let library_bottom_box = gtk4::Box::new(Orientation::Horizontal, 0);
    library_bottom_box.append(&grouping_mode_choice);
    library_bottom_box.append(&refresh_button);

    let media_library_clone = media_library.clone();
    let database_clone = database.clone();
    search.connect_search_changed(move |s| {
        on_search_changed(s, &search_result, &database_clone, &media_library_clone)
    });

    let media_library_box = gtk4::Box::new(Orientation::Vertical, 0);
    media_library_box.append(&search);
    media_library_box.append(&media_library_sw);
    media_library_box.append(&library_bottom_box);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&media_library_box));
    paned.set_end_child(Some(&box_));

    let window_clone = window.clone().upcast();
    let config_clone = config.clone();
    let database_clone = database.clone();
    let playlist_clone = playlist.clone();
    let media_library_clone = media_library.clone();
    let scanner_clone = scanner.clone();
    config_button.connect_clicked(move |_| {
        on_config_clicked(
            &window_clone,
            &config_clone,
            &database_clone,
            &scanner_clone,
            &media_library_clone,
            &playlist_clone,
        )
    });

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

    glib::spawn_future_local(process_commands(
        receiver,
        window.upcast(),
        playlist.store().clone(),
        playback_state,
    ));
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

        media_library_clone.repopulate();
        playlist_clone.refresh();

        button_clone.set_sensitive(enable_button);
    });
}

fn on_grouping_mode_change(
    selected: &str,
    grouping_mode: &GroupingModePtr,
    library: &media_library::Ui,
) {
    grouping_mode.set(GroupingMode::from_str(selected).unwrap());
    library.repopulate();
}

fn on_search_changed(
    searcher: &SearchEntry,
    search_result: &SearchResultPtr,
    database: &DatabasePtr,
    library: &media_library::Ui,
) {
    let result = database.read().unwrap().search(&searcher.text());
    search_result.replace(result);
    library.repopulate();
}

fn clear_library(
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    media_library: &media_library::Ui,
    playlist: &playlist::Ui,
) {
    *database.write().unwrap() = Database::default();
    *scanner.write().unwrap() = Scanner::default();
    media_library.repopulate();
    playlist.clear();
}

fn on_config_clicked(
    window: &gtk::Window,
    config: &ConfigPtr,
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    media_library: &media_library::Ui,
    playlist: &playlist::Ui,
) {
    let media_path = EntryRow::new();
    media_path.set_title("media path");
    media_path.set_text(&config.read().unwrap().media_path.to_string_lossy());
    media_path.set_show_apply_button(true);

    let config_clone = config.clone();
    media_path.connect_apply(move |entry| {
        config_clone.write().unwrap().media_path = entry.text().into();
    });

    let full_rescan = ButtonRow::new();
    full_rescan.set_title("clear database");
    full_rescan.set_end_icon_name(Some("view-refresh"));
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let media_library_clone = media_library.clone();
    let playlist_clone = playlist.clone();
    full_rescan.connect_activated(move |_| {
        clear_library(
            &database_clone,
            &scanner_clone,
            &media_library_clone,
            &playlist_clone,
        )
    });

    let group = PreferencesGroup::new();
    group.set_title("Main");
    group.add(&media_path);
    group.add(&full_rescan);

    let page = PreferencesPage::new();
    page.add(&group);

    let dialog = adw::PreferencesDialog::new();
    dialog.add(&page);
    dialog.present(Some(window));
}
