use super::dialogs::dialog_helpers;
use super::dialogs::server_database_add_edit_dlg::Msg as MsgServerDatabaseAddEditDialog;
use super::dialogs::server_extra_user_add_edit_dlg::Msg as MsgServerExtraUserAddEditDialog;
use super::dialogs::server_note_add_edit_dlg::Msg as MsgServerNoteAddEditDialog;
use super::dialogs::server_poi_add_edit_dlg::poi_get_text_label;
use super::dialogs::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::dialogs::server_website_add_edit_dlg::Msg as MsgServerWebsiteAddEditDialog;
use super::dialogs::standard_dialogs;
use super::dialogs::ServerAddEditDialogComponent;
use super::project_poi_header::{populate_grid, GridItem, LabelText};
use super::server_poi_contents::ServerItem;
use crate::icons::*;
use crate::notes;
use crate::sql_thread::SqlFunc;
use crate::sql_util;
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

#[derive(Msg, Clone)]
pub enum Msg {
    CopyClicked(String),
    ViewNote(ServerNote),
    EditNote(ServerNote),
    EditPoi(ServerPointOfInterest),
    EditDb(ServerDatabase),
    EditUser(ServerExtraUserAccount),
    EditWebsite(ServerWebsite),
    ServerItemUpdated(ServerItem),
    ServerWwwUpdated((ServerWebsite, Option<ServerDatabase>)),
    DeleteServerPoi(ServerPointOfInterest),
    DeleteServerDb(ServerDatabase),
    AskDeleteServerItem((String, &'static str, Box<Msg>)),
    DeleteServerExtraUser(ServerExtraUserAccount),
    DeleteServerWebsite(ServerWebsite),
    DeleteServerNote(ServerNote),
    ServerItemDeleted(ServerItem),
    RequestDisplayServerItem(ServerItem),
    ShowInfoBar(String),
    ItemGroupChanged,
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<ServerItem, (&'static str, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ServerItemListItem>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_add_edit_dialog: Option<(ServerAddEditDialogComponent, gtk::Dialog)>,
    server_item: ServerItem,
    database_for_item: Option<ServerDatabase>,
    websites_for_item: Vec<ServerWebsite>,
    header_popover: gtk::Popover,
    title: (String, Icon),
    _server_item_deleted_channel: relm::Channel<DeleteResult>,
    server_item_deleted_sender: relm::Sender<DeleteResult>,
}

pub fn get_server_item_grid_items(
    server_item: &ServerItem,
    database_for_item: &Option<ServerDatabase>,
) -> Vec<GridItem> {
    match server_item {
        ServerItem::Website(ref srv_w) => get_website_grid_items(srv_w, database_for_item),
        ServerItem::PointOfInterest(ref srv_poi) => get_poi_grid_items(srv_poi),
        ServerItem::Note(ref srv_n) => get_note_grid_items(srv_n),
        ServerItem::ExtraUserAccount(ref srv_u) => get_user_grid_items(srv_u),
        ServerItem::Database(ref srv_d) => get_db_grid_items(srv_d),
    }
}

fn get_website_grid_items(
    website: &ServerWebsite,
    database: &Option<ServerDatabase>,
) -> Vec<GridItem> {
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
            None,
        ),
        GridItem::new(
            "Username",
            None,
            LabelText::PlainText(website.username.clone()),
            website.username.clone(),
            None,
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
            None,
        ),
        GridItem::new(
            "Database",
            None,
            LabelText::PlainText(
                database
                    .as_ref()
                    .map(|db| db.desc.clone())
                    .unwrap_or_else(|| "".to_string()),
            ),
            website.username.clone(),
            None,
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
            None,
        ),
        GridItem::new(
            poi_get_text_label(poi.interest_type),
            None,
            LabelText::PlainText(poi.text.clone()),
            poi.text.clone(),
            None,
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
            None,
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
            None,
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
            None,
        ),
        GridItem::new(
            "Text",
            None,
            LabelText::PlainText(db.text.clone()),
            db.text.clone(),
            None,
        ),
        GridItem::new(
            "Username",
            None,
            LabelText::PlainText(db.username.clone()),
            db.username.clone(),
            None,
        ),
        GridItem::new(
            "Password",
            None,
            LabelText::PlainText(if db.password.is_empty() {
                "".to_string()
            } else {
                "●●●●●".to_string()
            }),
            db.password.clone(),
            None,
        ),
    ]
}

#[widget]
impl Widget for ServerItemListItem {
    fn init_view(&mut self) {
        self.widgets
            .items_frame
            .get_style_context()
            .add_class("items_frame");
        self.widgets
            .title
            .get_style_context()
            .add_class("items_frame_title");

        self.widgets
            .header_actions_btn
            .set_popover(Some(&self.model.header_popover));
        self.load_server_item();
    }

    fn load_server_item(&self) {
        let fields =
            get_server_item_grid_items(&self.model.server_item, &self.model.database_for_item);
        // TODO drop the clone
        let extra_btns = match self.model.server_item.clone() {
            ServerItem::Note(n) => {
                let view_btn = gtk::ModelButtonBuilder::new().label("View").build();
                let n0 = n.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &view_btn,
                    connect_clicked(_),
                    Msg::ViewNote(n0.clone())
                );
                let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
                let n1 = n.clone(); // TODO too many clones
                relm::connect!(
                    self.model.relm,
                    &edit_btn,
                    connect_clicked(_),
                    Msg::EditNote(n1.clone())
                );
                let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerItem((
                        n.title.clone(),
                        "note",
                        Box::new(Msg::DeleteServerNote(n.clone()))
                    ))
                );
                vec![view_btn, edit_btn, delete_btn]
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
                // TODO skip the ask step
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerItem((
                        poi.desc.clone(),
                        "server POI",
                        Box::new(Msg::DeleteServerPoi(poi.clone()))
                    ))
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
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerItem((
                        db.desc.clone(),
                        "server database",
                        Box::new(Msg::DeleteServerDb(db.clone()))
                    ))
                );
                let mut db_menu_items = vec![edit_btn, delete_btn];
                for www in &self.model.websites_for_item {
                    let go_to_www_btn = gtk::ModelButtonBuilder::new()
                        .label(&format!("Go to '{}'", &www.desc))
                        .build();
                    let w3 = www.clone(); // TODO too many clones
                    relm::connect!(
                        self.model.relm,
                        &go_to_www_btn,
                        connect_clicked(_),
                        Msg::RequestDisplayServerItem(ServerItem::Website(w3.clone()))
                    );
                    db_menu_items.push(go_to_www_btn);
                }
                db_menu_items
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
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerItem((
                        usr.desc.clone(),
                        "extra user",
                        Box::new(Msg::DeleteServerExtraUser(usr.clone()))
                    ))
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
                // TODO skip the ask step
                relm::connect!(
                    self.model.relm,
                    &delete_btn,
                    connect_clicked(_),
                    Msg::AskDeleteServerItem((
                        www.desc.clone(),
                        "server website",
                        Box::new(Msg::DeleteServerWebsite(www.clone()))
                    ))
                );
                let mut www_menu_items = vec![edit_btn, delete_btn];
                if let Some(db) = self.model.database_for_item.as_ref() {
                    let go_to_db_btn = gtk::ModelButtonBuilder::new()
                        .label("Go to database")
                        .build();
                    let d3 = db.clone(); // TODO too many clones
                    relm::connect!(
                        self.model.relm,
                        &go_to_db_btn,
                        connect_clicked(_),
                        Msg::RequestDisplayServerItem(ServerItem::Database(d3.clone()))
                    );
                    www_menu_items.push(go_to_db_btn);
                }
                www_menu_items
            }
        };
        populate_grid(
            self.widgets.items_grid.clone(),
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
            let truncated_contents = notes::note_markdown_to_quick_preview(&srv_n.contents)
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join("\n");
            self.widgets.items_grid.attach(
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
            self.widgets.items_grid.show_all();
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

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            ServerItem,
            Option<ServerDatabase>,
            Vec<ServerWebsite>,
        ),
    ) -> Model {
        let (db_sender, server_item, database_for_item, websites_for_item) = params;
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
            database_for_item,
            websites_for_item,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
            _server_item_deleted_channel,
            server_item_deleted_sender,
        }
    }

    fn run_delete_action<Tbl>(&self, table: Tbl, server_item: ServerItem)
    where
        Tbl: FindDsl<i32> + Send + 'static + Copy,
        Find<Tbl, i32>: IntoUpdateTarget,
        sql_util::DeleteFindStatement<Find<Tbl, i32>>: ExecuteDsl<SqliteConnection>,
    {
        let s = self.model.server_item_deleted_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(
                    sql_util::delete_row(sql_conn, table, server_item.get_id())
                        .map(|_| server_item.clone()),
                )
                .unwrap();
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::CopyClicked(val) => {
                if let Some(clip) =
                    gtk::Clipboard::get_default(&self.widgets.items_grid.get_display())
                {
                    clip.set_text(&val);
                }
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ShowInfoBar("Copied to the clipboard".to_string()));
            }
            // meant for my parent
            Msg::ViewNote(_) => {}
            Msg::EditNote(note) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        note.server_id,
                        Some(note),
                    ),
                    MsgServerNoteAddEditDialog::OkPressed,
                    "Server Note",
                );
                relm::connect!(
                    component@MsgServerNoteAddEditDialog::ServerNoteUpdated(ref note),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::Note(note.clone()))
                );
                self.model.server_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::Note(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            Msg::EditPoi(poi) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        poi.server_id,
                        Some(poi),
                    ),
                    MsgServerPoiAddEditDialog::OkPressed,
                    "Server POI",
                );
                relm::connect!(
                    component@MsgServerPoiAddEditDialog::ServerPoiUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::PointOfInterest(srv.clone()))
                );
                self.model.server_add_edit_dialog =
                    Some((ServerAddEditDialogComponent::Poi(component), dialog.clone()));
                dialog.show();
            }
            Msg::EditDb(db) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        db.server_id,
                        Some(db),
                    ),
                    MsgServerDatabaseAddEditDialog::OkPressed,
                    "Server Database",
                );
                relm::connect!(
                    component@MsgServerDatabaseAddEditDialog::ServerDbUpdated(ref srv),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::Database(srv.clone()))
                );
                self.model.server_add_edit_dialog =
                    Some((ServerAddEditDialogComponent::Db(component), dialog.clone()));
                dialog.show();
            }
            Msg::EditWebsite(www) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        www.server_id,
                        Some(www),
                    ),
                    MsgServerWebsiteAddEditDialog::OkPressed,
                    "Server Website",
                );
                relm::connect!(
                    component@MsgServerWebsiteAddEditDialog::ServerWwwUpdated(ref www_db),
                    self.model.relm,
                    Msg::ServerWwwUpdated(*www_db.clone())
                );
                self.model.server_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::Website(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            Msg::EditUser(usr) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        usr.server_id,
                        Some(usr),
                    ),
                    MsgServerExtraUserAddEditDialog::OkPressed,
                    "Server Extra User",
                );
                relm::connect!(
                    component@MsgServerExtraUserAddEditDialog::ServerUserUpdated(ref usr),
                    self.model.relm,
                    Msg::ServerItemUpdated(ServerItem::ExtraUserAccount(usr.clone()))
                );
                self.model.server_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::User(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            Msg::ServerWwwUpdated((www, db)) => {
                self.model.database_for_item = db;
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ServerItemUpdated(ServerItem::Website(www)));
            }
            Msg::ServerItemUpdated(server_item) => {
                self.model
                    .server_add_edit_dialog
                    .as_ref()
                    .unwrap()
                    .1
                    .close();
                self.model.server_add_edit_dialog = None;
                if self.model.server_item.group_name() != server_item.group_name() {
                    self.model.relm.stream().emit(Msg::ItemGroupChanged);
                }
                self.model.server_item = server_item;
                self.model.title = Self::get_title(&self.model.server_item);
                self.load_server_item();
            }
            Msg::DeleteServerPoi(poi) => {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                self.run_delete_action(
                    srv_poi::server_point_of_interest,
                    ServerItem::PointOfInterest(poi),
                );
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
                                sql_util::delete_row(sql_conn, srv_db::server_database, db.id)
                                    .map(|_| ServerItem::Database(db.clone())),
                            )
                        }
                        .unwrap();
                    }))
                    .unwrap();
            }
            Msg::AskDeleteServerItem((item_desc, message, delete_evt)) => {
                let relm = self.model.relm.clone();
                let evt = *delete_evt;
                standard_dialogs::confirm_deletion(
                    &format!("Delete server {}", message),
                    &format!("Are you sure you want to delete the server {} {}? This action cannot be undone.", message, &item_desc),
                    self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
                    move || relm.stream().emit(evt.clone()),
                );
            }
            Msg::DeleteServerExtraUser(user) => {
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                self.run_delete_action(
                    srv_usr::server_extra_user_account,
                    ServerItem::ExtraUserAccount(user),
                );
            }
            Msg::DeleteServerWebsite(website) => {
                use projectpadsql::schema::server_website::dsl as srv_www;
                self.run_delete_action(srv_www::server_website, ServerItem::Website(website));
            }
            Msg::DeleteServerNote(note) => {
                use projectpadsql::schema::server_note::dsl as srv_note;
                self.run_delete_action(srv_note::server_note, ServerItem::Note(note));
            }
            // for my parent
            Msg::ShowInfoBar(_) => {}
            Msg::ServerItemDeleted(_) => {}
            Msg::RequestDisplayServerItem(_) => {}
            Msg::ItemGroupChanged => {}
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
