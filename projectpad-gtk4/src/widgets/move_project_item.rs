use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::{EnvironmentType, Project, ProjectNote, Server, ServerLink};

use super::{
    environment_picker::{self, EnvironmentPicker},
    project_item_model::ProjectItemType,
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
    /// will return None if there's no currently selected project item (the current
    /// project has no project items at all, empty project)
    pub fn try_new() -> Option<Self> {
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

        let win = common::main_win();
        let select_project_item_state =
            glib::VariantDict::new(win.action_state("select-project-item").as_ref());

        let project_id = select_project_item_state
            .lookup::<i32>("project_id")
            .unwrap()
            .unwrap();

        let project_item_type = select_project_item_state
            .lookup::<Option<u8>>("item_type")
            .unwrap()
            .and_then(std::convert::identity)
            .and_then(ProjectItemType::from_repr);

        let m_project_item_id = select_project_item_state
            .lookup::<Option<i32>>("item_id")
            .unwrap()
            .unwrap();
        if m_project_item_id.is_none() {
            // the current project has no project item at all
            return None;
        }
        let project_item_id = m_project_item_id.unwrap();

        let projects_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            let prj = prj::project.load::<Project>(sql_conn).unwrap();

            let prj_item_env = match project_item_type.unwrap() {
                ProjectItemType::Server => {
                    use projectpadsql::schema::server::dsl as srv;
                    srv::server
                        .filter(srv::id.eq(&project_item_id))
                        .first::<Server>(sql_conn)
                        .unwrap()
                        .environment
                }
                ProjectItemType::ServerLink => {
                    use projectpadsql::schema::server_link::dsl as srv_link;
                    srv_link::server_link
                        .filter(srv_link::id.eq(&project_item_id))
                        .first::<ServerLink>(sql_conn)
                        .unwrap()
                        .environment
                }
                ProjectItemType::ProjectNote => {
                    use projectpadsql::schema::project_note::dsl as prj_note;
                    let note = prj_note::project_note
                        .filter(prj_note::id.eq(&project_item_id))
                        .first::<ProjectNote>(sql_conn)
                        .unwrap();
                    if note.has_prod {
                        EnvironmentType::EnvProd
                    } else if note.has_uat {
                        EnvironmentType::EnvUat
                    } else if note.has_stage {
                        EnvironmentType::EnvStage
                    } else {
                        EnvironmentType::EnvDevelopment
                    }
                }
                ProjectItemType::ProjectPointOfInterest => *prj
                    .iter()
                    .find(|p| p.id == project_id)
                    .unwrap()
                    .allowed_envs()
                    .first()
                    .unwrap(),
            };

            (prj, prj_item_env)
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

        prefs_group.add(&env_combo);

        glib::spawn_future_local(async move {
            let (projects, item_env) = projects_recv.recv().await.unwrap();
            let project_names = projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>();
            let dropdown_entries_store = gtk::StringList::new(&project_names);
            project_combo.set_model(Some(&dropdown_entries_store));
            project_combo
                .set_selected(projects.iter().position(|p| p.id == project_id).unwrap() as u32);

            fn set_envs(
                projects: &[Project],
                project_id: i32,
                env_combo: &adw::ComboRow,
                select_env: Option<EnvironmentType>,
            ) {
                let allowed_envs = projects
                    .iter()
                    .find(|p| p.id == project_id)
                    .unwrap()
                    .allowed_envs();
                let (env_strings, sorted_envs) =
                    EnvironmentPicker::dropdown_labels_and_vals(&allowed_envs);
                env_combo.set_model(Some(&gtk::StringList::new(&env_strings)));
                if let Some(env) = select_env {
                    env_combo
                        .set_selected(sorted_envs.iter().position(|e| *e == env).unwrap() as u32);
                }
            }

            let ec = env_combo.clone();
            let p = projects.clone();
            project_combo.connect_selected_notify(move |e| {
                let project = p.get(e.selected() as usize).unwrap();
                // don't select since which is the current project item
                // is now irrelevant since we work on another project
                set_envs(&p, project.id, &ec, None);
            });
            set_envs(&projects, project_id, &env_combo, Some(item_env));
        });

        contents_vbox.append(&prefs_group);

        vbox.append(&contents_vbox);

        this.set_child(Some(&vbox));

        Some(this)
    }
}
