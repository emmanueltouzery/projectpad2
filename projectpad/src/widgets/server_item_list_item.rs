use super::project_poi_header::{populate_grid, GridItem};
use super::server_poi_contents::ServerItem;
use crate::icons::*;
use gtk::prelude::*;
use projectpadsql::models::{
    InterestType, ServerDatabase, ServerExtraUserAccount, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    CopyClicked(String),
}

pub struct Model {
    relm: relm::Relm<ServerItemListItem>,
    server_item: ServerItem,
    header_popover: gtk::Popover,
    title: (String, Icon),
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
        let fields = &self.get_grid_items();
        populate_grid(
            self.items_grid.clone(),
            self.model.header_popover.clone(),
            fields,
            &|btn: &gtk::ModelButton, str_val: String| {
                relm::connect!(
                    self.model.relm,
                    &btn,
                    connect_clicked(_),
                    Msg::CopyClicked(str_val.clone())
                );
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

    fn get_grid_items(&self) -> Vec<GridItem> {
        match self.model.server_item {
            ServerItem::Website(ref srv_w) => Self::get_website_grid_items(srv_w),
            ServerItem::PointOfInterest(ref srv_poi) => Self::get_poi_grid_items(srv_poi),
            ServerItem::Note(ref srv_n) => Self::get_note_grid_items(srv_n),
            ServerItem::ExtraUserAccount(ref srv_u) => Self::get_user_grid_items(srv_u),
            ServerItem::Database(ref srv_d) => Self::get_db_grid_items(srv_d),
        }
    }

    fn get_website_grid_items(website: &ServerWebsite) -> Vec<GridItem> {
        vec![
            GridItem::new(
                "Address",
                Some(Icon::HTTP),
                format!("<a href=\"{}\">{}</a>", website.url, website.url),
                website.url.clone(),
            ),
            GridItem::new(
                "Username",
                None,
                website.username.clone(),
                website.username.clone(),
            ),
            GridItem::new(
                "Password",
                None,
                if website.username.is_empty() {
                    "".to_string()
                } else {
                    "●●●●●".to_string()
                },
                website.password.clone(),
            ),
        ]
    }

    fn get_poi_grid_items(poi: &ServerPointOfInterest) -> Vec<GridItem> {
        vec![
            // TODO lots of clones...
            GridItem::new("Path", None, poi.path.clone(), poi.path.clone()),
            GridItem::new("Text", None, poi.text.clone(), poi.text.clone()),
        ]
    }

    fn get_note_grid_items(note: &ServerNote) -> Vec<GridItem> {
        vec![]
    }

    fn get_user_grid_items(user: &ServerExtraUserAccount) -> Vec<GridItem> {
        vec![
            GridItem::new(
                "Username",
                None,
                user.username.clone(),
                user.username.clone(),
            ),
            GridItem::new(
                "Password",
                None,
                if user.password.is_empty() {
                    "".to_string()
                } else {
                    "●●●●●".to_string()
                },
                user.password.clone(),
            ),
        ]
    }

    fn get_db_grid_items(db: &ServerDatabase) -> Vec<GridItem> {
        vec![
            GridItem::new("Name", None, db.name.clone(), db.name.clone()),
            GridItem::new("Text", None, db.text.clone(), db.text.clone()),
            GridItem::new("Username", None, db.username.clone(), db.username.clone()),
            GridItem::new(
                "Text",
                None,
                if db.password.is_empty() {
                    "".to_string()
                } else {
                    "●●●●●".to_string()
                },
                db.password.clone(),
            ),
        ]
    }

    fn model(relm: &relm::Relm<Self>, server_item: ServerItem) -> Model {
        Model {
            relm: relm.clone(),
            title: Self::get_title(&server_item),
            server_item,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::CopyClicked(val) => {
                if let Some(clip) = self
                    .items_grid
                    .get_display()
                    .as_ref()
                    .and_then(gtk::Clipboard::get_default)
                {
                    clip.set_text(&val);
                }
            }
        }
    }

    fn format_link(str: &str) -> String {
        format!("<a href='{}'>{}</a>", str, str)
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
                        image: Some(&gtk::Image::new_from_icon_name(
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