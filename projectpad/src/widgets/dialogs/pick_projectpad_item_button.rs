use super::super::search_engine::PROJECT_FILTER_PREFIX;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::search_view;
use crate::widgets::search_view::Msg as SearchViewMsg;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerDatabase};
use relm::Widget;
use relm_derive::{widget, Msg};
use search_view::ProjectPadItem;
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    SetProjectNameAndId(Option<(String, i32)>),
    GotItem((ProjectPadItem, String, i32)),
    PickItemClick,
    RemoveItem,
    ItemSelected((ProjectPadItem, i32, String)),
}

#[derive(Copy, Clone)]
pub enum ItemType {
    ServerDatabase,
    Server,
}

pub struct Model {
    relm: relm::Relm<PickProjectpadItemButton>,
    db_sender: mpsc::Sender<SqlFunc>,
    item_type: ItemType,
    item_id: Option<i32>,
    item: Option<ProjectPadItem>,
    item_project_id: Option<i32>,
    item_name: Option<String>,
    // it's 'initial' because it's in the searchbox and the
    // user can always remove it
    initial_project_name_id: Option<(String, i32)>,

    _item_channel: relm::Channel<(ProjectPadItem, String, i32)>,
    item_sender: relm::Sender<(ProjectPadItem, String, i32)>,

    search_view_component: Option<relm::Component<search_view::SearchView>>,
}

pub struct PickProjectpadItemParams {
    pub db_sender: mpsc::Sender<SqlFunc>,
    pub item_type: ItemType,
    pub item_id: Option<i32>,
    pub project_name_id: Option<(String, i32)>,
}

#[widget]
impl Widget for PickProjectpadItemButton {
    fn init_view(&mut self) {
        self.widgets.btn_box.get_style_context().add_class("linked");
        self.update_display();
        if let Some(id) = self.model.item_id {
            self.fetch_item_name(id);
        }
    }

    fn update_display(&self) {
        // TODO show/hide the remove button
        // self.pick_item_stack
        //     .set_visible_child_name(if self.model.item_id.is_some() {
        //         "item"
        //     } else {
        //         "no_item"
        //     });
    }

    fn fetch_item_name(&self, item_id: i32) {
        let item_type = self.model.item_type;
        let s = self.model.item_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| match item_type {
                ItemType::ServerDatabase => {
                    use projectpadsql::schema::server::dsl as srv;
                    use projectpadsql::schema::server_database::dsl as db;
                    let (server_db, server) = db::server_database
                        .inner_join(srv::server)
                        .filter(db::id.eq(item_id))
                        .first::<(ServerDatabase, Server)>(sql_conn)
                        .unwrap();
                    let name = server_db.desc.clone();
                    s.send((
                        ProjectPadItem::ServerDatabase(server_db),
                        name,
                        server.project_id,
                    ))
                    .unwrap();
                }
                ItemType::Server => {
                    use projectpadsql::schema::server::dsl as srv;
                    let server: Server = srv::server.find(item_id).first(sql_conn).unwrap();
                    let project_id = server.project_id;
                    let name = server.desc.clone();
                    s.send((ProjectPadItem::Server(server), name, project_id))
                        .unwrap();
                }
            }))
            .unwrap();
    }

    fn model(relm: &relm::Relm<Self>, params: PickProjectpadItemParams) -> Model {
        let stream = relm.stream().clone();
        let (item_channel, item_sender) =
            relm::Channel::new(move |item: (ProjectPadItem, String, i32)| {
                stream.emit(Msg::GotItem(item));
            });
        Model {
            relm: relm.clone(),
            db_sender: params.db_sender,
            item_type: params.item_type,
            item_id: params.item_id,
            initial_project_name_id: params.project_name_id,
            search_view_component: None,
            item_name: None,
            item: None,
            item_project_id: None,
            _item_channel: item_channel,
            item_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::SetProjectNameAndId(name_id) => {
                self.model.initial_project_name_id = name_id;
            }
            Msg::GotItem((projectpad_item, name, project_id)) => {
                self.model.item_name = Some(name);
                self.model.item = Some(projectpad_item);
                self.model.item_project_id = Some(project_id);
            }
            Msg::PickItemClick => {
                self.open_pickitem_modal();
            }
            Msg::RemoveItem => {
                self.model.item = None;
                self.model.item_id = None;
                self.model.item_name = None;
            }
            Msg::ItemSelected((item, id, name)) => {
                self.model.item = Some(item);
                self.model.item_id = Some(id);
                self.model.item_name = Some(name);
            }
        }
    }

    fn open_pickitem_modal(&mut self) {
        let dialog = standard_dialogs::modal_dialog(
            self.widgets.pick_item_btn.clone().upcast::<gtk::Widget>(),
            800,
            400,
            "Pick item".to_string(),
        );
        let save_btn = dialog
            .add_button("Save", gtk::ResponseType::Ok)
            .downcast::<gtk::Button>()
            .expect("error reading the dialog save button");
        save_btn.get_style_context().add_class("suggested-action");
        let dialog_contents = relm::init::<search_view::SearchView>((
            self.model.db_sender.clone(),
            Some("".to_string()),
            match self.model.item_type {
                ItemType::ServerDatabase => search_view::SearchItemsType::ServerDbsOnly,
                ItemType::Server => search_view::SearchItemsType::ServersOnly,
            },
            search_view::OperationMode::SelectItem,
            Some(save_btn),
            None,
        ))
        .expect("error initializing the search modal");
        self.model.search_view_component = Some(dialog_contents);
        let comp = self.model.search_view_component.as_ref().unwrap();
        // do we filter by project in the search modal?
        let project_name = match (
            self.model.item_project_id,
            self.model.initial_project_name_id.as_ref(),
        ) {
            // we are given a project which matches the project
            // of the item to display => use that project
            (Some(item_prj_id), Some((prj_name, prj_id))) if item_prj_id == *prj_id => {
                Some(prj_name)
            }
            // there is no item to display, we are given a project
            // => use that project
            (None, Some((prj_name, _))) => Some(prj_name),
            // in all the other cases, don't filter by project
            _ => None,
        };

        // format the search text based on item name and potentially project name
        let search_text = match (&self.model.item_name, &project_name) {
            (Some(name), Some(project)) if project.contains(' ') => {
                format!("{} {}\"{}\"", name, PROJECT_FILTER_PREFIX, project)
            }
            (Some(name), Some(project)) => format!("{} {}{}", name, PROJECT_FILTER_PREFIX, project),
            (None, Some(project)) if project.contains(' ') => {
                format!("{}\"{}\"", PROJECT_FILTER_PREFIX, project)
            }
            (None, Some(project)) => format!("{}{}", PROJECT_FILTER_PREFIX, project),
            (Some(name), None) => name.clone(),
            (None, None) => "".to_string(),
        };
        comp.stream()
            .emit(search_view::Msg::FilterChanged(Some(search_text.clone())));
        comp.stream()
            .emit(search_view::Msg::SelectItem(self.model.item.clone()));
        standard_dialogs::prepare_custom_dialog_component_ref(&dialog, comp);

        let search_entry = gtk::SearchEntryBuilder::new().text(&search_text).build();
        let comp2 = comp.stream().clone();
        search_entry.connect_changed(move |se| {
            comp2.stream().emit(search_view::Msg::FilterChanged(Some(
                se.get_text().to_string(),
            )));
        });
        dialog.get_header_bar().unwrap().pack_end(&search_entry);
        search_entry.show();

        relm::connect!(comp@SearchViewMsg::SelectedItem(ref p), self.model.relm, Msg::ItemSelected(p.clone()));

        let comp3 = comp.stream().clone();
        dialog.connect_response(move |d, r| {
            if r == gtk::ResponseType::Ok {
                comp3.stream().emit(search_view::Msg::RequestSelectedItem);
                d.close();
            } else {
                d.close();
            }
        });

        dialog.show();
    }

    view! {
        #[name="btn_box"]
        gtk::Box {
            orientation: gtk::Orientation::Horizontal,
            #[name="pick_item_btn"]
            gtk::Button {
                hexpand: true,
                button_press_event(_, _) => (Msg::PickItemClick, Inhibit(false)),
                label: self.model.item_name.as_deref().unwrap_or("Pick item")
            },
            gtk::Button {
                always_show_image: true,
                image: Some(&gtk::Image::from_icon_name(
                    Some("edit-delete-symbolic"), gtk::IconSize::Menu)),
                button_press_event(_, _) => (Msg::RemoveItem, Inhibit(false)),
                visible: self.model.item.is_some()
            },
        },
    }
}
