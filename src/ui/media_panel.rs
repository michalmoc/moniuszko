use crate::db::database::DatabasePtr;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{Button, Widget, glib};

glib::wrapper! {
    pub struct MediaPanel(ObjectSubclass<imp::MediaPanel>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl MediaPanel {
    pub fn refresh_button(&self) -> &Button {
        &self.imp().refresh_button
    }

    pub fn repopulate(&self) {
        self.imp().media_library.repopulate()
    }

    pub fn bind_data(&self, database_ptr: DatabasePtr) {
        self.imp().media_library.bind_data(database_ptr);
    }
}

mod imp {
    use crate::data::grouping_mode::GroupingMode;
    use crate::data::object_id::ObjectId;
    use crate::db::database::AvailableDatabases;
    use crate::ui::media_library::MediaLibraryUi;
    use adw::glib::subclass::InitializingObject;
    use gtk4::glib::Properties;
    use gtk4::glib::subclass::Signal;
    use gtk4::prelude::{CastNone, EditableExt, ObjectExt, OrientableExt, StaticType, WidgetExt};
    use gtk4::subclass::prelude::{
        CompositeTemplateCallbacksClass, CompositeTemplateClass, DerivedObjectProperties,
        ObjectImpl, ObjectImplExt, ObjectSubclass, ObjectSubclassExt, WidgetClassExt,
    };
    use gtk4::subclass::widget::{CompositeTemplateInitializingExt, WidgetImpl};
    use gtk4::{
        Button, CompositeTemplate, DropDown, Orientation, SearchEntry, StringList, StringObject,
        TemplateChild, Widget, glib, template_callbacks,
    };
    use std::cell::Cell;
    use std::sync::OnceLock;

    #[derive(Properties, CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/media_panel.ui")]
    #[properties(wrapper_type = super::MediaPanel)]
    pub struct MediaPanel {
        #[property(get, construct_only, default)]
        pub subdatabase: Cell<AvailableDatabases>,

        #[template_child]
        pub search_entry: TemplateChild<SearchEntry>,

        #[template_child]
        pub grouping_mode: TemplateChild<DropDown>,

        #[template_child]
        pub refresh_button: TemplateChild<Button>,

        #[template_child]
        pub media_library: TemplateChild<MediaLibraryUi>,
        // pub database: RefCell<Option<DatabasePtr>>,
        // pub search_result: RefCell<Option<SearchResultPtr>>,
        // pub grouping_mode: RefCell<Option<GroupingModePtr>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaPanel {
        const NAME: &'static str = "MediaPanel";
        type Type = super::MediaPanel;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_layout_manager_type::<gtk4::BoxLayout>();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaPanel {
        fn constructed(&self) {
            self.parent_constructed();

            let layout_manager = self
                .obj()
                .layout_manager()
                .and_downcast::<gtk4::BoxLayout>()
                .unwrap();
            layout_manager.set_orientation(Orientation::Vertical);

            // TODO change into enum
            let grouping_mode_list = StringList::new(&[]);
            for e in GroupingMode::all_str() {
                grouping_mode_list.append(&e);
            }
            self.grouping_mode.set_model(Some(&grouping_mode_list));
            self.grouping_mode.set_selected(1);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("activate")
                        .param_types([ObjectId::static_type()])
                        .build(),
                    Signal::builder("refresh").build(),
                ]
            })
        }
    }

    impl WidgetImpl for MediaPanel {}

    #[template_callbacks]
    impl MediaPanel {
        #[template_callback]
        fn handle_search_changed(&self, entry: &SearchEntry) {
            self.media_library.set_search_text(entry.text())
        }

        #[template_callback]
        fn handle_grouping_mode_change(&self) {
            let selected = self
                .grouping_mode
                .selected_item()
                .and_downcast::<StringObject>()
                .unwrap()
                .string();

            self.media_library
                .set_grouping_mode(GroupingMode::from_str(&selected).unwrap());
        }

        #[template_callback]
        fn handle_library_activate(&self, obj: ObjectId) {
            self.obj().emit_by_name::<()>("activate", &[&obj]);
        }

        #[template_callback]
        fn handle_refresh(&self) {
            self.obj().emit_by_name::<()>("refresh", &[]);
        }
    }
}
