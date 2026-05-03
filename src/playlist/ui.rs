use crate::config::ConfigPtr;
use crate::database::{Database, DatabasePtr, ObjectId, TrackId};
use crate::playlist::ObjectIds;
use crate::playlist::box_with_playlist_entry::BoxWithPlaylistEntry;
use crate::playlist::ui_item::PlaylistItem;
use gio::prelude::{ListModelExt, ListModelExtManual};
use gtk4::gdk::{Drag, DragAction, Key, ModifierType};
use gtk4::glib::{Object, Propagation, Value, Variant};
use gtk4::graphene::Point;
use gtk4::prelude::{
    BoxExt, Cast, CastNone, ContentProviderExtManual, DragExt, EventControllerExt, ListItemExt,
    ObjectExt, SelectionModelExt, StaticType, ToValue, WidgetExt,
};
use gtk4::{
    Align, CallbackAction, ColumnView, ColumnViewColumn, DragSource, DropTarget, KeyvalTrigger,
    Label, ListScrollFlags, MultiSelection, PickFlags, Shortcut, ShortcutController,
    SignalListItemFactory, Widget, gdk, gio,
};
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;

#[derive(Clone)]
pub struct Ui {
    store: gio::ListStore,
    widget: ColumnView,
    database: DatabasePtr,
}

impl Ui {
    pub fn new(database: &DatabasePtr, config: &ConfigPtr) -> Self {
        let store = gio::ListStore::new::<PlaylistItem>();
        let path = config.read().unwrap().playlists_path();
        if let Ok(file) = fs::File::open(path) {
            let tracks: Vec<TrackId> = serde_json::from_reader(file).unwrap();
            let db = database.read().unwrap();
            for track_id in tracks {
                if db.has_track(track_id) {
                    store.append(&PlaylistItem::new(track_id, &db));
                }
            }
        }

        let config_clone = config.clone();
        store.connect_items_changed(move |list, _, _, _| save_playlist(list, &config_clone));

        let selection = MultiSelection::new(Some(store.clone()));

        let drop_target = DropTarget::new(ObjectIds::static_type(), DragAction::all());
        let store_clone = store.clone();
        let database_clone = database.clone();
        drop_target
            .connect_drop(move |t, v, x, y| on_drop(t, v, x, y, &store_clone, &database_clone));

        let drag_source = DragSource::new();
        drag_source.set_actions(DragAction::MOVE);
        let store_clone = store.clone();
        drag_source.connect_prepare(move |s, x, y| prepare_drag(s, x, y, &store_clone));
        let store_clone = store.clone();
        drag_source.connect_drag_end(move |s, d, r| drag_end(s, d, r, &store_clone));

        let shortcut_controller = ShortcutController::new();
        let store_clone = store.clone();
        shortcut_controller.add_shortcut(
            Shortcut::builder()
                .trigger(&KeyvalTrigger::new(Key::Delete, ModifierType::empty()))
                .action(&CallbackAction::new(move |w, a| {
                    on_delete(w, a, &store_clone)
                }))
                .build(),
        );

        let view = ColumnView::new(Some(selection));
        view.add_controller(drag_source);
        view.add_controller(drop_target);
        view.add_controller(shortcut_controller);

        view.append_column(&Column::new_numeric("#", "position").build());
        view.append_column(&Column::new_text("title", "name").build());
        view.append_column(&Column::new_text("artists", "artists").build());
        view.append_column(&Column::new_text("album", "album").build());
        view.append_column(&Column::new_numeric("duration", "duration").build());

        Self {
            store,
            widget: view,
            database: database.clone(),
        }
    }

    pub fn refresh(&self) {
        let db = self.database.read().unwrap();
        for i in 0..self.store.n_items() {
            let item = self.store.item(i).and_downcast::<PlaylistItem>().unwrap();
            item.set_data(&db);
        }
    }

    pub fn connect_activate<F: Fn(u32) + 'static>(&self, f: F) {
        self.widget.connect_activate(move |_, p| f(p));
    }

    pub fn append(&self, object_id: ObjectId) {
        let db = self.database.read().unwrap();

        let tracks = get_tracks(&db, object_id);
        self.widget.model().unwrap().unselect_all();

        for track in tracks {
            self.store.append(&PlaylistItem::new(track, &db));
            self.widget
                .model()
                .unwrap()
                .select_item(self.store.n_items() - 1, false);
        }
        if self.store.n_items() > 0 {
            self.widget
                .scroll_to(self.store.n_items() - 1, None, ListScrollFlags::FOCUS, None);
        }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }

    pub fn store(&self) -> &gio::ListStore {
        &self.store
    }
}

struct Column {
    title: String,
    property: String,

    align: Align,
    class: Option<String>,
}

impl Column {
    fn new_text(title: &str, property: &str) -> Column {
        Self {
            title: title.to_string(),
            property: property.to_string(),
            align: Align::Start,
            class: None,
        }
    }

    fn new_numeric(title: &str, property: &str) -> Column {
        Self {
            title: title.to_string(),
            property: property.to_string(),
            align: Align::End,
            class: Some("numeric".to_string()),
        }
    }

    fn build(self) -> ColumnViewColumn {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, item| Self::setup(item, self.align, &self.class));
        factory.connect_bind(move |_, item| Self::bind(item, &self.property));

        let column = ColumnViewColumn::new(Some(&self.title), Some(factory));
        column.set_resizable(true);

        column
    }

    fn setup(list_item: &Object, align: Align, class: &Option<String>) {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

        let label = Label::new(None);
        label.set_halign(align);
        label.set_hexpand(true);

        let box_ = BoxWithPlaylistEntry::new();
        box_.append(&label);

        if let Some(class) = &class {
            box_.add_css_class(class);
        }

        list_item.set_child(Some(&box_));
    }

    fn bind(list_item: &Object, property: &str) {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

        let box_ = list_item
            .child()
            .and_downcast::<BoxWithPlaylistEntry>()
            .unwrap();
        let label = box_.first_child().and_downcast::<Label>().unwrap();

        let dataobj = list_item.item().and_downcast::<PlaylistItem>().unwrap();

        dataobj
            .bind_property("uuid", &box_, "playlist")
            .sync_create()
            .build();
        dataobj
            .bind_property(property, &label, "label")
            .sync_create()
            .build();
        dataobj
            .bind_property("is_playing", &label, "css-classes")
            .transform_to(|_, v: bool| {
                if v {
                    Some(["current"].to_value())
                } else {
                    Some([].to_value())
                }
            })
            .sync_create()
            .build();
    }
}

fn on_drop(
    target: &DropTarget,
    value: &Value,
    x: f64,
    y: f64,
    store: &gio::ListStore,
    database: &DatabasePtr,
) -> bool {
    let column_view = target.widget().unwrap().downcast::<ColumnView>().unwrap();
    column_view.grab_focus();

    let closest = find_closest(x, y, column_view.upcast_ref());
    let tracks = get_dropped_tracks(value, database);
    let sm = column_view.model().unwrap();

    if closest == column_view {
        sm.unselect_all();

        let db = database.read().unwrap();
        for track in tracks {
            store.append(&PlaylistItem::new(track, &db));
            sm.select_item(store.n_items() - 1, false);
        }
        if store.n_items() > 0 {
            column_view.scroll_to(store.n_items() - 1, None, ListScrollFlags::FOCUS, None);
        }
    } else if closest.type_().name() == "GtkColumnViewRowWidget" {
        let Some(index) = find_index(store, &closest) else {
            return false;
        };

        if is_in_top_half(y, &closest) {
            sm.unselect_all();

            let mut last = 0;
            let db = database.read().unwrap();
            for track in tracks.iter().rev() {
                store.insert(index, &PlaylistItem::new(*track, &db));
                sm.select_item(index, false);
                last = index;
            }

            column_view.scroll_to(last, None, ListScrollFlags::FOCUS, None);
        } else {
            sm.unselect_all();

            let mut last = 0;
            let db = database.read().unwrap();
            for track in tracks.iter().rev() {
                store.insert(index + 1, &PlaylistItem::new(*track, &db));
                sm.select_item(index + 1, false);
                last = index + 1;
            }

            column_view.scroll_to(last, None, ListScrollFlags::FOCUS, None);
        }
    } else {
        println!("unknown type {}", closest.type_())
    }

    true
}

fn get_dropped_tracks(value: &Value, database: &DatabasePtr) -> Vec<TrackId> {
    let dropped = value.get::<ObjectIds>().unwrap();
    let mut tracks = Vec::new();
    let database = database.read().unwrap();

    for item in dropped {
        tracks.extend(get_tracks(&database, item));
    }

    tracks
}

fn get_tracks(database: &Database, item: ObjectId) -> Vec<TrackId> {
    // TODO: move resolving to drag
    match item {
        ObjectId::None => {
            vec![]
        }
        ObjectId::TrackId(track_id) => {
            vec![track_id]
        }
        ObjectId::AlbumId(album_id) => database[album_id].sorted_tracks().collect(),
        ObjectId::ArtistId(artist) => {
            let albums = database.sorted_albums_of_artist(artist);
            albums
                .into_iter()
                .flat_map(|a| database[a].sorted_tracks())
                .collect()
        }
        ObjectId::Genre(genre) => {
            let albums = database.sorted_albums_of_genre(genre);
            albums
                .into_iter()
                .flat_map(|a| database[a].sorted_tracks())
                .collect()
        }
        ObjectId::Year(year) => {
            let albums = database.sorted_albums_of_year(year);
            albums
                .into_iter()
                .flat_map(|a| database[a].sorted_tracks())
                .collect()
        }
    }
}

fn find_closest(x: f64, y: f64, column_view: &Widget) -> Widget {
    let mut closest = column_view.pick(x, y, PickFlags::DEFAULT).unwrap();

    while let Some(parent) = closest.parent()
        && closest != *column_view
        && closest.type_().name() != "GtkColumnViewRowWidget"
    {
        closest = parent;
    }

    closest
}

fn is_in_top_half(y: f64, row: &Widget) -> bool {
    let mut parent = row.parent().unwrap();
    while parent.type_() != ColumnView::static_type() {
        parent = parent.parent().unwrap();
    }

    let new_point = parent
        .compute_point(row, &Point::new(0.0, y as f32))
        .unwrap();

    (new_point.y() as i32) < row.height() / 2
}

fn find_index(store: &gio::ListStore, row: &Widget) -> Option<u32> {
    let entry_uuid = row
        .first_child()?
        .first_child()?
        .downcast::<BoxWithPlaylistEntry>()
        .ok()?
        .playlist();

    store.find_with_equal_func(|o| o.downcast_ref::<PlaylistItem>().unwrap().uuid() == entry_uuid)
}

fn on_delete(widget: &Widget, _args: Option<&Variant>, store: &gio::ListStore) -> Propagation {
    let widget = widget.downcast_ref::<ColumnView>().unwrap();
    let sm = widget.model().unwrap();

    let mut to_remove = HashSet::new();
    for i in 0..sm.n_items() {
        if sm.is_selected(i) {
            let item = sm.item(i).unwrap().downcast::<PlaylistItem>().unwrap();
            to_remove.insert(item);
        }
    }

    store.retain(|e| !to_remove.contains(e));

    Propagation::Stop
}

fn prepare_drag(
    source: &DragSource,
    x: f64,
    y: f64,
    store: &gio::ListStore,
) -> Option<gdk::ContentProvider> {
    let widget = source.widget().and_downcast::<ColumnView>().unwrap();
    let sm = widget.model().unwrap();

    let closest = find_closest(x, y, &source.widget().unwrap());
    if let Some(index) = find_index(store, &closest) {
        if !sm.is_selected(index) {
            sm.select_item(index, true);
        }
    }

    let mut object_ids = ObjectIds::new();
    for i in 0..sm.n_items() {
        if sm.is_selected(i) {
            let row = sm.item(i).and_downcast::<PlaylistItem>().unwrap();
            object_ids.push(row.stored_track().into());
            object_ids.mark_entry_for_removal(row.uuid());
        }
    }

    let value = Value::from(object_ids);
    let content = gdk::ContentProvider::for_value(&value);

    Some(content)
}

fn drag_end(source: &DragSource, drag: &Drag, remove: bool, store: &gio::ListStore) {
    if !remove {
        return;
    }

    if let Ok(content) = drag.content().value(ObjectIds::static_type()) {
        let content = content.get::<ObjectIds>().unwrap();
        let to_remove = content.entries_to_remove();

        store.retain(|e| !to_remove.contains(&e.downcast_ref::<PlaylistItem>().unwrap().uuid()));

        let column_view = source.widget().and_downcast::<ColumnView>().unwrap();
        let sm = column_view.model().unwrap();

        let last = (0..sm.n_items())
            .map(|i| sm.is_selected(i))
            .rposition(|i| i);

        if let Some(last) = last {
            column_view.scroll_to(last as u32, None, ListScrollFlags::FOCUS, None);
        }
    }
}

fn save_playlist(store: &gio::ListStore, config: &ConfigPtr) {
    let config = config.read().unwrap();

    fs::create_dir_all(config.playlists_path().parent().unwrap()).unwrap();
    match fs::File::create(config.playlists_path()) {
        Err(e) => {
            println!("Error creating playlist file: {}", e);
        }
        Ok(file) => {
            let playlist: Vec<TrackId> = store
                .iter::<PlaylistItem>()
                .filter_map(|i| i.ok())
                .map(|p| p.stored_track())
                .collect();

            serde_json::to_writer(file, &playlist).unwrap();
        }
    }
}
