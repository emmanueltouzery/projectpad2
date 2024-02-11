use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::{
    app::ProjectpadApplication,
    widgets::{project_item_model::ProjectItemType, project_items::note},
};

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

        #[property(get, set)]
        pub item_id: Cell<i32>,

        #[property(get, set)]
        pub project_item_type: Cell<u8>,
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
                    // TODO maybe don't reload widgets completely when toggling edit mode?
                    project_item.display_item();
                });
            // TODO this is crappy. the owner sets project_item_type and then item_id and we react
            // only on the second. there must be a better way...
            let _ = self
                .obj()
                .connect_item_id_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.display_item();
                });
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
    fn display_item(&self) {
        println!("projectitem::display_item_id({})", self.imp().item_id.get());
        let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
        // TODO receive the item type besides the item_id and switch on item type here
        // also possibly receive the ProjectItem, telling me much more than the id
        let db_sender = app.unwrap().get_sql_channel();
        let item_id = Some(self.imp().item_id.get());
        let item_type = ProjectItemType::from_repr(self.imp().project_item_type.get());
        let widget_mode = if self.edit_mode() {
            WidgetMode::Edit
        } else {
            WidgetMode::Show
        };

        match item_type {
            Some(ProjectItemType::Server) => super::project_items::server::load_and_display_server(
                &self.imp().project_item,
                db_sender,
                item_id,
                widget_mode,
            ),
            Some(ProjectItemType::ProjectNote) => {
                let note = note::Note::new();
                // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
                // it feels even worse (would like not to rebuild the widget every time...)
                note.set_project_note_id(&item_id.unwrap());
                note.set_edit_mode(self.edit_mode());
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
                super::project_items::project_poi::load_and_display_project_poi(
                    &self.imp().project_item,
                    db_sender,
                    item_id,
                    widget_mode,
                )
            }
            _ => {
                // TODO remove the fallback case
                eprintln!("unhandled item type!");
            }
        }
    }
}
