use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::{EnvironmentType, Project, ProjectNote, Server, ServerLink};

use crate::string_sidecar_object::StringSidecarObject;

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
    pub fn new(project_id: i32, project_item_id: i32, project_item_type: ProjectItemType) -> Self {
        let this = glib::Object::new::<Self>();

        let contents_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(15)
            .margin_start(15)
            .margin_end(15)
            .margin_bottom(15)
            .spacing(10)
            .build();

        let projects_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            let prj = prj::project.load::<Project>(sql_conn).unwrap();

            let prj_item_env = match project_item_type {
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

        env_combo
            .bind_property("selected", &this, "environment")
            .transform_to(move |binding, number: u32| {
                let combo = binding
                    .source()
                    .unwrap()
                    .downcast::<adw::ComboRow>()
                    .unwrap();
                if let Some(model) = combo.model() {
                    let env = model
                        .item(number)
                        .unwrap()
                        .downcast::<StringSidecarObject>()
                        .unwrap()
                        .sidecar();
                    Some(env.to_value())
                } else {
                    None
                }
            })
            .sync_create()
            .build();

        let t = this.clone();
        glib::spawn_future_local(async move {
            let (projects, item_env) = projects_recv.recv().await.unwrap();
            let project_names = projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>();
            let dropdown_entries_store = gtk::StringList::new(&project_names);
            project_combo.set_model(Some(&dropdown_entries_store));
            project_combo
                .set_selected(projects.iter().position(|p| p.id == project_id).unwrap() as u32);

            let ps = projects.clone();
            project_combo
                .bind_property("selected", &t, "project_id")
                .transform_to(move |_, number: u32| {
                    Some(ps.get(number as usize).unwrap().id.to_value())
                })
                .sync_create()
                .build();

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

                let model = gio::ListStore::new::<StringSidecarObject>();

                for (env_string, env) in env_strings.iter().zip(sorted_envs.iter()) {
                    model.append(&StringSidecarObject::new(
                        (*env_string).to_owned(),
                        *env as i32,
                    ));
                }
                env_combo.set_model(Some(&model));
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

        this.set_child(Some(&contents_vbox));

        this
    }
}
