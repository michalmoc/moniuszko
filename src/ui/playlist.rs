use gtk4::Widget;
use gtk4::glib;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct PlaylistUi(ObjectSubclass<imp::PlaylistUi>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl PlaylistUi {
    pub fn request_delete_selected(&self) {
        imp::on_delete_selected(self.imp().view.borrow().as_ref().unwrap(), self);
    }
}

mod imp {
    use crate::data::object_id::ObjectIds;
    use crate::data::playlist_entry_uuid::PlaylistEntryUuids;
    use crate::ui::box_with_playlist_entry::BoxWithPlaylistEntry;
    use crate::ui::dnd_item::DndItem;
    use crate::ui::playlist_item::PlaylistItem;
    use adw::glib::Propagation;
    use adw::prelude::{Cast, ObjectExt};
    use gettextrs::gettext;
    use gio::prelude::ListModelExt;
    use gtk4::gdk::{DragAction, Key, ModifierType};
    use gtk4::glib::subclass::Signal;
    use gtk4::glib::{Object, Properties, Value, clone};
    use gtk4::graphene::Point;
    use gtk4::prelude::StaticTypeExt;
    use gtk4::prelude::{BoxExt, CastNone, SelectionModelExt, StaticType, ToValue, WidgetExt};
    use gtk4::prelude::{EventControllerExt, ListItemExt};
    use gtk4::subclass::prelude::*;
    use gtk4::{
        Align, CallbackAction, ColumnView, ColumnViewColumn, DragSource, DropTarget, KeyvalTrigger,
        Label, ListScrollFlags, MultiSelection, PickFlags, SelectionModel, Shortcut,
        ShortcutController, SignalListItemFactory, Widget, gdk, glib,
    };
    use log::error;
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::PlaylistUi)]
    pub struct PlaylistUi {
        #[property(get, construct_only)]
        playlist: RefCell<Option<gio::ListStore>>,

        pub view: RefCell<Option<ColumnView>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistUi {
        const NAME: &'static str = "PlaylistUi";
        type Type = super::PlaylistUi;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            PlaylistItem::ensure_type();
            klass.set_layout_manager_type::<gtk4::BinLayout>();
            klass.set_css_name("playlist");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistUi {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let playlist_ref = self.playlist.borrow();
            let playlist = playlist_ref.as_ref().unwrap();

            let view = new_view(playlist, &obj);
            view.set_parent(&*obj);

            view.connect_activate(clone!(
                #[strong]
                obj,
                move |_, pos| {
                    obj.emit_by_name::<()>("activate", &[&pos]);
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
                    Signal::builder("request-remove-tracks")
                        .param_types([PlaylistEntryUuids::static_type()])
                        .build(),
                    Signal::builder("request-add-tracks")
                        .param_types([ObjectIds::static_type(), u32::static_type()])
                        .build(),
                    Signal::builder("request-move-tracks")
                        .param_types([PlaylistEntryUuids::static_type(), u32::static_type()])
                        .build(),
                    Signal::builder("activate")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for PlaylistUi {}

    pub fn new_view(playlist: &gio::ListStore, this: &super::PlaylistUi) -> ColumnView {
        let selection = MultiSelection::new(Some(playlist.clone()));
        let view = ColumnView::new(Some(selection));

        let drop_target = DropTarget::new(DndItem::static_type(), DragAction::all());
        drop_target.connect_drop(clone!(
            #[weak]
            this,
            #[upgrade_or_default]
            move |t, v, x, y| on_drop(t, v, x, y, &this)
        ));

        let drag_source = DragSource::new();
        drag_source.set_actions(DragAction::MOVE);
        drag_source.connect_prepare(prepare_drag);

        let shortcut_controller = ShortcutController::new();
        shortcut_controller.add_shortcut(
            Shortcut::builder()
                .trigger(&KeyvalTrigger::new(Key::Delete, ModifierType::empty()))
                .action(&CallbackAction::new(clone!(
                    #[weak]
                    this,
                    #[upgrade_or]
                    Propagation::Proceed,
                    move |w, _| {
                        on_delete_selected(w.downcast_ref().unwrap(), &this);
                        Propagation::Stop
                    }
                )))
                .build(),
        );

        view.add_controller(drag_source);
        view.add_controller(drop_target);
        view.add_controller(shortcut_controller);

        view.append_column(&Column::new_numeric(&gettext("column-track"), "position").build());
        view.append_column(&Column::new_text(&gettext("column-title"), "name").build());
        view.append_column(&Column::new_text(&gettext("column-artists"), "artists").build());
        view.append_column(&Column::new_text(&gettext("column-album"), "album").build());
        view.append_column(&Column::new_numeric(&gettext("column-duration"), "duration").build());

        playlist.connect_items_changed(clone!(
            #[weak]
            view,
            move |_, pos, _, _| {
                let len = view.model().unwrap().n_items();
                let pos = if len == 0 {
                    return;
                } else if len <= pos {
                    len - 1
                } else {
                    pos
                };
                view.scroll_to(pos, None, ListScrollFlags::FOCUS, None)
            }
        ));

        view
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

            let column = ColumnViewColumn::new(Some(&gettext(&self.title)), Some(factory));
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
        this: &super::PlaylistUi,
    ) -> bool {
        let column_view = target.widget().unwrap().downcast::<ColumnView>().unwrap();
        column_view.grab_focus();

        let closest = find_closest(x, y, column_view.upcast_ref());
        let sm = column_view.model().unwrap();

        let index = if closest == column_view {
            u32::MAX
        } else if closest.type_().name() == "GtkColumnViewRowWidget" {
            let Some(index) = find_index(&sm, &closest) else {
                return false;
            };

            if is_in_top_half(y, &closest) {
                index
            } else {
                index + 1
            }
        } else {
            error!("unknown type {}", closest.type_());
            return false;
        };

        match value.get::<DndItem>().unwrap() {
            DndItem::Add(to_add) => {
                this.emit_by_name::<()>("request-add-tracks", &[&to_add, &index]);
            }
            DndItem::Move(to_move) => {
                this.emit_by_name::<()>("request-move-tracks", &[&to_move, &index]);
            }
        }

        true
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

    fn find_index(store: &SelectionModel, row: &Widget) -> Option<u32> {
        let entry_uuid = row
            .first_child()?
            .first_child()?
            .downcast::<BoxWithPlaylistEntry>()
            .ok()?
            .playlist();

        for i in 0..store.n_items() {
            let item = store.item(i).unwrap().downcast::<PlaylistItem>().unwrap();
            if item.uuid() == entry_uuid {
                return Some(i);
            }
        }

        None
    }

    fn prepare_drag(source: &DragSource, x: f64, y: f64) -> Option<gdk::ContentProvider> {
        let widget = source.widget().and_downcast::<ColumnView>().unwrap();
        let sm = widget.model().unwrap();

        let closest = find_closest(x, y, &source.widget().unwrap());
        if let Some(index) = find_index(&sm, &closest) {
            if !sm.is_selected(index) {
                sm.select_item(index, true);
            }
        }

        let mut drag = PlaylistEntryUuids::default();
        for i in 0..sm.n_items() {
            if sm.is_selected(i) {
                let row = sm.item(i).and_downcast::<PlaylistItem>().unwrap();
                drag.insert(row.uuid());
            }
        }

        let value = Value::from(DndItem::Move(drag));
        let content = gdk::ContentProvider::for_value(&value);

        Some(content)
    }

    pub fn on_delete_selected(widget: &ColumnView, this: &super::PlaylistUi) {
        let sm = widget.model().unwrap();

        let mut to_remove = PlaylistEntryUuids::default();
        for i in 0..sm.n_items() {
            if sm.is_selected(i) {
                let item = sm.item(i).unwrap().downcast::<PlaylistItem>().unwrap();
                to_remove.insert(item.uuid());
            }
        }

        this.emit_by_name::<()>("request-remove-tracks", &[&to_remove]);
    }
}
