use super::dialogs::dialog_helpers;
use super::dialogs::server_database_add_edit_dlg::Msg as MsgServerDatabaseAddEditDialog;
use super::dialogs::server_extra_user_add_edit_dlg::Msg as MsgServerExtraUserAddEditDialog;
use super::dialogs::server_poi_add_edit_dlg::server_poi_get_text_label;
use super::dialogs::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::dialogs::server_website_add_edit_dlg::Msg as MsgServerWebsiteAddEditDialog;
use super::dialogs::standard_dialogs;
use super::dialogs::AddEditDialogComponent;
use super::project_poi_header::{populate_grid, GridItem, LabelText};
use super::server_poi_contents::ServerItem;
use crate::icons::*;
use crate::sql_thread::SqlFunc;
use diesel::helper_types::Find;
use diesel::prelude::*;
use diesel::query_builder::IntoUpdateTarget;
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::query_dsl::methods::FindDsl;
use diesel::sqlite::SqliteConnection;
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
    EditDb(ServerDatabase),
    EditUser(ServerExtraUserAccount),
    EditWebsite(ServerWebsite),
    ServerItemUpdated(ServerItem),
    AskDeleteServerPoi(ServerPointOfInterest),
    DeleteServerPoi(ServerPointOfInterest),
    AskDeleteDb(ServerDatabase),
    DeleteServerDb(ServerDatabase),
    AskDeleteServerExtraUser(ServerExtraUserAccount),
    DeleteServerExtraUser(ServerExtraUserAccount),
    AskDeleteServerWebsite(ServerWebsite),
    DeleteServerWebsite(ServerWebsite),
    ServerItemDeleted(ServerItem),
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<ServerItem, (&'static str, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ServerItemListItem>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_add_edit_dialog: Option<AddEditDialogComponent>,
    server_item: ServerItem,
    header_popover: gtk::Popover,
    title: (String, Icon),
    _server_item_deleted_channel: relm::Channel<DeleteResult>,
    server_item_deleted_sender: relm::Sender<DeleteResult>,
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
            server_poi_get_text_label(poi.interest_type),
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
        self.load_server_item();
    }

    fn load_server_item(&self) {
        let fields = get_server_item_grid_items(&self.model.server_item);
        // TODO drop the clone
        let extra_btns = match self.model.server_item.clone() {
            ServerItem::Note(n) => {
                let view_btn = gtk::ModelButtonBuilder::new().label("View").build();
                relm::connect!(
                    self.model.relm,
                    &view_btn,
                    connect_clicked(_),
                    Msg::ViewNote(n.clone())
                );
                vec![view_btn]
            }
            ServerItem::PointOfInterest(poi) => {
                let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
                let p = poi.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &edit_btn,
                    connect_clicked(_),
                    Msg::EditPoi(p.clone())
                );
                let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
                let p2 = poi.clone(); // TODO too many clones
                                      // TODO skip the ask step
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerPoi(p2.clone())
                );
                vec![edit_btn, delete_btn]
            }
            ServerItem::Database(db) => {
                let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
                let d = db.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &edit_btn,
                    connect_clicked(_),
                    Msg::EditDb(d.clone())
                );
                let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
                let d2 = db.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteDb(d2.clone())
                );
                vec![edit_btn, delete_btn]
            }
            ServerItem::ExtraUserAccount(usr) => {
                let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
                let u = usr.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &edit_btn,
                    connect_clicked(_),
                    Msg::EditUser(u.clone())
                );
                let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
                let u2 = usr.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerExtraUser(u2.clone())
                );
                vec![edit_btn, delete_btn]
            }
            ServerItem::Website(www) => {
                let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
                let w = www.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &edit_btn,
                    connect_clicked(_),
                    Msg::EditWebsite(w.clone())
                );
                let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
                let w2 = www.clone(); // TODO too many clones
                                      // TODO skip the ask step
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerWebsite(w2.clone())
                );
                vec![edit_btn, delete_btn]
            }
            _ => vec![],
        };
        let server_item = self.model.server_item.clone();
        populate_grid(
            self.items_grid.clone(),
            self.model.header_popover.clone(),
            &fields,
            &extra_btns,
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

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, ServerItem)) -> Model {
        let (db_sender, server_item) = params;
        let stream = relm.stream().clone();
        let (_server_item_deleted_channel, server_item_deleted_sender) =
            relm::Channel::new(move |r: DeleteResult| match r {
                Ok(server_item) => {
                    stream.emit(Msg::ServerItemDeleted(server_item));
                }
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        Model {
            relm: relm.clone(),
            db_sender,
            server_add_edit_dialog: None,
            title: Self::get_title(&server_item),
            server_item,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
            _server_item_deleted_channel,
            server_item_deleted_sender,
        }
    }

    fn run_delete_action<Tbl>(&self, table: Tbl, server_item: ServerItem)
    where
        Tbl: FindDsl<i32> + Send + 'static + Copy,
        Find<Tbl, i32>: IntoUpdateTarget,
        dialog_helpers::DeleteFindStatement<Find<Tbl, i32>>: ExecuteDsl<SqliteConnection>,
    {
        let s = self.model.server_item_deleted_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(
                    dialog_helpers::delete_row(sql_conn, table, server_item.get_id())
                        .map(|_| server_item.clone()),
                )
                .unwrap();
            }))
            .unwrap();
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
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.items_frame.clone().upcast::<gtk::Widget>(),
                    (self.model.db_sender.clone(), poi.server_id, Some(poi)),
                    MsgServerPoiAddEditDialog::OkPressed,
                    "Server POI",
                );
                relm::connect!(
                    component@MsgServerPoiAddEditDialog::ServerPoiUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::PointOfInterest(srv.clone()))
                );
                self.model.server_add_edit_dialog = Some(AddEditDialogComponent::Poi(component));
                dialog.show();
            }
            Msg::EditDb(db) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.items_frame.clone().upcast::<gtk::Widget>(),
                    (self.model.db_sender.clone(), db.server_id, Some(db)),
                    MsgServerDatabaseAddEditDialog::OkPressed,
                    "Server Database",
                );
                relm::connect!(
                    component@MsgServerDatabaseAddEditDialog::ServerDbUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::Database(srv.clone()))
                );
                self.model.server_add_edit_dialog = Some(AddEditDialogComponent::Db(component));
                dialog.show();
            }
            Msg::EditWebsite(www) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.items_frame.clone().upcast::<gtk::Widget>(),
                    (self.model.db_sender.clone(), www.server_id, Some(www)),
                    MsgServerWebsiteAddEditDialog::OkPressed,
                    "Server Website",
                );
                relm::connect!(
                    component@MsgServerWebsiteAddEditDialog::ServerWwwUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::Website(srv.clone()))
                );
                self.model.server_add_edit_dialog =
                    Some(AddEditDialogComponent::Website(component));
                dialog.show();
            }
            Msg::EditUser(usr) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.items_frame.clone().upcast::<gtk::Widget>(),
                    (self.model.db_sender.clone(), usr.server_id, Some(usr)),
                    MsgServerExtraUserAddEditDialog::OkPressed,
                    "Server Extra User",
                );
                relm::connect!(
                    component@MsgServerExtraUserAddEditDialog::ServerUserUpdated(ref usr),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::ExtraUserAccount(usr.clone()))
                );
                self.model.server_add_edit_dialog = Some(AddEditDialogComponent::User(component));
                dialog.show();
            }
            Msg::ServerItemUpdated(server_item) => {
                self.model.server_item = server_item;
                self.model.title = Self::get_title(&self.model.server_item);
                self.load_server_item();
            }
            Msg::AskDeleteServerPoi(poi) => {
                let relm = self.model.relm.clone();
                standard_dialogs::confirm_deletion(
                            "Delete server POI",
                            &format!("Are you sure you want to delete the server POI {}? This action cannot be undone.", poi.desc),
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                    move || relm.stream().emit(Msg::DeleteServerPoi(poi.clone())));
            }
            Msg::DeleteServerPoi(poi) => {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                self.run_delete_action(
                    srv_poi::server_point_of_interest,
                    ServerItem::PointOfInterest(poi),
                );
            }
            Msg::AskDeleteDb(db) => {
                let relm = self.model.relm.clone();

                standard_dialogs::confirm_deletion(
                            "Delete server database",
                            &format!("Are you sure you want to delete the server database {}? This action cannot be undone.", db.desc),
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                    move || relm.stream().emit(Msg::DeleteServerDb(db.clone())));
            }
            Msg::DeleteServerDb(db) => {
                use projectpadsql::schema::server_database::dsl as srv_db;
                let s = self.model.server_item_deleted_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        use projectpadsql::schema::server_database::dsl as db;
                        use projectpadsql::schema::server_website::dsl as srvw;
                        let dependent_websites = srvw::server_website
                            .inner_join(db::server_database)
                            .filter(db::id.eq(db.id))
                            .load::<(ServerWebsite, ServerDatabase)>(sql_conn)
                            .unwrap();
                        if !dependent_websites.is_empty() {
                            s.send(Err((
                                "Cannot delete database",
                                Some(format!(
                                    "this database is used by websites: {}",
                                    itertools::join(
                                        dependent_websites.iter().map(|(w, _)| &w.desc),
                                        ", "
                                    )
                                )),
                            )))
                        } else {
                            s.send(
                                dialog_helpers::delete_row(
                                    sql_conn,
                                    srv_db::server_database,
                                    db.id,
                                )
                                .map(|_| ServerItem::Database(db.clone())),
                            )
                        }
                        .unwrap();
                    }))
                    .unwrap();
            }
            Msg::AskDeleteServerExtraUser(user) => {
                let relm = self.model.relm.clone();
                standard_dialogs::confirm_deletion(
                            "Delete server extra user",
                            &format!("Are you sure you want to delete the server extra user {}? This action cannot be undone.", user.desc),
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                    move || relm.stream().emit(Msg::DeleteServerExtraUser(user.clone())));
            }
            Msg::DeleteServerExtraUser(user) => {
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                self.run_delete_action(
                    srv_usr::server_extra_user_account,
                    ServerItem::ExtraUserAccount(user),
                );
            }
            Msg::AskDeleteServerWebsite(user) => {
                let relm = self.model.relm.clone();
                standard_dialogs::confirm_deletion(
                            "Delete server website",
                            &format!("Are you sure you want to delete the server website {}? This action cannot be undone.", user.desc),
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                    move || relm.stream().emit(Msg::DeleteServerWebsite(user.clone())));
            }
            Msg::DeleteServerWebsite(website) => {
                use projectpadsql::schema::server_website::dsl as srv_www;
                self.run_delete_action(srv_www::server_website, ServerItem::Website(website));
            }
            // for my parent
            Msg::ServerItemDeleted(_) => {}
        }
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
