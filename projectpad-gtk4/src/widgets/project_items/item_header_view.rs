/// This can be a PROJECT item or a SERVER item
/// So we just say Item...
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
    #[properties(wrapper_type = super::ItemHeaderView)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_items/item_header_view.ui"
    )]
    pub struct ItemHeaderView {
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
    impl ObjectSubclass for ItemHeaderView {
        const NAME: &'static str = "ItemHeaderView";
        type ParentType = adw::Bin;
        type Type = super::ItemHeaderView;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ItemHeaderView {
        fn constructed(&self) {
            // assuming all the properties are set at once if modified
            // let _ = self
            //     .obj()
            //     .connect_title_notify(|header: &super::ItemHeaderView| {});
        }
    }

    impl WidgetImpl for ItemHeaderView {}

    impl adw::subclass::prelude::BinImpl for ItemHeaderView {}
}

glib::wrapper! {
    pub struct ItemHeaderView(ObjectSubclass<imp::ItemHeaderView>)
        @extends gtk::Widget, adw::Bin;
}

impl ItemHeaderView {
    pub fn new(project_item_type: ProjectItemType, icon: Option<&'static str>) -> Self {
        let this = glib::Object::new::<Self>();

        this.imp()
            .header_icon
            .set_icon_name(icon.or(Some(project_item_type.get_icon())));

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
