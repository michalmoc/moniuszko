use crate::database::{DatabasePtr, ObjectId};
use crate::media_library::ui_item::MediaListItem;
use crate::playlist::ObjectIds;
use gio::prelude::{ListModelExt, ObjectExt, StaticType};
use gtk4::glib::{Object, Value};
use gtk4::prelude::{
    Cast, CastNone, EventControllerExt, ListItemExt, SelectionModelExt, WidgetExt,
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
        factory.connect_bind(tree_bind);

        let store = gio::ListStore::new::<MediaListItem>();
        let database_clone = database.clone();
        let model = TreeListModel::new(store.clone(), false, false, move |i| {
            create(i, &database_clone)
        });

        let selection = MultiSelection::new(Some(model));
        let drag_source = DragSource::new();
        drag_source.connect_prepare(drag_prepare);

        let tree = ListView::new(Some(selection), Some(factory));
        tree.add_controller(drag_source);
        tree.add_css_class("navigation-sidebar");

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

        let db = database.read().unwrap();

        let mut albums = db
            .albums
            .iter()
            .map(|(id, album)| (*id, album.title))
            .collect::<Vec<_>>();
        albums.sort_by_key(|k| k.1);

        for (album_id, _) in albums {
            self.store.append(&MediaListItem::new_album(album_id, &db));
        }
    }
}

fn tree_setup(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let label = Label::new(None);
    let expander = TreeExpander::new();
    expander.set_child(Some(&label));
    list_item.set_child(Some(&expander));
}

fn tree_bind(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let expander = list_item.child().and_downcast::<TreeExpander>().unwrap();
    let label = expander.child().and_downcast::<Label>().unwrap();

    let row = list_item.item().and_downcast::<TreeListRow>().unwrap();
    expander.set_list_row(Some(&row));

    let dataobj = row.item().and_downcast::<MediaListItem>().unwrap();
    dataobj
        .bind_property("name", &label, "label")
        .sync_create()
        .build();
}

fn create(item: &Object, database: &DatabasePtr) -> Option<gio::ListModel> {
    let item = item.downcast_ref::<MediaListItem>().unwrap();

    match item.stored_object() {
        ObjectId::None => None,
        ObjectId::TrackId(_) => None,
        ObjectId::AlbumId(album_id) => {
            let store = gio::ListStore::new::<MediaListItem>();

            let db = database.read().unwrap();
            for (_, track) in &db[album_id].tracks {
                store.append(&MediaListItem::new_track(*track, &db));
            }

            Some(store.upcast())
        }
    }
}

fn drag_prepare(drag_source: &DragSource, x: f64, y: f64) -> Option<gdk::ContentProvider> {
    let list_view = drag_source.widget().and_downcast::<ListView>().unwrap();
    let selection = list_view.model().unwrap();

    let mut current = list_view.pick(x, y, PickFlags::DEFAULT).unwrap();
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

    let mut object_ids = ObjectIds::new();
    for i in 0..selection.n_items() {
        if selection.is_selected(i) {
            let row = selection
                .item(i)
                .unwrap()
                .downcast::<TreeListRow>()
                .unwrap();
            let dataobj = row.item().and_downcast::<MediaListItem>().unwrap();
            object_ids.push(dataobj.stored_object());
        }
    }

    let value = Value::from(object_ids);
    let content = gdk::ContentProvider::for_value(&value);
    Some(content)
}
