use gtk4::glib;
use gtk4::glib::Object;

mod imp {
    use gtk4::glib;
    use gtk4::glib::Properties;
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::box_::BoxImpl;
    use gtk4::subclass::prelude::DerivedObjectProperties;
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass, WidgetImpl};
    use std::cell::RefCell;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::BoxWithData)]
    pub struct BoxWithData {
        #[property(get, set)]
        custom_data: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BoxWithData {
        const NAME: &'static str = "BoxWithData";
        type Type = super::BoxWithData;
        type ParentType = gtk4::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for BoxWithData {}

    impl WidgetImpl for BoxWithData {}

    impl BoxImpl for BoxWithData {}
}

glib::wrapper! {
    pub struct BoxWithData(ObjectSubclass<imp::BoxWithData>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Orientable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl BoxWithData {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
