use crate::commands::Command;
use crate::playlist::box_with_playlist_entry::BoxWithPlaylistEntry;
use crate::playlist::ui_item::PlaylistItem;
use crate::playlist::{ObjectIds, Playlist};
use adw::glib::Propagation;
use async_channel::Sender;
use fluent_zero::t;
use gio::prelude::ListModelExt;
use gtk4::gdk::{Drag, DragAction, Key, ModifierType};
use gtk4::glib::{Object, Value, clone};
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
use gtk4::{SelectionModel, glib};
use std::collections::HashSet;

#[derive(Clone)]
pub struct Ui {
    widget: ColumnView,
}

impl Ui {
    pub fn new(playlist: Playlist, commands: Sender<Command>) -> Self {
        let selection = MultiSelection::new(Some(playlist.inner().clone()));

        let drop_target = DropTarget::new(ObjectIds::static_type(), DragAction::all());
        drop_target.connect_drop(clone!(
            #[strong]
            commands,
            move |t, v, x, y| on_drop(t, v, x, y, &commands)
        ));

        let drag_source = DragSource::new();
        drag_source.set_actions(DragAction::MOVE);
        drag_source.connect_prepare(prepare_drag);
        drag_source.connect_drag_end(clone!(
            #[strong]
            commands,
            move |_, d, r| drag_end(d, r, &commands)
        ));
        let shortcut_controller = ShortcutController::new();

        let view = ColumnView::new(Some(selection));

        shortcut_controller.add_shortcut(
            Shortcut::builder()
                .trigger(&KeyvalTrigger::new(Key::Delete, ModifierType::empty()))
                .action(&CallbackAction::new(move |view, _| {
                    on_delete_selected(view.downcast_ref().unwrap(), &commands);
                    Propagation::Stop
                }))
                .build(),
        );

        view.add_controller(drag_source);
        view.add_controller(drop_target);
        view.add_controller(shortcut_controller);

        view.append_column(&Column::new_numeric("column-track", "position").build());
        view.append_column(&Column::new_text("column-title", "name").build());
        view.append_column(&Column::new_text("column-artists", "artists").build());
        view.append_column(&Column::new_text("column-album", "album").build());
        view.append_column(&Column::new_numeric("column-duration", "duration").build());

        playlist.connect_changed_listener(clone!(
            #[weak]
            view,
            move |pos| view.scroll_to(pos, None, ListScrollFlags::FOCUS, None)
        ));

        Self { widget: view }
    }

    pub fn connect_activate<F: Fn(u32) + 'static>(&self, f: F) {
        self.widget.connect_activate(move |_, p| f(p));
    }

    pub fn delete_selected(&self, commands: &Sender<Command>) {
        on_delete_selected(&self.widget, commands);
    }

    pub fn widget(&self) -> Widget {
        self.widget.clone().upcast()
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

        let column = ColumnViewColumn::new(Some(&t!(&self.title)), Some(factory));
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

fn on_drop(target: &DropTarget, value: &Value, x: f64, y: f64, commands: &Sender<Command>) -> bool {
    let column_view = target.widget().unwrap().downcast::<ColumnView>().unwrap();
    column_view.grab_focus();

    let closest = find_closest(x, y, column_view.upcast_ref());
    let obj_ids = value.get::<ObjectIds>().unwrap();
    let sm = column_view.model().unwrap();

    if closest == column_view {
        commands
            .send_blocking(Command::AppendToPlaylist(obj_ids))
            .unwrap();
    } else if closest.type_().name() == "GtkColumnViewRowWidget" {
        let Some(index) = find_index(&sm, &closest) else {
            return false;
        };

        if is_in_top_half(y, &closest) {
            commands
                .send_blocking(Command::InsertInPlaylist(obj_ids, index))
                .unwrap();
        } else {
            commands
                .send_blocking(Command::InsertInPlaylist(obj_ids, index + 1))
                .unwrap();
        }
    } else {
        println!("unknown type {}", closest.type_())
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

fn drag_end(drag: &Drag, remove: bool, commands: &Sender<Command>) {
    if !remove {
        return;
    }

    if let Ok(content) = drag.content().value(ObjectIds::static_type()) {
        let content = content.get_owned::<ObjectIds>().unwrap();
        let to_remove = content.entries_to_remove();

        commands
            .send_blocking(Command::RemoveFromPlaylist(to_remove))
            .unwrap();
    }
}

pub fn on_delete_selected(widget: &ColumnView, commands: &Sender<Command>) {
    let sm = widget.model().unwrap();

    let mut to_remove = HashSet::new();
    for i in 0..sm.n_items() {
        if sm.is_selected(i) {
            let item = sm.item(i).unwrap().downcast::<PlaylistItem>().unwrap();
            to_remove.insert(item.uuid());
        }
    }

    commands
        .send_blocking(Command::RemoveFromPlaylist(to_remove))
        .unwrap();
}
