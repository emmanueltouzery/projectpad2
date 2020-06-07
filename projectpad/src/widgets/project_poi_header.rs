use super::project_items_list::ProjectItem;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
}

pub struct Model {
    project_item: Option<ProjectItem>,
}

#[widget]
impl Widget for ProjectPoiHeader {
    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model { project_item: None }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => self.model.project_item = pi,
        }
    }

    fn project_item_desc(pi: &ProjectItem) -> &str {
        match pi {
            ProjectItem::Server(srv) => &srv.desc,
            ProjectItem::ServerLink(srv) => &srv.desc,
            ProjectItem::ProjectNote(note) => &note.title,
            ProjectItem::ProjectPointOfInterest(poi) => &poi.desc,
        }
    }

    view! {
        gtk::Box {
            hexpand: true,
            gtk::Label {
                margin_top: 8,
                margin_bottom: 8,
                hexpand: true,
                markup: self.model.project_item
                    .as_ref()
                    .map(Self::project_item_desc)
                    .map(|v| format!("<b>{}</b>", v))
                    .as_deref().unwrap_or("")
            }
        }
    }
}
