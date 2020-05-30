use super::project_poi_item_list_item::ProjectPoiItemListItem;
use super::win::{ProjectPoi, ProjectPoiItem};
use gtk::prelude::*;
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    cur_project_item: Option<ProjectPoi>,
    project_poi_items: Vec<ProjectPoiItem>,
}

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {
        self.update_contents_list();
    }

    fn model(relm: &relm::Relm<Self>, params: (Option<ProjectPoi>, Vec<ProjectPoiItem>)) -> Model {
        Model {
            cur_project_item: params.0,
            project_poi_items: params.1,
        }
    }

    fn update(&mut self, _event: Msg) {}

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        for project_poi_item in &self.model.project_poi_items {
            let child = self
                .contents_list
                .add_widget::<ProjectPoiItemListItem>(project_poi_item.clone());
        }
    }

    view! {
        #[name="contents_list"]
        gtk::ListBox {
            selection_mode: gtk::SelectionMode::None,
        }
    }
}
