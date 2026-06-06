use glib::Object;
use gtk4::{gio, glib};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder().property("application", app).build()
    }
}

mod imp {
    use adw::subclass::prelude::AdwApplicationWindowImpl;
    use fluent_zero::{lookup_static, t};
    use gtk4::glib::subclass::InitializingObject;
    use gtk4::subclass::prelude::{
        ApplicationWindowImpl, CompositeTemplateCallbacks, CompositeTemplateClass, ObjectImpl,
        ObjectSubclass,
    };
    use gtk4::subclass::widget::{CompositeTemplateInitializingExt, WidgetImpl};
    use gtk4::subclass::window::WindowImpl;
    use gtk4::{CompositeTemplate, glib};
    use std::borrow::Cow;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/moniuszko/window.ui")]
    pub struct Window {}

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "MyGtkAppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            UtilityCallbacks::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {}

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}

    struct UtilityCallbacks {}

    #[gtk4::template_callbacks(functions)]
    impl UtilityCallbacks {
        #[template_callback]
        fn translate(value: &str) -> &str {
            let Cow::Borrowed(s) = t!(value) else {
                panic!()
            };
            s
        }
    }
}
