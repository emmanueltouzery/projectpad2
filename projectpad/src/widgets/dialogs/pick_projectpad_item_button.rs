use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::search_view;
use diesel::prelude::*;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    GotItemName(String),
    PickItemClick,
}

pub enum ItemType {
    ServerDatabase,
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    item_type: ItemType,
    item_id: Option<i32>,

    _item_name_channel: relm::Channel<String>,
    item_name_sender: relm::Sender<String>,

    search_view_component: Option<relm::Component<search_view::SearchView>>,

    item_name: Option<String>,
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
        self.pick_item_stack
            .set_visible_child_name(if self.model.item_id.is_some() {
                "item"
            } else {
                "no_item"
            });
    }

    fn fetch_item_name(&self, item_id: i32) {
        let s = self.model.item_name_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as db;
                let server_db_name = db::server_database
                    .find(item_id)
                    .select(db::desc)
                    .first(sql_conn)
                    .unwrap();
                s.send(server_db_name).unwrap();
            }))
            .unwrap();
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, ItemType, Option<i32>),
    ) -> Model {
        let stream = relm.stream().clone();
        let (item_name_channel, item_name_sender) = relm::Channel::new(move |item_name: String| {
            stream.emit(Msg::GotItemName(item_name));
        });
        Model {
            db_sender: params.0,
            item_type: params.1,
            item_id: params.2,
            search_view_component: None,
            item_name: None,
            _item_name_channel: item_name_channel,
            item_name_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotItemName(name) => self.item_name.set_label(&name),
            Msg::PickItemClick => {
                let dialog_contents = relm::init::<search_view::SearchView>((
                    self.model.db_sender.clone(),
                    Some("".to_string()),
                ))
                .expect("error initializing the search modal");
                self.model.search_view_component = Some(dialog_contents);
                let comp = self.model.search_view_component.as_ref().unwrap();
                let (dialog, button) = standard_dialogs::prepare_custom_dialog_component_ref(
                    self.pick_item_stack.clone().upcast::<gtk::Widget>(),
                    800,
                    400,
                    "Pick item".to_string(),
                    comp,
                    move |_| standard_dialogs::DialogActionResult::CloseDialog,
                );
                button.hide();

                let search_entry = gtk::SearchEntryBuilder::new().build();
                let comp2 = comp.clone();
                search_entry.connect_changed(move |se| {
                    comp2.stream().emit(search_view::Msg::FilterChanged(Some(
                        se.get_text().to_string(),
                    )));
                });
                dialog.get_header_bar().unwrap().pack_end(&search_entry);
                search_entry.show();

                dialog.show();
            }
        }
    }

    view! {
        #[name="pick_item_stack"]
        gtk::Stack {
            // if there is no db, a picker...
            gtk::Button {
                child: {
                    name: Some("no_item")
                },
                button_press_event(_, _) => (Msg::PickItemClick, Inhibit(false)),
                label: "Pick item"
            },
            // if there is a db, a label with the db name,
            // and a button to remove the db
            gtk::Box {
                orientation: gtk::Orientation::Horizontal,
                child: {
                    name: Some("item")
                },
                #[name="item_name"]
                gtk::Label {
                    hexpand: true,
                },
                // gtk::Button {
                //     always_show_image: true,
                //     image: Some(&gtk::Image::from_icon_name(
                //         Some("document-save-symbolic"), gtk::IconSize::Menu)),
                //     button_press_event(_, _) => (Msg::SaveAuthFile, Inhibit(false)),
                // },
                // gtk::Button {
                //     always_show_image: true,
                //     image: Some(&gtk::Image::from_icon_name(
                //         Some("edit-delete-symbolic"), gtk::IconSize::Menu)),
                //     button_press_event(_, _) => (Msg::RemoveAuthFile, Inhibit(false)),
                // },
            },
        }
    }
}
