use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    GotItemName(String),
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
            item_name: None,
            _item_name_channel: item_name_channel,
            item_name_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotItemName(name) => self.item_name.set_label(&name),
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
