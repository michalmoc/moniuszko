use crate::data::category::Category;
use crate::db::database::DatabasePtr;
use crate::ui::media_library_item::MediaLibraryItem;
use gtk4::prelude::{Cast, CastNone};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{MultiSelection, TreeListModel, Widget, glib};
use std::cell::Ref;

glib::wrapper! {
    pub struct MediaLibraryUi(ObjectSubclass<imp::MediaLibraryUi>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}
impl MediaLibraryUi {
    pub fn bind_data(&self, database: DatabasePtr) {
        self.imp().database.replace(Some(database));
    }

    fn database(&self) -> Ref<'_, DatabasePtr> {
        Ref::map(self.imp().database.borrow(), |r| r.as_ref().unwrap())
    }

    pub fn repopulate(&self) {
        let view_ptr = self.imp().view.borrow();
        let view = view_ptr.as_ref().unwrap();
        let store = view
            .model()
            .and_downcast::<MultiSelection>()
            .unwrap()
            .model()
            .and_downcast::<TreeListModel>()
            .unwrap()
            .model()
            .downcast::<gio::ListStore>()
            .unwrap();

        store.remove_all();

        if self.imp().database.borrow().is_none() {
            return;
        }

        let database_ptr = self.database();
        let database = database_ptr.read().unwrap();
        let subdb = database.get_subdb(self.subdatabase());

        let search_result = self.imp().search_result.borrow();

        match self.grouping_mode().top_category() {
            Category::Track => {
                for track_id in subdb.sorted_tracks() {
                    if search_result.has_track(track_id) {
                        store.append(&MediaLibraryItem::new_track(track_id, vec![], &subdb));
                    }
                }
            }
            Category::Album => {
                for album_id in subdb.sorted_albums() {
                    if search_result.has_album(album_id) {
                        store.append(&MediaLibraryItem::new_album(album_id, vec![], &subdb));
                    }
                }
            }
            Category::Artist => {
                for artist_id in subdb.sorted_artists() {
                    if search_result.has_artist(artist_id) {
                        store.append(&MediaLibraryItem::new_artist(artist_id, vec![], &subdb));
                    }
                }
            }
            Category::Genre => {
                for genre in subdb.sorted_genres() {
                    if search_result.has_genre(genre) {
                        store.append(&MediaLibraryItem::new_genre(genre, vec![]));
                    }
                }
            }
            Category::Year => {
                for year in subdb.sorted_years() {
                    if search_result.has_year(year) {
                        store.append(&MediaLibraryItem::new_year(year, vec![]));
                    }
                }
            }
        }
    }
}

mod imp {
    use crate::data::category::Category;
    use crate::data::grouping_mode::GroupingMode;
    use crate::data::object_id::{ObjectId, ObjectIds};
    use crate::db::database::{AvailableDatabases, DatabasePtr};
    use crate::db::search_result::SearchResult;
    use crate::ui::dnd_item::DndItem;
    use crate::ui::media_library_item::MediaLibraryItem;
    use adw::glib;
    use adw::glib::Properties;
    use adw::glib::subclass::Signal;
    use adw::subclass::prelude::{ObjectImpl, ObjectImplExt, ObjectSubclass, ObjectSubclassExt};
    use gio::prelude::ListModelExt;
    use gtk4::glib::{Object, Value, clone};
    use gtk4::prelude::{
        BoxExt, Cast, CastNone, EventControllerExt, ListItemExt, ObjectExt, SelectionModelExt,
        StaticType, WidgetExt,
    };
    use gtk4::subclass::prelude::{DerivedObjectProperties, ObjectSubclassIsExt};
    use gtk4::subclass::prelude::{WidgetClassExt, WidgetImpl};
    use gtk4::{
        DragSource, Image, Label, ListView, MultiSelection, Orientation, PickFlags,
        SignalListItemFactory, TreeExpander, TreeListModel, TreeListRow, Widget, gdk,
    };
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MediaLibraryUi)]
    pub struct MediaLibraryUi {
        #[property(get, set, default)]
        pub subdatabase: Cell<AvailableDatabases>,

        #[property(get, set)]
        pub search_text: RefCell<String>,

        #[property(get, set, default)]
        pub grouping_mode: RefCell<GroupingMode>,

        pub view: RefCell<Option<ListView>>,

        pub database: RefCell<Option<DatabasePtr>>,
        pub search_result: RefCell<SearchResult>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaLibraryUi {
        const NAME: &'static str = "MediaLibraryUi";
        type Type = super::MediaLibraryUi;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk4::BinLayout>();
            klass.set_css_name("media_library");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaLibraryUi {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.connect_subdatabase_notify(|this| this.imp().on_subdb_change());
            obj.connect_search_text_notify(|this| this.imp().on_search_changed());
            obj.connect_grouping_mode_notify(|this| this.imp().on_grouping_changed());

            let view = new_view(obj.clone());
            view.set_parent(&*obj);
            view.connect_activate(clone!(
                #[strong]
                obj,
                move |w, p| {
                    let sm = w.model().and_downcast::<MultiSelection>().unwrap();
                    let store = sm.model().unwrap();

                    let row = store.item(p).and_downcast::<TreeListRow>().unwrap();
                    let item = row.item().and_downcast::<MediaLibraryItem>().unwrap();
                    obj.emit_by_name::<()>("activate", &[&item.stored_object()]);
                }
            ));
            self.view.replace(Some(view));
        }

        fn dispose(&self) {
            if let Some(child) = self.view.borrow_mut().take() {
                child.unparent();
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("activate")
                        .param_types([ObjectId::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for MediaLibraryUi {}

    impl MediaLibraryUi {
        fn on_search_changed(&self) {
            if let Some(db_ptr) = self.database.borrow().as_ref() {
                let db = db_ptr.read().unwrap();
                let subdb = db.get_subdb(self.subdatabase.get());

                self.search_result
                    .replace(subdb.search(&self.search_text.borrow()));
            } else {
                self.search_result.replace(SearchResult::default());
            }

            self.obj().repopulate()
        }

        fn on_grouping_changed(&self) {
            self.obj().repopulate()
        }

        fn on_subdb_change(&self) {
            self.on_search_changed()
        }
    }

    fn new_view(obj: super::MediaLibraryUi) -> ListView {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(tree_setup);
        factory.connect_bind(move |_, i| tree_bind(i));

        let store = gio::ListStore::new::<MediaLibraryItem>();
        let model = TreeListModel::new(store.clone(), false, false, move |i| create(i, &obj));

        let selection = MultiSelection::new(Some(model.clone()));
        let drag_source = DragSource::new();
        drag_source.connect_prepare(drag_prepare);

        let tree = ListView::new(Some(selection), Some(factory));
        tree.add_controller(drag_source);
        tree.add_css_class("navigation-sidebar");

        tree
    }

    fn tree_setup(_factory: &SignalListItemFactory, list_item: &Object) {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

        let expander = TreeExpander::new();
        list_item.set_child(Some(&expander));
    }

    fn tree_bind(list_item: &Object) {
        let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();

        let expander = list_item.child().and_downcast::<TreeExpander>().unwrap();
        let row = list_item.item().and_downcast::<TreeListRow>().unwrap();
        expander.set_list_row(Some(&row));

        let dataobj = row.item().and_downcast::<MediaLibraryItem>().unwrap();

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
            ObjectId::AlbumId(_) => {
                let image = Image::new();
                image.add_css_class("large-icons");
                dataobj
                    .bind_property("image", &image, "file")
                    .sync_create()
                    .build();

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

    fn create(item: &Object, obj: &super::MediaLibraryUi) -> Option<gio::ListModel> {
        let item = item.downcast_ref::<MediaLibraryItem>().unwrap();
        let grouping_mode = obj.grouping_mode();

        let current = item.stored_object();
        let current_category = Category::of(&current);
        let next_category = grouping_mode.next_category(current_category);

        let mut filters = item.filters();
        filters.push(current);

        let store = gio::ListStore::new::<MediaLibraryItem>();

        let database_ptr = obj.database();
        let database = database_ptr.read().unwrap();
        let subdb = database.get_subdb(obj.subdatabase());

        let search_result = obj.imp().search_result.borrow();

        match (current, next_category) {
            (ObjectId::None, _) => None,
            (ObjectId::TrackId(_), _) => None,
            (ObjectId::AlbumId(album_id), Category::Track) => {
                for track in subdb.sorted_tracks_of_album(album_id) {
                    if subdb.track_matches_filter(track, &filters) && search_result.has_track(track)
                    {
                        store.append(&MediaLibraryItem::new_track(track, filters.clone(), &subdb));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::ArtistId(artist_id), Category::Album) => {
                for album in subdb.sorted_albums_of_artist(artist_id) {
                    if subdb.album_matches_filter(album, &filters) && search_result.has_album(album)
                    {
                        store.append(&MediaLibraryItem::new_album(album, filters.clone(), &subdb));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::ArtistId(artist_id), Category::Year) => {
                for year in subdb.sorted_years_of_artist(artist_id) {
                    // TODO: _matches_filter
                    if search_result.has_year(year) {
                        store.append(&MediaLibraryItem::new_year(year, filters.clone()));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::Genre(genre), Category::Album) => {
                for album in subdb.sorted_albums_of_genre(genre) {
                    if subdb.album_matches_filter(album, &filters) && search_result.has_album(album)
                    {
                        store.append(&MediaLibraryItem::new_album(album, filters.clone(), &subdb));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::Genre(genre), Category::Year) => {
                for year in subdb.sorted_years_of_genre(genre) {
                    // TODO: _matches_filter
                    if search_result.has_year(year) {
                        store.append(&MediaLibraryItem::new_year(year, filters.clone()));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::Genre(genre), Category::Artist) => {
                for artist in subdb.sorted_artists_of_genre(genre) {
                    // TODO: _matches_filter
                    if search_result.has_artist(artist) {
                        store.append(&MediaLibraryItem::new_artist(
                            artist,
                            filters.clone(),
                            &subdb,
                        ));
                    }
                }

                Some(store.upcast())
            }
            (ObjectId::Year(year), Category::Album) => {
                for album in subdb.sorted_albums_of_year(year) {
                    if subdb.album_matches_filter(album, &filters) && search_result.has_album(album)
                    {
                        store.append(&MediaLibraryItem::new_album(album, filters.clone(), &subdb));
                    }
                }

                Some(store.upcast())
            }
            _ => panic!(
                "cannot get {:?} out of {:?}",
                next_category, current_category
            ),
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

        let mut drop = ObjectIds::new();
        for i in 0..selection.n_items() {
            if selection.is_selected(i) {
                let row = selection
                    .item(i)
                    .unwrap()
                    .downcast::<TreeListRow>()
                    .unwrap();
                let dataobj = row.item().and_downcast::<MediaLibraryItem>().unwrap();
                drop.push(dataobj.stored_object());
            }
        }

        let value = Value::from(DndItem::Add(drop));
        let content = gdk::ContentProvider::for_value(&value);
        Some(content)
    }
}
