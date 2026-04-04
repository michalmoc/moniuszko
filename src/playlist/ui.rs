use crate::database::{DatabasePtr, ObjectId, Track, TrackId};
use crate::media_library::MediaListItem;
use crate::playlist::box_with_data::BoxWithData;
use crate::playlist::ui_item::PlaylistItem;
use gio::prelude::ListModelExt;
use gtk4::gdk::DragAction;
use gtk4::glib::{Object, Type, Value};
use gtk4::graphene::Point;
use gtk4::prelude::{
    BoxExt, Cast, CastNone, EventControllerExt, ListItemExt, ObjectExt, ObjectType, StaticType,
    WidgetExt,
};
use gtk4::{
    ColumnView, ColumnViewColumn, DropTarget, Label, ListStore, MultiSelection, PickFlags,
    SignalListItemFactory, TreeListRow, Widget, gio,
};

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
        let column1 = ColumnViewColumn::new(Some("title"), Some(factory1));

        let drop_target = DropTarget::new(gio::ListStore::static_type(), DragAction::all());
        let store_clone = store.clone();
        let database_clone = database.clone();
        drop_target
            .connect_drop(move |t, v, x, y| on_drop(t, v, x, y, &store_clone, &database_clone));

        let view = ColumnView::new(Some(selection));
        view.add_controller(drop_target);
        view.append_column(&column1);

        Self {
            store,
            widget: view,
        }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }
}

fn tree_setup1(factory: &SignalListItemFactory, list_item: &Object) {
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

    label.set_label(&database.read().unwrap()[track].title);
    box_.set_custom_data(uuid);
}

fn on_drop(
    target: &DropTarget,
    value: &Value,
    x: f64,
    y: f64,
    store: &gio::ListStore,
    database: &DatabasePtr,
) -> bool {
    // TODO: reject drop on headers. Accept only ListStore of MediaLibraryItem

    println!("on_drop");

    let column_view = target.widget().unwrap();
    let closest = find_closest(x, y, &column_view);
    let tracks = get_dropped_tracks(value, database);

    if closest == column_view {
        for track in tracks {
            store.append(&PlaylistItem::new(track));
        }
    } else if closest.type_().name() == "GtkColumnViewRowWidget" {
        let Some(index) = find_index(store, &closest) else {
            return false;
        };
        println!("index: {:?}", index);

        if is_in_top_half(y, &closest) {
            for track in tracks.iter().rev() {
                store.insert(index, &PlaylistItem::new(*track));
            }
        } else {
            for track in tracks.iter().rev() {
                store.insert(index + 1, &PlaylistItem::new(*track));
            }
        }
    } else {
        println!("unknown type {}", closest.type_())
    }

    true
}

fn get_dropped_tracks(value: &Value, database: &DatabasePtr) -> Vec<TrackId> {
    let dropped = value.get::<gio::ListStore>().unwrap();
    let mut tracks = Vec::new();
    let database = database.read().unwrap();

    for i in 0..dropped.n_items() {
        let item = dropped
            .item(i)
            .unwrap()
            .downcast::<MediaListItem>()
            .unwrap();
        match item.get_object_id() {
            ObjectId::None => {}
            ObjectId::TrackId(track_id) => {
                tracks.push(track_id);
            }
            ObjectId::AlbumId(album_id) => {
                tracks.extend(&database[album_id].tracks);
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
