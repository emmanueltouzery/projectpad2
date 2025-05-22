use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};

use super::{
    environment_picker::{self, EnvironmentPicker},
    project_items::common,
};

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::MoveProjectItem)]
    pub struct MoveProjectItem {
        #[property(get, set)]
        project_id: Rc<RefCell<i32>>,

        #[property(get, set)]
        environment: Rc<RefCell<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MoveProjectItem {
        const NAME: &'static str = "MoveProjectItem";
        type ParentType = adw::Bin;
        type Type = super::MoveProjectItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MoveProjectItem {}

    impl WidgetImpl for MoveProjectItem {}

    impl adw::subclass::prelude::BinImpl for MoveProjectItem {}
}

glib::wrapper! {
    pub struct MoveProjectItem(ObjectSubclass<imp::MoveProjectItem>)
        @extends gtk::Widget, adw::Bin;
}

impl MoveProjectItem {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header_bar = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(false)
            .build();

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        header_bar.pack_start(&cancel_btn);
        vbox.append(&header_bar);

        let contents_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(15)
            .margin_start(15)
            .margin_end(15)
            .margin_bottom(15)
            .spacing(10)
            .build();

        let projects_recv = common::run_sqlfunc(Box::new(|sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            prj::project.load::<Project>(sql_conn)
        }));

        contents_vbox.append(
            &gtk::Label::builder()
                .label("Select where to move the project item to:")
                .halign(gtk::Align::Start)
                .build(),
        );

        let prefs_group = adw::PreferencesGroup::builder().build();
        let project_combo = adw::ComboRow::builder().title("Project").build();
        prefs_group.add(&project_combo);

        glib::spawn_future_local(async move {
            let projects = projects_recv.recv().await.unwrap().unwrap();
            let project_names = projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>();
            let dropdown_entries_store = gtk::StringList::new(&project_names);
            project_combo.set_model(Some(&dropdown_entries_store));
        });

        let env_combo = adw::ComboRow::builder().title("Environment").build();

        let list_item_factory = environment_picker::dropdown_get_factory(
            &env_combo,
            environment_picker::DropDownFactoryMode::ListItem,
        );
        let item_factory = environment_picker::dropdown_get_factory(
            &env_combo,
            environment_picker::DropDownFactoryMode::Item,
        );

        env_combo.set_list_factory(Some(&list_item_factory));
        env_combo.set_factory(Some(&item_factory));

        // TODO correct env types list depending on the project
        let allowed_envs = &[EnvironmentType::EnvDevelopment, EnvironmentType::EnvProd];
        let (env_strings, sorted_envs) = EnvironmentPicker::dropdown_labels_and_vals(allowed_envs);
        env_combo.set_model(Some(&gtk::StringList::new(&env_strings)));

        prefs_group.add(&env_combo);

        contents_vbox.append(&prefs_group);

        vbox.append(&contents_vbox);

        this.set_child(Some(&vbox));

        this
    }
}
