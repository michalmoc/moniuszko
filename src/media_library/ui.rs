use crate::database::{DatabasePtr, ObjectId};
use crate::media_library::ui_item::MediaListItem;
use gio::ListStore;
use gio::prelude::{ListModelExt, ObjectExt, StaticType};
use gtk4::glib::{Object, Value};
use gtk4::prelude::{
    Cast, CastNone, EventControllerExt, IsA, ListItemExt, SelectionModelExt, WidgetExt,
};
use gtk4::{
    DragSource, Label, ListView, MultiSelection, PickFlags, SignalListItemFactory, TreeExpander,
    TreeListModel, TreeListRow, Widget, gdk, gio,
};

#[derive(Clone)]
pub struct Ui {
    store: gio::ListStore,
    widget: ListView,
}

impl Ui {
    pub fn new(database: &DatabasePtr) -> Self {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(tree_setup);
        let database_clone = database.clone();
        factory.connect_bind(move |factory, item| tree_bind(factory, item, &database_clone));

        let store = gio::ListStore::new::<MediaListItem>();
        let database_clone = database.clone();
        let model = TreeListModel::new(store.clone(), false, false, move |i| {
            create(i, &database_clone)
        });

        let selection = MultiSelection::new(Some(model));
        let drag_source = DragSource::new();
        let selection_clone = selection.clone();
        drag_source
            .connect_prepare(move |source, x, y| drag_prepare(source, x, y, &selection_clone));

        let tree = ListView::new(Some(selection), Some(factory));
        tree.add_controller(drag_source);

        Self {
            store,
            widget: tree,
        }
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }

    pub fn repopulate(&self, database: &DatabasePtr) {
        self.store.remove_all();

        let mut albums = database
            .read()
            .unwrap()
            .albums
            .iter()
            .map(|(id, album)| (*id, album.title))
            .collect::<Vec<_>>();
        albums.sort_by_key(|k| k.1);

        for (album_id, _) in albums {
            self.store.append(&MediaListItem::new_album(album_id));
        }
    }
}

fn tree_setup(factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = Label::new(None);
    let expander = TreeExpander::new();
    expander.set_child(Some(&label));
    list_item.set_child(Some(&expander));
}

fn tree_bind(factory: &SignalListItemFactory, list_item: &Object, database: &DatabasePtr) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let expander = list_item.child().and_downcast::<TreeExpander>().unwrap();
    let label = expander.child().and_downcast::<Label>().unwrap();

    let row = list_item.item().and_downcast::<TreeListRow>().unwrap();
    expander.set_list_row(Some(&row));

    let dataobj = row
        .item()
        .and_downcast::<MediaListItem>()
        .unwrap()
        .get_object_id();

    match dataobj {
        ObjectId::None => label.set_label("unknown"),
        ObjectId::TrackId(track_id) => label.set_label(&database.read().unwrap()[track_id].title),
        ObjectId::AlbumId(album_id) => label.set_label(&database.read().unwrap()[album_id].title),
    }
}

fn create(item: &Object, database: &DatabasePtr) -> Option<gio::ListModel> {
    let item = item.downcast_ref::<MediaListItem>().unwrap();

    match item.get_object_id() {
        ObjectId::None => None,
        ObjectId::TrackId(_) => None,
        ObjectId::AlbumId(album_id) => {
            let store = gio::ListStore::new::<MediaListItem>();

            for (_, track) in &database.read().unwrap()[album_id].tracks {
                store.append(&MediaListItem::new_track(*track));
            }

            Some(store.upcast())
        }
    }
}

fn drag_prepare(
    drag_source: &DragSource,
    x: f64,
    y: f64,
    selection: &MultiSelection,
) -> Option<gdk::ContentProvider> {
    // TODO: selection is inside drag_source
    let mut current = drag_source
        .widget()
        .unwrap()
        .pick(x, y, PickFlags::DEFAULT)
        .unwrap();
    while current.type_() != TreeExpander::static_type() {
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            return None;
        }
    }
    let current = current.downcast::<TreeExpander>().unwrap();
    let current = current.list_row().unwrap();

    for i in 0..selection.n_items() {
        if selection.item(i).unwrap() == current && !selection.is_selected(i) {
            selection.select_item(i, true);
            break;
        }
    }

    let mut object_ids = ListStore::new::<MediaListItem>();
    for i in 0..selection.n_items() {
        if selection.is_selected(i) {
            let row = selection
                .item(i)
                .unwrap()
                .downcast::<TreeListRow>()
                .unwrap();
            let dataobj = row.item().and_downcast::<MediaListItem>().unwrap();
            object_ids.append(&dataobj);
        }
    }

    let value = Value::from(object_ids);
    let content = gdk::ContentProvider::for_value(&value);
    Some(content)
}
