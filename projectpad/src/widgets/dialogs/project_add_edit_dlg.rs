use super::dialog_helpers;
use super::environments_picker::Msg::EnvironmentToggled as EnvironmentsPickerMsgEnvToggled;
use super::environments_picker::{EnvironmentsPicker, SelectedEnvironments};
use super::file_contents_button::FileContentsButton;
use super::file_contents_button::Msg::FileChanged as FileContentsButtonFileChanged;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    EnvironmentToggled(EnvironmentType),
    IconChanged((Option<String>, Option<Vec<u8>>)),
    OkPressed,
    ProjectUpdated(Project),
    HideInfobar,
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<Project, (String, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ProjectAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    _project_updated_channel: relm::Channel<SaveResult>,
    project_updated_sender: relm::Sender<SaveResult>,
    project_id: Option<i32>,

    name: String,
    icon: Option<Vec<u8>>,
    icon_desc: Option<String>,
    has_dev: bool,
    has_stg: bool,
    has_uat: bool,
    has_prod: bool,

    infobar: gtk::InfoBar,
    infobar_label: gtk::Label,
}

#[widget]
impl Widget for ProjectAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_infobar_overlay();
    }

    fn init_infobar_overlay(&self) {
        self.infobar_overlay.add_overlay(&self.model.infobar);
        self.infobar_overlay
            .set_overlay_pass_through(&self.model.infobar, true);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, Option<Project>, gtk::AccelGroup),
    ) -> Model {
        let (db_sender, project, _) = params;
        let p = project.as_ref();
        let stream = relm.stream().clone();
        let (project_updated_channel, project_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(prj) => stream.emit(Msg::ProjectUpdated(prj)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let name = p.map(|p| p.name.clone()).unwrap_or_else(|| "".to_string());
        let icon = p.and_then(|p| p.icon.clone()).filter(|i| !i.is_empty());
        let infobar = gtk::InfoBarBuilder::new()
            .revealed(false)
            .message_type(gtk::MessageType::Info)
            .valign(gtk::Align::Start)
            .build();

        let infobar_label = gtk::LabelBuilder::new().label("").build();
        infobar_label.show();
        infobar.get_content_area().add(&infobar_label);
        infobar.show();
        Model {
            relm: relm.clone(),
            db_sender,
            project_updated_sender,
            _project_updated_channel: project_updated_channel,
            project_id: p.map(|p| p.id),
            icon_desc: Self::icon_desc(&name, &icon),
            name,
            icon,
            has_dev: p.map(|p| p.has_dev).unwrap_or(false),
            has_stg: p.map(|p| p.has_stage).unwrap_or(false),
            has_uat: p.map(|p| p.has_uat).unwrap_or(false),
            has_prod: p.map(|p| p.has_prod).unwrap_or(false),
            infobar,
            infobar_label,
        }
    }

    fn show_infobar(&self, msg: &str) {
        self.model.infobar_label.set_text(msg);
        self.model.infobar.set_revealed(true);
        relm::timeout(self.model.relm.stream(), 1500, || Msg::HideInfobar);
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment) => {
                self.model.has_dev = !self.model.has_dev;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvStage) => {
                self.model.has_stg = !self.model.has_stg;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvUat) => {
                self.model.has_uat = !self.model.has_uat;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvProd) => {
                self.model.has_prod = !self.model.has_prod;
            }
            Msg::IconChanged((_, contents)) => {
                self.model.icon = contents;
                self.model.icon_desc = Self::icon_desc(&self.model.name, &self.model.icon);
            }
            Msg::OkPressed => {
                if !(self.model.has_dev
                    || self.model.has_stg
                    || self.model.has_uat
                    || self.model.has_prod)
                {
                    self.show_infobar("Please pick at least one environment");
                    return;
                }
                self.update_project();
            }
            Msg::HideInfobar => {
                self.model.infobar.set_revealed(false);
            }
            // for my parent
            Msg::ProjectUpdated(_) => {}
        }
    }

    fn update_project(&self) {
        let project_id = self.model.project_id;
        let new_name = self.name_entry.get_text();
        let new_icon = self.model.icon.clone();
        let new_has_dev = self.model.has_dev;
        let new_has_stg = self.model.has_stg;
        let new_has_uat = self.model.has_uat;
        let new_has_prod = self.model.has_prod;
        let s = self.model.project_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let changeset = (
                    prj::name.eq(new_name.as_str()),
                    prj::has_dev.eq(new_has_dev),
                    prj::has_stage.eq(new_has_stg),
                    prj::has_uat.eq(new_has_uat),
                    prj::has_prod.eq(new_has_prod),
                    // TODO the icon is actually not-null in SQL...
                    prj::icon.eq(Some(new_icon.clone().unwrap_or_default())),
                );
                let project_after_result = perform_insert_or_update!(
                    sql_conn,
                    project_id,
                    prj::project,
                    prj::id,
                    changeset,
                    Project,
                );
                s.send(project_after_result).unwrap();
            }))
            .unwrap();
    }

    fn icon_desc(name: &str, icon: &Option<Vec<u8>>) -> Option<String> {
        Some(name.clone())
            .filter(|_| icon.is_some())
            .map(|n| format!("<{} picture>", &n))
    }

    view! {
        #[name="infobar_overlay"]
        gtk::Overlay {
            #[name="grid"]
            gtk::Grid {
                gtk::Label {
                    text: "Name",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    },
                },
                #[name="name_entry"]
                gtk::Entry {
                    hexpand: true,
                    text: &self.model.name,
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    },
                },
                EnvironmentsPicker(SelectedEnvironments {
                    has_dev: self.model.has_dev,
                    has_stg: self.model.has_stg,
                    has_uat: self.model.has_uat,
                    has_prod: self.model.has_prod,
                }) {
                    cell: {
                        left_attach: 0,
                        top_attach: 2,
                        width: 2,
                    },
                    EnvironmentsPickerMsgEnvToggled(env_type) => Msg::EnvironmentToggled(env_type)
                },
                gtk::Label {
                    text: "Icon",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 3,
                    },
                },
                FileContentsButton((
                    self.model.icon_desc.clone(),
                    self.model.icon.clone(),
                )) {
                    FileContentsButtonFileChanged(ref val) => Msg::IconChanged(val.clone()),
                    cell: {
                        left_attach: 1,
                        top_attach: 3,
                    },
                },
            }
        }
    }
}
