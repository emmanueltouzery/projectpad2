use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod imp {
    use std::cell::Cell;

    use super::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectItem)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item.ui")]
    pub struct ProjectItem {
        #[template_child]
        pub project_item: TemplateChild<adw::Bin>,

        #[property(get, set)]
        edit_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItem {
        const NAME: &'static str = "ProjectItem";
        type ParentType = adw::Bin;
        type Type = super::ProjectItem;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItem {
        fn constructed(&self) {
            //     self.obj().init_list();
            let _ = self
                .obj()
                .connect_edit_mode_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.display_item_id(0);
                });
            //     "edit-mode",
            //     false,
            //     closure_local!(move |project_item: i32, edit_mode: bool| {
            //         println!("edit mode changed: {edit_mode}");
            //     }),
            // );
        }
    }

    impl WidgetImpl for ProjectItem {}

    impl adw::subclass::prelude::BinImpl for ProjectItem {}
}

glib::wrapper! {
    pub struct ProjectItem(ObjectSubclass<imp::ProjectItem>)
        @extends gtk::Widget, adw::Bin;
}

impl ProjectItem {
    pub fn display_item_id(&self, id: i32) {
        println!("projectitem::display_item_id({id})");
        super::project_items::server::display_server(
            &self.imp().project_item,
            id,
            self.edit_mode(),
        );
    }
}
