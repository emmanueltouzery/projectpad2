use projectpadsql::models::Project;
use relm4::{
    factory::FactoryVecDeque,
    gtk::{
        self,
        traits::{BoxExt, OrientableExt},
    },
    prelude::SimpleComponent,
    ComponentParts, ComponentSender,
};

use crate::AppMsg;

use super::project_badge::ProjectBadge;

#[derive(Debug)]
pub struct ProjectList {
    active_project_id: Option<i32>,
    badges: FactoryVecDeque<ProjectBadge>,
}

#[derive(Debug)]
pub enum ProjectListMsg {
    GotProjectList(Vec<Project>),
}

#[relm4::component(pub)]
impl SimpleComponent for ProjectList {
    type Init = ();
    type Input = ProjectListMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            #[local_ref]
            badge_box -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let badges = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());
        let model = ProjectList {
            active_project_id: None,
            badges,
        };

        let badge_box = model.badges.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        let mut badges_guard = self.badges.guard();
        match msg {
            ProjectListMsg::GotProjectList(projects) => {
                badges_guard.clear();
                for prj in projects {
                    badges_guard.push_back((prj, false));
                }
            }
        }
    }
}
