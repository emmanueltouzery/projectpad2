use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::{
    app::ProjectpadApplication,
    widgets::{project_item_model::ProjectItemType, project_items::note},
};

mod imp {
    use std::{cell::Cell, sync::OnceLock};

    use super::*;
    use glib::subclass::Signal;
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
        pub clamp: TemplateChild<adw::Clamp>,

        #[template_child]
        pub project_item: TemplateChild<adw::Bin>,

        // these properties are meant to be set all at once
        // using GObjectExt.set_properties START
        #[property(get, set)]
        pub item_id: Cell<i32>,

        #[property(get, set)]
        pub project_item_type: Cell<u8>,

        #[property(get, set)]
        pub sub_item_id: Cell<i32>,
        // these properties are meant to be set all at once
        // using GObjectExt.set_properties END
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
            let _ = self
                .obj()
                .connect_item_id_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.refresh_item();
                });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("request-scroll")
                    .param_types([f32::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ProjectItem {}

    impl adw::subclass::prelude::BinImpl for ProjectItem {}
}

glib::wrapper! {
    pub struct ProjectItem(ObjectSubclass<imp::ProjectItem>)
        @extends gtk::Widget, adw::Bin;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetMode {
    Show,
    Edit,
}

impl WidgetMode {
    pub fn get_edit_mode(&self) -> bool {
        match &self {
            WidgetMode::Show => false,
            _ => true,
        }
    }
}

impl ProjectItem {
    pub fn refresh_item(&self) {
        let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
        let item_id = self.imp().item_id.get();
        let sub_item_id = Some(self.imp().sub_item_id.get());
        let item_type = ProjectItemType::from_repr(self.imp().project_item_type.get());
        // TODO receive the item type besides the item_id and switch on item type here
        // also possibly receive the ProjectItem, telling me much more than the id
        let db_sender = app.unwrap().get_sql_channel();

        // reset the scroll, who knows what we were displaying before
        self.emit_by_name::<()>("request-scroll", &[&0f32]);

        match item_type {
            Some(ProjectItemType::Server) => {
                self.imp().clamp.set_maximum_size(750);
                super::project_items::server::load_and_display_server(
                    &self.imp().project_item,
                    db_sender,
                    item_id,
                    sub_item_id,
                    &self,
                )
            }
            Some(ProjectItemType::ProjectNote) => {
                self.imp().clamp.set_maximum_size(1100);
                let note = note::Note::new();
                // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
                // it feels even worse (would like not to rebuild the widget every time...)
                // move to set_properties with freeze_notify
                note.set_project_note_id(&item_id);
                note.set_edit_mode(false);
                self.imp().project_item.set_child(Some(
                    // &note::Note::new().set_note_id(&glib::Value::from(item_id)),
                    &note,
                ));
                //     db_sender,
                //     item_id,
                //     widget_mode,
                // )
            }
            Some(ProjectItemType::ProjectPointOfInterest) => {
                self.imp().clamp.set_maximum_size(750);
                super::project_items::project_poi::load_and_display_project_poi(
                    &self.imp().project_item,
                    db_sender,
                    item_id,
                )
            }
            Some(ProjectItemType::ServerLink) => {
                self.imp().clamp.set_maximum_size(750);
                super::project_items::server_link::load_and_display_server_link(
                    &self.imp().project_item,
                    db_sender,
                    item_id,
                    self,
                )
            }
            None => panic!(),
        }
    }
}
