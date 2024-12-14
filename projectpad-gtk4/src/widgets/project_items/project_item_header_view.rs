use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::widgets::project_item_model::ProjectItemType;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectItemHeaderView)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_items/project_item_header_view.ui"
    )]
    pub struct ProjectItemHeaderView {
        #[template_child]
        pub header_icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub header_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub header_second_col: TemplateChild<gtk::Box>,

        #[property(get, set)]
        title: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemHeaderView {
        const NAME: &'static str = "ProjectItemHeaderView";
        type ParentType = adw::Bin;
        type Type = super::ProjectItemHeaderView;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItemHeaderView {
        fn constructed(&self) {
            // assuming all the properties are set at once if modified
            // let _ = self
            //     .obj()
            //     .connect_title_notify(|header: &super::ProjectItemHeaderView| {});
        }
    }

    impl WidgetImpl for ProjectItemHeaderView {}

    impl adw::subclass::prelude::BinImpl for ProjectItemHeaderView {}
}

glib::wrapper! {
    pub struct ProjectItemHeaderView(ObjectSubclass<imp::ProjectItemHeaderView>)
        @extends gtk::Widget, adw::Bin;
}

impl ProjectItemHeaderView {
    pub fn new(project_item_type: ProjectItemType) -> Self {
        let this = glib::Object::new::<Self>();

        this.imp()
            .header_icon
            .set_icon_name(Some(&project_item_type.get_icon()));

        // TODO add this through the UI file not the code
        let title_label = gtk::Label::builder()
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        this.bind_property("title", &title_label, "label")
            .bidirectional()
            .sync_create()
            .build();
        this.imp().header_second_col.append(&title_label);

        this
    }

    pub fn header_box(&self) -> gtk::Box {
        self.imp().header_box.clone()
    }
}
