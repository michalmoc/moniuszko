use crate::database::{DatabasePtr, ObjectId, TrackId};
use crate::playlist::ObjectIds;
use crate::playlist::box_with_data::BoxWithData;
use crate::playlist::ui_item::PlaylistItem;
use gio::prelude::ListModelExt;
use gtk4::gdk::{Drag, DragAction, Key, ModifierType};
use gtk4::glib::{Object, Propagation, Value, Variant};
use gtk4::graphene::Point;
use gtk4::prelude::{
    BoxExt, Cast, CastNone, ContentProviderExtManual, DragExt, EventControllerExt, ListItemExt,
    ObjectExt, SelectionModelExt, StaticType, WidgetExt,
};
use gtk4::{
    CallbackAction, ColumnView, ColumnViewColumn, DragSource, DropTarget, KeyvalTrigger, Label,
    ListScrollFlags, MultiSelection, PickFlags, Shortcut, ShortcutController,
    SignalListItemFactory, Widget, gdk, gio,
};
use std::collections::HashSet;

pub struct Ui {
    store: gio::ListStore,
    widget: ColumnView,
}

impl Ui {
    pub fn new(database: &DatabasePtr) -> Self {
        let store = gio::ListStore::new::<PlaylistItem>();
        let selection = MultiSelection::new(Some(store.clone()));

        let factory1 = SignalListItemFactory::new();
        factory1.connect_setup(tree_setup1);
        let database_clone = database.clone();
        factory1.connect_bind(move |_, item| tree_bind1(item, &database_clone));
        let column1 = ColumnViewColumn::new(Some("#"), Some(factory1));

        let factory2 = SignalListItemFactory::new();
        factory2.connect_setup(tree_setup2);
        let database_clone = database.clone();
        factory2.connect_bind(move |_, item| tree_bind2(item, &database_clone));
        let column2 = ColumnViewColumn::new(Some("title"), Some(factory2));

        let factory3 = SignalListItemFactory::new();
        factory3.connect_setup(tree_setup3);
        let database_clone = database.clone();
        factory3.connect_bind(move |_, item| tree_bind3(item, &database_clone));
        let column3 = ColumnViewColumn::new(Some("album"), Some(factory3));

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
        view.append_column(&column1);
        view.append_column(&column2);
        view.append_column(&column3);

        Self {
            store,
            widget: view,
        }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }
}

fn tree_setup1(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = Label::new(None);

    let box_ = BoxWithData::new();
    box_.append(&label);

    list_item.set_child(Some(&box_));
}

fn tree_bind1(list_item: &Object, database: &DatabasePtr) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let box_ = list_item.child().and_downcast::<BoxWithData>().unwrap();
    let label = box_.first_child().and_downcast::<Label>().unwrap();

    let dataobj = list_item.item().and_downcast::<PlaylistItem>().unwrap();
    let uuid = dataobj.uuid();
    let track = dataobj.get_track_id();

    label.set_label(&database.read().unwrap()[track].position.to_string());
    box_.set_custom_data(uuid);
}

fn tree_setup2(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = Label::new(None);

    list_item.set_child(Some(&label));
}

fn tree_bind2(list_item: &Object, database: &DatabasePtr) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = list_item.child().and_downcast::<Label>().unwrap();

    let dataobj = list_item.item().and_downcast::<PlaylistItem>().unwrap();
    let track = dataobj.get_track_id();

    label.set_label(&database.read().unwrap()[track].title);
}

fn tree_setup3(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = Label::new(None);

    list_item.set_child(Some(&label));
}

fn tree_bind3(list_item: &Object, database: &DatabasePtr) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = list_item.child().and_downcast::<Label>().unwrap();

    let dataobj = list_item.item().and_downcast::<PlaylistItem>().unwrap();
    let track = dataobj.get_track_id();
    let album = database.read().unwrap()[track].album;
    let name = database.read().unwrap()[album].title;

    label.set_label(&name);
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
    let closest = find_closest(x, y, column_view.upcast_ref());
    let tracks = get_dropped_tracks(value, database);
    let sm = column_view.model().unwrap();

    if closest == column_view {
        sm.unselect_all();

        for track in tracks {
            store.append(&PlaylistItem::new(track));
            sm.select_item(store.n_items() - 1, false);
        }
        column_view.scroll_to(store.n_items() - 1, None, ListScrollFlags::FOCUS, None);
    } else if closest.type_().name() == "GtkColumnViewRowWidget" {
        let Some(index) = find_index(store, &closest) else {
            return false;
        };

        if is_in_top_half(y, &closest) {
            sm.unselect_all();

            let mut last = 0;
            for track in tracks.iter().rev() {
                store.insert(index, &PlaylistItem::new(*track));
                sm.select_item(index, false);
                last = index;
            }

            column_view.scroll_to(last, None, ListScrollFlags::FOCUS, None);
        } else {
            sm.unselect_all();

            let mut last = 0;
            for track in tracks.iter().rev() {
                store.insert(index + 1, &PlaylistItem::new(*track));
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
        match item {
            ObjectId::None => {}
            ObjectId::TrackId(track_id) => {
                tracks.push(track_id);
            }
            ObjectId::AlbumId(album_id) => {
                tracks.extend(database[album_id].tracks.values());
            }
        }
    }

    tracks
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
        .downcast::<BoxWithData>()
        .ok()?
        .custom_data();

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
            object_ids.push(row.get_track_id().into());
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
