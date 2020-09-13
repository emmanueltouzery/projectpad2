use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::search_view;
use crate::widgets::search_view::Msg as SearchViewMsg;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerDatabase};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    GotItem((search_view::ProjectPadItem, String)),
    PickItemClick,
    RemoveItem,
    ItemSelected((search_view::ProjectPadItem, i32, String)),
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
    item: Option<search_view::ProjectPadItem>,
    item_name: Option<String>,

    _item_channel: relm::Channel<(search_view::ProjectPadItem, String)>,
    item_sender: relm::Sender<(search_view::ProjectPadItem, String)>,

    search_view_component: Option<relm::Component<search_view::SearchView>>,
}

#[widget]
impl Widget for PickProjectpadItemButton {
    fn init_view(&mut self) {
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
                    use projectpadsql::schema::server_database::dsl as db;
                    let server_db: ServerDatabase =
                        db::server_database.find(item_id).first(sql_conn).unwrap();
                    let name = server_db.desc.clone();
                    s.send((search_view::ProjectPadItem::ServerDatabase(server_db), name))
                        .unwrap();
                }
                ItemType::Server => {
                    use projectpadsql::schema::server::dsl as srv;
                    let server: Server = srv::server.find(item_id).first(sql_conn).unwrap();
                    let name = server.desc.clone();
                    s.send((search_view::ProjectPadItem::Server(server), name))
                        .unwrap();
                }
            }))
            .unwrap();
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, ItemType, Option<i32>),
    ) -> Model {
        let stream = relm.stream().clone();
        let (item_channel, item_sender) =
            relm::Channel::new(move |item: (search_view::ProjectPadItem, String)| {
                stream.emit(Msg::GotItem(item));
            });
        Model {
            relm: relm.clone(),
            db_sender: params.0,
            item_type: params.1,
            item_id: params.2,
            search_view_component: None,
            item_name: None,
            item: None,
            _item_channel: item_channel,
            item_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotItem((projectpad_item, name)) => {
                self.model.item_name = Some(name);
                self.model.item = Some(projectpad_item);
            }
            Msg::PickItemClick => {
                let dialog = standard_dialogs::modal_dialog(
                    self.pick_item_btn.clone().upcast::<gtk::Widget>(),
                    800,
                    400,
                    "Pick item".to_string(),
                );
                let save = dialog
                    .add_button("Save", gtk::ResponseType::Ok)
                    .downcast::<gtk::Button>()
                    .expect("error reading the dialog save button");
                save.get_style_context().add_class("suggested-action");
                let save_btn = save.clone();
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
                let search_text = self.model.item_name.clone().unwrap_or("".to_string());
                comp.stream()
                    .emit(search_view::Msg::FilterChanged(Some(search_text.clone())));
                comp.stream()
                    .emit(search_view::Msg::SelectItem(self.model.item.clone()));
                standard_dialogs::prepare_custom_dialog_component_ref(&dialog, comp);

                let search_entry = gtk::SearchEntryBuilder::new().text(&search_text).build();
                let comp2 = comp.clone();
                search_entry.connect_changed(move |se| {
                    comp2.stream().emit(search_view::Msg::FilterChanged(Some(
                        se.get_text().to_string(),
                    )));
                });
                dialog.get_header_bar().unwrap().pack_end(&search_entry);
                search_entry.show();

                relm::connect!(comp@SearchViewMsg::SelectedItem(ref p), self.model.relm, Msg::ItemSelected(p.clone()));

                let comp3 = comp.clone();
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

    view! {
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
