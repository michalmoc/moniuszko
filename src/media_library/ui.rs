use crate::database::{AlbumId, Database, DatabasePtr, ObjectId};
use crate::media_library::grouping_mode::{Category, GroupingModePtr};
use crate::media_library::ui_item::MediaListItem;
use crate::playlist::ObjectIds;
use gio::prelude::{ListModelExt, ObjectExt, StaticType};
use gtk4::glib::{Object, Value, spawn_future_local};
use gtk4::prelude::{
    BoxExt, Cast, CastNone, EventControllerExt, ListItemExt, SelectionModelExt, WidgetExt,
};
use gtk4::{
    DragSource, Image, Label, ListView, MultiSelection, Orientation, PickFlags,
    SignalListItemFactory, TreeExpander, TreeListModel, TreeListRow, Widget, gdk, gio, glib,
};
use lofty::picture::PictureType;
use lofty::prelude::TaggedFileExt;
use lofty::probe::Probe;

#[derive(Clone)]
pub struct Ui {
    top_store: gio::ListStore,
    tree_store: TreeListModel,
    widget: ListView,
    database: DatabasePtr,
    grouping_mode: GroupingModePtr,
}

impl Ui {
    pub fn new(database: DatabasePtr, grouping_mode: GroupingModePtr) -> Self {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(tree_setup);
        let database_clone = database.clone();
        factory.connect_bind(move |_, i| tree_bind(i, &database_clone));

        let store = gio::ListStore::new::<MediaListItem>();
        let database_clone = database.clone();
        let grouping_mode_clone = grouping_mode.clone();
        let model = TreeListModel::new(store.clone(), false, false, move |i| {
            create(i, &database_clone, &grouping_mode_clone)
        });

        let selection = MultiSelection::new(Some(model.clone()));
        let drag_source = DragSource::new();
        drag_source.connect_prepare(drag_prepare);

        let tree = ListView::new(Some(selection), Some(factory));
        tree.add_controller(drag_source);
        tree.add_css_class("navigation-sidebar");

        Self {
            top_store: store,
            widget: tree,
            tree_store: model,
            database,
            grouping_mode,
        }
    }

    pub fn connect_activate<F>(&self, f: F)
    where
        F: Fn(ObjectId) + 'static,
    {
        let store = self.tree_store.clone();
        self.widget.connect_activate(move |_, p| {
            let row = store.item(p).and_downcast::<TreeListRow>().unwrap();
            let item = row.item().and_downcast::<MediaListItem>().unwrap();
            f(item.stored_object());
        });
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
    }

    pub fn repopulate(&self) {
        self.top_store.remove_all();

        let db = self.database.read().unwrap();
        let mode = self.grouping_mode.get();

        match mode.top_category() {
            Category::Track => {
                for track_id in db.sorted_tracks() {
                    self.top_store
                        .append(&MediaListItem::new_track(track_id, &db));
                }
            }
            Category::Album => {
                for album_id in db.sorted_albums() {
                    self.top_store
                        .append(&MediaListItem::new_album(album_id, &db));
                }
            }
            Category::Artist => {
                for artist_id in db.sorted_artists() {
                    self.top_store
                        .append(&MediaListItem::new_artist(artist_id, &db));
                }
            }
            Category::Genre => {
                for genre in db.sorted_genres() {
                    self.top_store.append(&MediaListItem::new_genre(genre));
                }
            }
            Category::Year => {
                for year in db.sorted_years() {
                    self.top_store.append(&MediaListItem::new_year(year));
                }
            }
        }
    }
}

fn tree_setup(_factory: &SignalListItemFactory, list_item: &Object) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let expander = TreeExpander::new();
    list_item.set_child(Some(&expander));
}

fn tree_bind(list_item: &Object, database: &DatabasePtr) {
    let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

    let expander = list_item.child().and_downcast::<TreeExpander>().unwrap();
    let row = list_item.item().and_downcast::<TreeListRow>().unwrap();
    expander.set_list_row(Some(&row));

    let dataobj = row.item().and_downcast::<MediaListItem>().unwrap();

    let label = Label::new(None);
    dataobj
        .bind_property("name", &label, "label")
        .sync_create()
        .build();

    match dataobj.stored_object() {
        ObjectId::None => {}
        ObjectId::TrackId(_) => {
            expander.set_child(Some(&label));
        }
        ObjectId::AlbumId(album) => {
            let image = Image::new();
            image.add_css_class("large-icons");
            dataobj
                .bind_property("image", &image, "paintable")
                .sync_create()
                .build();

            if dataobj.image().is_none() {
                let database_clone = database.clone();
                spawn_future_local(async move {
                    let img = gio::spawn_blocking(move || {
                        load_image(album, &database_clone.read().unwrap())
                    })
                    .await
                    .unwrap();
                    dataobj.set_image(img);
                });
            }

            let box_ = gtk4::Box::new(Orientation::Horizontal, 10);
            box_.append(&image);
            box_.append(&label);
            expander.set_child(Some(&box_));
        }
        ObjectId::ArtistId(_) => {
            expander.set_child(Some(&label));
        }
        ObjectId::Genre(_) => {
            expander.set_child(Some(&label));
        }
        ObjectId::Year(_) => {
            label.add_css_class("numeric");
            expander.set_child(Some(&label));
        }
    }
}

fn create(
    item: &Object,
    database: &DatabasePtr,
    grouping_mode: &GroupingModePtr,
) -> Option<gio::ListModel> {
    let item = item.downcast_ref::<MediaListItem>().unwrap();

    match item.stored_object() {
        ObjectId::None => None,
        ObjectId::TrackId(_) => None,
        ObjectId::AlbumId(album_id) => {
            let store = gio::ListStore::new::<MediaListItem>();

            let db = database.read().unwrap();
            for track in db.sorted_tracks_of_album(album_id) {
                store.append(&MediaListItem::new_track(track, &db));
            }

            Some(store.upcast())
        }
        ObjectId::ArtistId(artist_id) => {
            let store = gio::ListStore::new::<MediaListItem>();

            let db = database.read().unwrap();
            for album in db.sorted_albums_of_artist(artist_id) {
                store.append(&MediaListItem::new_album(album, &db));
            }

            Some(store.upcast())
        }
        ObjectId::Genre(genre) => {
            let store = gio::ListStore::new::<MediaListItem>();

            let db = database.read().unwrap();
            for album in db.sorted_albums_of_genre(genre) {
                store.append(&MediaListItem::new_album(album, &db));
            }

            Some(store.upcast())
        }
        ObjectId::Year(year) => {
            let store = gio::ListStore::new::<MediaListItem>();

            let db = database.read().unwrap();
            for album in db.sorted_albums_of_year(year) {
                store.append(&MediaListItem::new_album(album, &db));
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

fn load_image(album: AlbumId, db: &Database) -> Option<gdk::Texture> {
    let any_track = db.sorted_tracks_of_album(album).into_iter().next()?;
    let path = &db[any_track].path;

    let tagged_file = Probe::open(path).ok()?.read().ok()?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())?;

    let pic = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;

    gdk::Texture::from_bytes(&glib::Bytes::from(pic.data())).ok()
}
