use super::dialogs::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::dialogs::server_poi_add_edit_dlg::ServerPoiAddEditDialog;
use super::dialogs::standard_dialogs::*;
use super::project_poi_header::{populate_grid, GridItem, LabelText};
use super::server_poi_contents::ServerItem;
use crate::icons::*;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::{
    InterestType, ServerDatabase, ServerExtraUserAccount, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    CopyClicked(String),
    ViewNote(ServerNote),
    EditPoi(ServerPointOfInterest),
    ServerPoiUpdated(ServerPointOfInterest),
}

pub struct Model {
    relm: relm::Relm<ServerItemListItem>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_poi_add_edit_dialog: Option<relm::Component<ServerPoiAddEditDialog>>,
    server_item: ServerItem,
    header_popover: gtk::Popover,
    title: (String, Icon),
}

pub fn get_server_item_grid_items(server_item: &ServerItem) -> Vec<GridItem> {
    match server_item {
        ServerItem::Website(ref srv_w) => get_website_grid_items(srv_w),
        ServerItem::PointOfInterest(ref srv_poi) => get_poi_grid_items(srv_poi),
        ServerItem::Note(ref srv_n) => get_note_grid_items(srv_n),
        ServerItem::ExtraUserAccount(ref srv_u) => get_user_grid_items(srv_u),
        ServerItem::Database(ref srv_d) => get_db_grid_items(srv_d),
    }
}

fn get_website_grid_items(website: &ServerWebsite) -> Vec<GridItem> {
    vec![
        GridItem::new(
            "Address",
            Some(Icon::HTTP),
            LabelText::Markup(format!(
                "<a href=\"{}\">{}</a>",
                glib::markup_escape_text(&website.url),
                glib::markup_escape_text(&website.url)
            )),
            website.url.clone(),
        ),
        GridItem::new(
            "Username",
            None,
            LabelText::PlainText(website.username.clone()),
            website.username.clone(),
        ),
        GridItem::new(
            "Password",
            None,
            LabelText::PlainText(if website.username.is_empty() {
                "".to_string()
            } else {
                "●●●●●".to_string()
            }),
            website.password.clone(),
        ),
    ]
}

fn get_poi_grid_items(poi: &ServerPointOfInterest) -> Vec<GridItem> {
    vec![
        // TODO lots of clones...
        GridItem::new(
            "Path",
            None,
            LabelText::PlainText(poi.path.clone()),
            poi.path.clone(),
        ),
        GridItem::new(
            "Text",
            None,
            LabelText::PlainText(poi.text.clone()),
            poi.text.clone(),
        ),
    ]
}

fn get_note_grid_items(_note: &ServerNote) -> Vec<GridItem> {
    vec![]
}

fn get_user_grid_items(user: &ServerExtraUserAccount) -> Vec<GridItem> {
    vec![
        GridItem::new(
            "Username",
            None,
            LabelText::PlainText(user.username.clone()),
            user.username.clone(),
        ),
        GridItem::new(
            "Password",
            None,
            LabelText::PlainText(if user.password.is_empty() {
                "".to_string()
            } else {
                "●●●●●".to_string()
            }),
            user.password.clone(),
        ),
    ]
}

fn get_db_grid_items(db: &ServerDatabase) -> Vec<GridItem> {
    vec![
        GridItem::new(
            "Name",
            None,
            LabelText::PlainText(db.name.clone()),
            db.name.clone(),
        ),
        GridItem::new(
            "Text",
            None,
            LabelText::PlainText(db.text.clone()),
            db.text.clone(),
        ),
        GridItem::new(
            "Username",
            None,
            LabelText::PlainText(db.username.clone()),
            db.username.clone(),
        ),
        GridItem::new(
            "Text",
            None,
            LabelText::PlainText(if db.password.is_empty() {
                "".to_string()
            } else {
                "●●●●●".to_string()
            }),
            db.password.clone(),
        ),
    ]
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActionTypes {
    Copy,
    Edit,
    View,
}

#[widget]
impl Widget for ServerItemListItem {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
        self.title
            .get_style_context()
            .add_class("items_frame_title");

        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
        let fields = get_server_item_grid_items(&self.model.server_item);
        let extra_btns = match self.model.server_item {
            ServerItem::Note(_) => vec![(
                gtk::ModelButtonBuilder::new().label("View").build(),
                ActionTypes::View,
            )],
            ServerItem::PointOfInterest(_) => vec![(
                gtk::ModelButtonBuilder::new().label("Edit").build(),
                ActionTypes::Edit,
            )],
            _ => vec![],
        };
        let server_item = self.model.server_item.clone();
        populate_grid(
            self.items_grid.clone(),
            self.model.header_popover.clone(),
            &fields,
            ActionTypes::Copy,
            &extra_btns,
            &|btn: &gtk::ModelButton, action_type: ActionTypes, str_val: String| match action_type {
                ActionTypes::View => match server_item.clone() {
                    ServerItem::Note(n) => relm::connect!(
                        self.model.relm,
                        &btn,
                        connect_clicked(_),
                        Msg::ViewNote(n.clone())
                    ),
                    _ => panic!(),
                },
                ActionTypes::Edit => match server_item.clone() {
                    ServerItem::PointOfInterest(poi) => relm::connect!(
                        self.model.relm,
                        &btn,
                        connect_clicked(_),
                        Msg::EditPoi(poi.clone())
                    ),
                    _ => panic!(),
                },
                _ => {
                    relm::connect!(
                        self.model.relm,
                        &btn,
                        connect_clicked(_),
                        Msg::CopyClicked(str_val.clone())
                    );
                }
            },
        );
        // TODO i don't like that note is special-cased here.
        if let ServerItem::Note(ref srv_n) = self.model.server_item {
            let truncated_contents = srv_n
                .contents
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join("\n");
            self.items_grid.attach(
                &gtk::LabelBuilder::new()
                    .hexpand(true)
                    .single_line_mode(true)
                    .use_markup(true)
                    .ellipsize(pango::EllipsizeMode::End)
                    .xalign(0.0)
                    .label(&truncated_contents)
                    .build(),
                0,
                fields.len() as i32,
                2,
                1,
            );
            self.items_grid.show_all();
        }
    }

    fn get_title(server_item: &ServerItem) -> (String, Icon) {
        match server_item {
            ServerItem::Website(ref srv_w) => (srv_w.desc.clone(), Icon::HTTP),
            ServerItem::PointOfInterest(ref srv_poi) => {
                (srv_poi.desc.clone(), Self::server_poi_get_icon(srv_poi))
            }
            ServerItem::Note(ref srv_n) => (srv_n.title.clone(), Icon::NOTE),
            ServerItem::ExtraUserAccount(ref srv_u) => (srv_u.desc.clone(), Icon::USER),
            ServerItem::Database(ref srv_d) => (srv_d.desc.clone(), Icon::DATABASE),
        }
    }

    fn server_poi_get_icon(server_poi: &ServerPointOfInterest) -> Icon {
        match server_poi.interest_type {
            InterestType::PoiLogFile => Icon::LOG_FILE,
            InterestType::PoiConfigFile => Icon::CONFIG_FILE,
            InterestType::PoiApplication => Icon::FOLDER_PLUS,
            InterestType::PoiCommandToRun => Icon::COG,
            InterestType::PoiBackupArchive => Icon::ARCHIVE,
            InterestType::PoiCommandTerminal => Icon::TERMINAL,
        }
    }

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, ServerItem)) -> Model {
        let (db_sender, server_item) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            server_poi_add_edit_dialog: None,
            title: Self::get_title(&server_item),
            server_item,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::CopyClicked(val) => {
                if let Some(clip) = gtk::Clipboard::get_default(&self.items_grid.get_display()) {
                    clip.set_text(&val);
                }
            }
            // meant for my parent
            Msg::ViewNote(_) => {}
            Msg::EditPoi(poi) => {
                let (dialog, component) = Self::prepare_add_edit_server_poi_dialog(
                    self.items_frame.clone().upcast::<gtk::Widget>(),
                    self.model.db_sender.clone(),
                    poi.server_id,
                    Some(poi),
                );
                relm::connect!(
                    component@MsgServerPoiAddEditDialog::ServerPoiUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerPoiUpdated(srv.clone())
                );
                self.model.server_poi_add_edit_dialog = Some(component);
                dialog.show_all();
            }
            Msg::ServerPoiUpdated(_) => {}
        }
    }

    pub fn prepare_add_edit_server_poi_dialog(
        widget_for_window: gtk::Widget,
        db_sender: mpsc::Sender<SqlFunc>,
        server_id: i32,
        server_poi: Option<ServerPointOfInterest>,
    ) -> (gtk::Dialog, relm::Component<ServerPoiAddEditDialog>) {
        let title = if server_poi.is_some() {
            "Edit server POI"
        } else {
            "Add server POI"
        };
        let dialog_contents =
            relm::init::<ServerPoiAddEditDialog>((db_sender, server_id, server_poi))
                .expect("error initializing the server poi add edit modal");
        let d_c = dialog_contents.clone();
        prepare_custom_dialog(
            widget_for_window,
            600,
            200,
            title,
            dialog_contents,
            move || d_c.emit(MsgServerPoiAddEditDialog::OkPressed),
        )
    }

    view! {
        #[name="items_frame"]
        gtk::Frame {
            margin_start: 20,
            margin_end: 20,
            margin_top: 20,
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                #[name="title"]
                gtk::Box {
                    orientation: gtk::Orientation::Horizontal,
                    gtk::Image {
                        property_icon_name: Some(self.model.title.1.name()),
                        // https://github.com/gtk-rs/gtk/issues/837
                        property_icon_size: 1, // gtk::IconSize::Menu,
                    },
                    gtk::Label {
                        margin_start: 5,
                        text: &self.model.title.0,
                        ellipsize: pango::EllipsizeMode::End,
                    },
                    #[name="header_actions_btn"]
                    gtk::MenuButton {
                        child: {
                            pack_type: gtk::PackType::End,
                        },
                        always_show_image: true,
                        image: Some(&gtk::Image::from_icon_name(
                            Some(Icon::COG.name()), gtk::IconSize::Menu)),
                        halign: gtk::Align::End,
                        valign: gtk::Align::Center,
                    },
                },
                #[name="items_grid"]
                gtk::Grid {
                    margin_start: 10,
                    margin_end: 10,
                    margin_top: 10,
                    margin_bottom: 5,
                    row_spacing: 5,
                    column_spacing: 10,
                }
            }
        }
    }
}
