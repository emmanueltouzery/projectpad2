use super::environment_picker::EnvironmentPicker;
use super::environment_picker::Msg::EnvironmentSelected as EnvPickerMsgEnvSelected;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::ProjectItem;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ProjectUpdated {
    Yes,
    No,
}

#[derive(Msg, Clone)]
pub enum Msg {
    GotProjectList(Result<Vec<Project>, String>),
    EnvironmentSelected(EnvironmentType),
    MoveActionTriggered,
    MoveApplied(Result<(Project, ProjectItem, ProjectUpdated), String>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    cur_project_id: i32,
    environment: EnvironmentType,
    project_item: ProjectItem,
    displayed_projects: Vec<Project>,

    _projectlist_channel: relm::Channel<Result<Vec<Project>, String>>,
    projectlist_sender: relm::Sender<Result<Vec<Project>, String>>,

    _move_applied_channel: relm::Channel<Result<(Project, ProjectItem, ProjectUpdated), String>>,
    move_applied_sender: relm::Sender<Result<(Project, ProjectItem, ProjectUpdated), String>>,
}

#[widget]
impl Widget for ProjectItemMoveDialog {
    fn init_view(&mut self) {
        self.fetch_project_list();
    }

    fn model(
        relm: &relm::Relm<ProjectItemMoveDialog>,
        cur_vals: (mpsc::Sender<SqlFunc>, ProjectItem),
    ) -> Model {
        let (db_sender, project_item) = cur_vals;
        let (cur_project_id, environment) = Self::get_project_env(&project_item);
        let stream1 = relm.stream().clone();
        let (_move_applied_channel, move_applied_sender) =
            relm::Channel::new(move |r| stream1.emit(Msg::MoveApplied(r)));
        let stream2 = relm.stream().clone();
        let (_projectlist_channel, projectlist_sender) =
            relm::Channel::new(move |r| stream2.emit(Msg::GotProjectList(r)));
        Model {
            db_sender,
            project_item,
            cur_project_id,
            environment,
            displayed_projects: vec![],
            _projectlist_channel,
            projectlist_sender,
            _move_applied_channel,
            move_applied_sender,
        }
    }

    fn get_project_env(pi: &ProjectItem) -> (i32, EnvironmentType) {
        match pi {
            ProjectItem::Server(s) => (s.project_id, s.environment),
            ProjectItem::ServerLink(sl) => (sl.project_id, sl.environment),
            ProjectItem::ProjectNote(n) => (
                n.project_id,
                if n.has_prod {
                    EnvironmentType::EnvProd
                } else if n.has_uat {
                    EnvironmentType::EnvUat
                } else if n.has_stage {
                    EnvironmentType::EnvStage
                } else {
                    EnvironmentType::EnvDevelopment
                },
            ),
            ProjectItem::ProjectPointOfInterest(ppoi) => {
                (ppoi.project_id, EnvironmentType::EnvProd)
            }
        }
    }

    fn apply_move(&mut self) {
        let selected_project = self
            .widgets
            .project_list
            .get_selected_row()
            .and_then(|row| self.model.displayed_projects.get(row.get_index() as usize))
            .map(|r| (*r).clone());
        let env = self.model.environment;
        let applied_sender = self.model.move_applied_sender.clone();
        if let Some(prj) = selected_project {
            let project_item = self.model.project_item.clone();
            self.model
                .db_sender
                .send(SqlFunc::new(move |sql_conn| {
                    applied_sender
                        .send(Self::apply_move_sql(
                            sql_conn,
                            prj.clone(),
                            project_item.clone(),
                            env,
                        ))
                        .unwrap();
                }))
                .unwrap();
        }
    }

    fn apply_move_sql(
        sql_conn: &diesel::SqliteConnection,
        prj: Project,
        project_item: ProjectItem,
        env: EnvironmentType,
    ) -> Result<(Project, ProjectItem, ProjectUpdated), String> {
        use projectpadsql::schema::project_note::dsl as prj_note;
        use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
        use projectpadsql::schema::server::dsl as srv;
        use projectpadsql::schema::server_link::dsl as srvl;

        // in case this environment was not yet active for this project, make it active
        let (is_project_updated, updated_prj) =
            Self::project_enable_env_if_needed(sql_conn, prj, env)?;

        match &project_item {
            ProjectItem::Server(s) => {
                let changeset = (srv::project_id.eq(updated_prj.id), srv::environment.eq(env));
                diesel::update(srv::server.filter(srv::id.eq(s.id)))
                    .set(changeset)
                    .execute(sql_conn)
                    .and_then(|_| srv::server.filter(srv::id.eq(s.id)).first(sql_conn))
                    .map(|s| {
                        (
                            updated_prj.clone(),
                            ProjectItem::Server(s),
                            is_project_updated,
                        )
                    })
            }
            ProjectItem::ServerLink(s) => {
                let changeset = (
                    srvl::project_id.eq(updated_prj.id),
                    srvl::environment.eq(env),
                );
                diesel::update(srvl::server_link.filter(srvl::id.eq(s.id)))
                    .set(changeset)
                    .execute(sql_conn)
                    .and_then(|_| srvl::server_link.filter(srvl::id.eq(s.id)).first(sql_conn))
                    .map(|s| {
                        (
                            updated_prj.clone(),
                            ProjectItem::ServerLink(s),
                            is_project_updated,
                        )
                    })
            }
            ProjectItem::ProjectNote(n) => {
                let update_note_env =
                    diesel::update(prj_note::project_note.filter(prj_note::id.eq(n.id)));
                match env {
                    EnvironmentType::EnvDevelopment => update_note_env
                        .set(prj_note::has_dev.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvStage => update_note_env
                        .set(prj_note::has_stage.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvUat => update_note_env
                        .set(prj_note::has_uat.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvProd => update_note_env
                        .set(prj_note::has_prod.eq(true))
                        .execute(sql_conn),
                }
                .map_err(|e| e.to_string())?;
                diesel::update(prj_note::project_note.filter(prj_note::id.eq(n.id)))
                    .set(prj_note::project_id.eq(updated_prj.id))
                    .execute(sql_conn)
                    .and_then(|_| {
                        prj_note::project_note
                            .filter(prj_note::id.eq(n.id))
                            .first(sql_conn)
                    })
                    .map(|s| {
                        (
                            updated_prj.clone(),
                            ProjectItem::ProjectNote(s),
                            is_project_updated,
                        )
                    })
            }
            ProjectItem::ProjectPointOfInterest(ppoi) => {
                let changeset = (prj_poi::project_id.eq(updated_prj.id),);
                diesel::update(prj_poi::project_point_of_interest.filter(prj_poi::id.eq(ppoi.id)))
                    .set(changeset)
                    .execute(sql_conn)
                    .and_then(|_| {
                        prj_poi::project_point_of_interest
                            .filter(prj_poi::id.eq(ppoi.id))
                            .first(sql_conn)
                    })
                    .map(|s| {
                        (
                            updated_prj.clone(),
                            ProjectItem::ProjectPointOfInterest(s),
                            is_project_updated,
                        )
                    })
            }
        }
        .map_err(|e| e.to_string())
    }

    fn project_enable_env_if_needed(
        sql_conn: &diesel::SqliteConnection,
        prj: Project,
        env: EnvironmentType,
    ) -> Result<(ProjectUpdated, Project), String> {
        use projectpadsql::schema::project::dsl as prj;
        let activate_env_result = match env {
            EnvironmentType::EnvDevelopment if !prj.has_dev => {
                diesel::update(prj::project.filter(prj::id.eq(prj.id)))
                    .set(prj::has_dev.eq(true))
                    .execute(sql_conn)
            }
            EnvironmentType::EnvStage if !prj.has_stage => {
                diesel::update(prj::project.filter(prj::id.eq(prj.id)))
                    .set(prj::has_stage.eq(true))
                    .execute(sql_conn)
            }
            EnvironmentType::EnvUat if !prj.has_uat => {
                diesel::update(prj::project.filter(prj::id.eq(prj.id)))
                    .set(prj::has_uat.eq(true))
                    .execute(sql_conn)
            }
            EnvironmentType::EnvProd if !prj.has_prod => {
                diesel::update(prj::project.filter(prj::id.eq(prj.id)))
                    .set(prj::has_prod.eq(true))
                    .execute(sql_conn)
            }
            _ => Ok(0),
        }
        .map_err(|e| e.to_string())?;

        let is_project_updated = if activate_env_result == 1 {
            ProjectUpdated::Yes
        } else {
            ProjectUpdated::No
        };

        // re-read the project since i updated the environments
        let updated_prj = if is_project_updated == ProjectUpdated::Yes {
            prj::project
                .filter(prj::id.eq(prj.id))
                .first::<Project>(sql_conn)
                .map_err(|e| e.to_string())?
        } else {
            prj
        };
        Ok((is_project_updated, updated_prj))
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotProjectList(Err(e)) => {
                standard_dialogs::display_error_str("Error loading the project list", Some(e));
            }
            Msg::GotProjectList(Ok(project_names)) => self.populate_project_list(project_names),
            Msg::EnvironmentSelected(e) => {
                self.model.environment = e;
            }
            Msg::MoveActionTriggered => self.apply_move(),
            // meant for my parent
            Msg::MoveApplied(_) => {}
        }
    }

    fn fetch_project_list(&self) {
        let sender = self.model.projectlist_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                sender
                    .send(
                        prj::project
                            .order(prj::name.asc())
                            .load(sql_conn)
                            .map_err(|e| format!("Error loading projects: {:?}", e)),
                    )
                    .unwrap();
            }))
            .unwrap();
    }

    fn populate_project_list(&mut self, projects: Vec<Project>) {
        self.model.displayed_projects = projects;
        for child in self.widgets.project_list.get_children() {
            self.widgets.project_list.remove(&child);
        }
        let mut selected_idx = 0;
        let mut idx = 0;
        for project in &self.model.displayed_projects {
            self.widgets.project_list.add(
                &gtk::LabelBuilder::new()
                    .label(&project.name)
                    .xalign(0.0)
                    .margin(5)
                    .build(),
            );
            if project.id == self.model.cur_project_id {
                selected_idx = idx;
            }
            idx += 1;
        }
        self.widgets.project_list.select_row(
            self.widgets
                .project_list
                .get_row_at_index(selected_idx)
                .as_ref(),
        );
        self.widgets.project_list.show_all();
    }

    view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            margin_top: 10,
            margin_start: 10,
            margin_end: 10,
            margin_bottom: 10,
            spacing: 10,
            gtk::Frame {
                child: {
                    expand: true,
                },
                gtk::ScrolledWindow {
                    #[name="project_list"]
                    gtk::ListBox {
                        // // https://gitlab.gnome.org/GNOME/gtk/-/issues/497
                        // activate_on_single_click: false,
                    },
                },
            },
            EnvironmentPicker(self.model.environment) {
                EnvPickerMsgEnvSelected(e) => Msg::EnvironmentSelected(e)
            }
        }
    }
}
