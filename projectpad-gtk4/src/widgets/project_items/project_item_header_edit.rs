use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::widgets::{
    environment_list_picker::EnvironmentListPicker, environment_picker::EnvironmentPicker,
    project_item_model::ProjectItemType,
};

use super::common::{ask_user, EnvOrEnvs};

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
    #[properties(wrapper_type = super::ProjectItemHeaderEdit)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_items/project_item_header_edit.ui"
    )]
    pub struct ProjectItemHeaderEdit {
        #[template_child]
        pub header_icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub header_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub header_second_col: TemplateChild<gtk::Box>,

        #[template_child]
        pub group_dropdown: TemplateChild<gtk::DropDown>,

        #[property(get, set)]
        title: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemHeaderEdit {
        const NAME: &'static str = "ProjectItemHeaderEdit";
        type ParentType = adw::Bin;
        type Type = super::ProjectItemHeaderEdit;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItemHeaderEdit {
        fn constructed(&self) {
            // assuming all the properties are set at once if modified
            // let _ = self
            //     .obj()
            //     .connect_title_notify(|header: &super::ProjectItemHeaderEdit| {});
        }
    }

    impl WidgetImpl for ProjectItemHeaderEdit {}

    impl adw::subclass::prelude::BinImpl for ProjectItemHeaderEdit {}
}

glib::wrapper! {
    pub struct ProjectItemHeaderEdit(ObjectSubclass<imp::ProjectItemHeaderEdit>)
        @extends gtk::Widget, adw::Bin;
}

/// for the group names, i could require just the project id,
/// but the problem are notes, for which we share the code
/// between project notes and server notes (and these are
/// different groups...)
impl ProjectItemHeaderEdit {
    pub fn new(
        project_item_type: ProjectItemType,
        group_name: Option<&str>,
        all_group_names: &[String],
        env: EnvOrEnvs,
    ) -> Self {
        let this = glib::Object::new::<Self>();

        this.imp()
            .header_icon
            .set_icon_name(Some(&project_item_type.get_icon()));

        // TODO add this through the UI file not the code
        let title_entry = gtk::Entry::builder()
            .halign(gtk::Align::Fill)
            .hexpand(true)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        this.bind_property("title", &title_entry, "text")
            .bidirectional()
            .sync_create()
            .build();
        this.imp().header_second_col.append(&title_entry);

        let environment_picker = match env {
            EnvOrEnvs::Env(e) => Some(EnvironmentPicker::new(e).upcast::<gtk::Widget>()),
            EnvOrEnvs::Envs(es) => Some(EnvironmentListPicker::new(es).upcast::<gtk::Widget>()),
            EnvOrEnvs::None => None,
        };
        if let Some(ep) = environment_picker {
            ep.set_halign(gtk::Align::End);
            ep.set_hexpand(true);
            this.imp().header_box.append(&ep);
        }

        let mut group_name_items = vec!["New group..."];
        group_name_items.extend(all_group_names.iter().map(String::as_str));
        let dropdown_entries_store = gtk::StringList::new(&group_name_items);
        this.imp()
            .group_dropdown
            .set_model(Some(&dropdown_entries_store));
        let store = dropdown_entries_store.clone();
        this.imp()
            .group_dropdown
            .connect_selected_item_notify(move |dropdown: &gtk::DropDown| {
                if dropdown.selected() == 0 {
                    // new group
                    let dds = store.clone();
                    let d = dropdown.clone();
                    // new group, ask the user for the name
                    // check the other modals i created
                    ask_user(
                        "New Group",
                        "Group Name",
                        &(*dropdown).clone().upcast::<gtk::Widget>(),
                        Box::new(move |name| {
                            dds.append(&name);
                            d.set_selected(dds.n_items() - 1);
                        }),
                    );
                }
            });

        if let Some(gn) = group_name {
            if let Some(pos) = all_group_names.iter().position(|x| x == gn) {
                this.imp().group_dropdown.set_selected(
                    (pos + 1/* 1 due to the default entries no group+new group */) as u32,
                );
            }
        } else {
            this.imp()
                .group_dropdown
                .set_selected(dropdown_entries_store.n_items() + 1); // 1 past the end => "none"
        }

        this
    }

    pub fn header_box(&self) -> gtk::Box {
        self.imp().header_box.clone()
    }
}