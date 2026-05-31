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
use crate::constants::{APP_ID, APP_NAME, FANCY_APP_NAME};
use crate::database::{Database, DatabasePtr, Scanner, ScannerPtr, SearchResult, SearchResultPtr};
use crate::media_library::{GroupingMode, GroupingModePtr};
use crate::mpris::mpris;
use crate::player::PlaybackState;
use crate::playlist::Playlist;
use adw::glib::Propagation;
use adw::prelude::{
    ActionRowExt, AdwDialogExt, EntryRowExt, PreferencesDialogExt, PreferencesGroupExt,
    PreferencesPageExt, PreferencesRowExt,
};
use adw::{ButtonRow, EntryRow, PreferencesGroup, PreferencesPage, SwitchRow};
use async_channel::Sender;
use fluent_langneg::{LanguageIdentifier, NegotiationStrategy, negotiate_languages};
use fluent_zero::{set_lang, t};
use gio::{ActionEntry, Menu, SimpleActionGroup};
use gtk::prelude::*;
use gtk::{ApplicationWindow, glib};
use gtk4 as gtk;
use gtk4::gdk::Display;
use gtk4::glib::clone;
use gtk4::{
    Button, CssProvider, DropDown, Expression, HeaderBar, MenuButton, Orientation, Paned,
    SearchEntry, StringList, StringObject,
};
use image::GenericImageView;
use itertools::Itertools;
use ksni::TrayMethods;
use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, RwLock};
use sys_locale::get_locale;

include!(concat!(env!("OUT_DIR"), "/static_cache.rs"));

#[derive(Debug)]
struct MyTray {
    commands: Sender<Command>,
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        APP_NAME.into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        self.commands.send_blocking(Command::HideShow).unwrap()
    }

    fn title(&self) -> String {
        FANCY_APP_NAME.into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
            let img = image::load_from_memory_with_format(
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png")),
                image::ImageFormat::Png,
            )
            .expect("valid image");
            let (width, height) = img.dimensions();
            let mut data = img.into_rgba8().into_vec();
            assert_eq!(data.len() % 4, 0);
            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1) // rgba to argb
            }
            ksni::Icon {
                width: width as i32,
                height: height as i32,
                data,
            }
        });

        vec![ICON.clone()]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: t!("tray-hide-show").into(),
                activate: Box::new(|t: &mut MyTray| {
                    t.commands.send_blocking(Command::HideShow).unwrap()
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: t!("tray-exit").into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut MyTray| {
                    t.commands.send_blocking(Command::Quit).unwrap()
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn main() -> glib::ExitCode {
    let requested: Vec<LanguageIdentifier> = get_locale()
        .and_then(|l| l.parse().ok())
        .into_iter()
        .collect_vec();

    let available: Vec<LanguageIdentifier> =
        LOCALES.keys().filter_map(|l| l.parse().ok()).collect_vec();

    let default: LanguageIdentifier = "en-UK".parse().expect("Parsing langid failed.");

    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );

    let locale = supported[0];
    set_lang(locale.to_string().parse().unwrap());

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

    let config_button = make_menu_button();

    let titlebar = HeaderBar::new();
    titlebar.pack_start(&config_button);

    let window = ApplicationWindow::builder()
        .application(app)
        .titlebar(&titlebar)
        .title(FANCY_APP_NAME)
        .default_width(config.read().unwrap().window_width)
        .default_height(config.read().unwrap().window_height)
        .maximized(config.read().unwrap().window_maximized)
        .hide_on_close(config.read().unwrap().hide_on_close)
        .build();

    let playlist = Playlist::load_or_new(database, config.clone());
    let playlist_ui = playlist::Ui::new(database, playlist.clone(), sender.clone());
    let playlist_sw = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(120)
        .child(&playlist_ui.widget())
        .vexpand(true)
        .hexpand(true)
        .build();

    let playback_state = PlaybackState::new();
    let playback_state_clone = playback_state.clone();
    let commands_clone = sender.clone();
    playback_state.connect_ended_notify(move |_| {
        if playback_state_clone.ended() {
            commands_clone.send_blocking(Command::Next).unwrap();
        }
    });

    let player = player::new(&playback_state, sender.clone());

    let sender_clone = sender.clone();
    playlist_ui.connect_activate(move |p| {
        sender_clone
            .send_blocking(Command::PlayFromPlaylist(p))
            .unwrap()
    });

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
    let playlist_clone = playlist_ui.clone();
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

    let grouping_mode_list = StringList::new(&[]);
    for e in GroupingMode::all_str() {
        grouping_mode_list.append(&e);
    }

    let grouping_mode_choice = DropDown::new(Some(grouping_mode_list), None::<Expression>);
    grouping_mode_choice.set_selected(1);
    grouping_mode_choice.set_hexpand(true);
    let sender_clone = sender.clone();
    let grouping_mode_clone = grouping_mode.clone();
    grouping_mode_choice.connect_selected_item_notify(move |d| {
        on_grouping_mode_change(
            d.selected_item()
                .and_downcast::<StringObject>()
                .unwrap()
                .string()
                .as_str(),
            &grouping_mode_clone,
            &sender_clone,
        )
    });

    let refresh_button = Button::from_icon_name("view-refresh");
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let config_clone = config.clone();
    let sender_clone = sender.clone();
    refresh_button.connect_clicked(move |button| {
        refresh_button_cb(
            button,
            &database_clone,
            &scanner_clone,
            &config_clone,
            &sender_clone,
        )
    });

    let library_bottom_box = gtk4::Box::new(Orientation::Horizontal, 0);
    library_bottom_box.append(&grouping_mode_choice);
    library_bottom_box.append(&refresh_button);

    let sender_clone = sender.clone();
    let database_clone = database.clone();
    search.connect_search_changed(move |s| {
        on_search_changed(s, &search_result, &database_clone, &sender_clone)
    });

    let media_library_box = gtk4::Box::new(Orientation::Vertical, 0);
    media_library_box.append(&search);
    media_library_box.append(&media_library_sw);
    media_library_box.append(&library_bottom_box);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&media_library_box));
    paned.set_end_child(Some(&box_));

    let action_delete_selected = ActionEntry::builder("current-playlist-delete-selected")
        .activate(clone!(
            #[strong]
            playlist_ui,
            #[strong]
            sender,
            move |_, _, _| playlist_ui.delete_selected(&sender)
        ))
        .build();

    let action_clear_playlist = ActionEntry::builder("current-playlist-clear")
        .activate(clone!(
            #[strong]
            sender,
            move |_, _, _| {
                sender.send_blocking(Command::ClearPlaylist).unwrap();
            }
        ))
        .build();

    let action_config = ActionEntry::builder("config")
        .activate(clone!(
            #[strong]
            sender,
            #[weak]
            window,
            #[weak]
            config,
            #[weak]
            database,
            #[weak]
            scanner,
            move |_, _, _| on_config_clicked(&window, &config, &database, &scanner, &sender)
        ))
        .build();

    let action_quit = ActionEntry::builder("quit")
        .activate(clone!(
            #[strong]
            sender,
            move |_, _, _| {
                sender.send_blocking(Command::Quit).unwrap();
            }
        ))
        .build();

    app.add_action_entries([
        action_clear_playlist,
        action_delete_selected,
        action_config,
        action_quit,
    ]);
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);

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
        playlist,
        playback_state.clone(),
        database.clone(),
        media_library,
    ));

    glib::spawn_future_local(mpris(sender.clone(), playback_state, database.clone()));

    if config.read().unwrap().tray_enabled {
        glib::spawn_future_local(async {
            let tray = MyTray { commands: sender };
            let _handle = tray.spawn().await.unwrap();
            std::future::pending::<()>().await
        });
    }
}

fn make_menu_button() -> MenuButton {
    let menu_playlist = Menu::new();
    menu_playlist.append(
        Some(&t!("menu-remove-selected-from-playlist")),
        Some("app.current-playlist-delete-selected"),
    );
    menu_playlist.append(
        Some(&t!("menu-clear-playlist")),
        Some("app.current-playlist-clear"),
    );

    let menu_program = Menu::new();
    menu_program.append(Some(&t!("menu-config")), Some("app.config"));
    menu_program.append(Some(&t!("menu-quit")), Some("app.quit"));

    let menu_model = Menu::new();
    menu_model.append_section(Some(&t!("menu-playlist")), &menu_playlist);
    menu_model.append_section(Some(&t!("menu-program")), &menu_program);

    let config_button = MenuButton::new();
    config_button.set_menu_model(Some(&menu_model));
    config_button
}

fn refresh_button_cb(
    button: &Button,
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    config: &ConfigPtr,
    commands: &Sender<Command>,
) {
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let button_clone = button.clone();
    let config_clone = config.clone();
    let commands_clone = commands.clone();

    glib::spawn_future_local(async move {
        button_clone.set_sensitive(false);

        let enable_button = gio::spawn_blocking(move || {
            let config = config_clone.read().unwrap();
            let mut scanner = scanner_clone.write().unwrap();
            scanner.scan(&config.media_path, &config);
            let db = scanner.make_database();

            fs::create_dir_all(config.database_path().parent().unwrap()).unwrap();
            let file = File::create(config.database_path()).unwrap();
            serde_json::to_writer(file, scanner.deref()).unwrap();

            *database_clone.write().unwrap() = db;

            true
        })
        .await
        .expect("Task needs to finish successfully.");

        commands_clone
            .send_blocking(Command::RepopulateMediaLibrary)
            .unwrap();
        commands_clone
            .send_blocking(Command::RefreshPlaylist)
            .unwrap();

        button_clone.set_sensitive(enable_button);
    });
}

fn on_grouping_mode_change(
    selected: &str,
    grouping_mode: &GroupingModePtr,
    sender: &Sender<Command>,
) {
    grouping_mode.set(GroupingMode::from_str(selected).unwrap());
    sender
        .send_blocking(Command::RepopulateMediaLibrary)
        .unwrap();
}

fn on_search_changed(
    searcher: &SearchEntry,
    search_result: &SearchResultPtr,
    database: &DatabasePtr,
    sender: &Sender<Command>,
) {
    let result = database.read().unwrap().search(&searcher.text());
    search_result.replace(result);
    sender
        .send_blocking(Command::RepopulateMediaLibrary)
        .unwrap();
}

fn clear_library(database: &DatabasePtr, scanner: &ScannerPtr, sender: &Sender<Command>) {
    *database.write().unwrap() = Database::default();
    *scanner.write().unwrap() = Scanner::default();
    sender
        .send_blocking(Command::RepopulateMediaLibrary)
        .unwrap();
    sender.send_blocking(Command::ClearPlaylist).unwrap();
}

fn on_config_clicked(
    window: &ApplicationWindow,
    config: &ConfigPtr,
    database: &DatabasePtr,
    scanner: &ScannerPtr,
    sender: &Sender<Command>,
) {
    let cfg = config.read().unwrap();

    let media_path = EntryRow::new();
    media_path.set_title(&t!("settings-media-path"));
    media_path.set_text(&cfg.media_path.to_string_lossy());
    media_path.set_show_apply_button(true);

    let config_clone = config.clone();
    media_path.connect_apply(move |entry| {
        config_clone.write().unwrap().media_path = entry.text().into();
    });

    let full_rescan = ButtonRow::new();
    full_rescan.set_title(&t!("settings-clear-database"));
    full_rescan.set_end_icon_name(Some("view-refresh"));
    let database_clone = database.clone();
    let scanner_clone = scanner.clone();
    let sender_clone = sender.clone();
    full_rescan
        .connect_activated(move |_| clear_library(&database_clone, &scanner_clone, &sender_clone));

    let main_group = PreferencesGroup::new();
    main_group.set_title(&t!("settings-main"));
    main_group.add(&media_path);
    main_group.add(&full_rescan);

    let enable_tray = SwitchRow::new();
    enable_tray.set_title(&t!("settings-enable-tray"));
    enable_tray.set_subtitle(&t!("settings-requires-restart"));
    enable_tray.set_active(cfg.tray_enabled);

    let hide_on_close = SwitchRow::new();
    hide_on_close.set_title(&t!("settings-hide-on-close"));
    hide_on_close.set_active(cfg.hide_on_close);
    hide_on_close.set_sensitive(cfg.tray_enabled);

    enable_tray.connect_active_notify(clone!(
        #[weak]
        hide_on_close,
        #[weak]
        config,
        move |this| {
            if this.is_active() {
                hide_on_close.set_sensitive(true);
                config.write().unwrap().tray_enabled = true;
            } else {
                hide_on_close.set_sensitive(false);
                hide_on_close.set_active(false);
                config.write().unwrap().tray_enabled = false;
            }
        }
    ));

    hide_on_close.connect_active_notify(clone!(
        #[weak]
        config,
        #[weak]
        window,
        move |this| {
            let mut cfg = config.write().unwrap();
            cfg.hide_on_close = this.is_active();
            window.set_hide_on_close(this.is_active());
        }
    ));

    let tray_group = PreferencesGroup::new();
    tray_group.set_title(&t!("settings-tray"));
    tray_group.add(&enable_tray);
    tray_group.add(&hide_on_close);

    let page = PreferencesPage::new();
    page.add(&main_group);
    page.add(&tray_group);

    let dialog = adw::PreferencesDialog::new();
    dialog.add(&page);
    dialog.present(Some(window));
}
