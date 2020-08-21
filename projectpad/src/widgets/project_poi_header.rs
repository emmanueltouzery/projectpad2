use super::dialogs::server_add_edit_dlg::Msg as MsgServerAddEditDialog;
use super::dialogs::server_add_edit_dlg::ServerAddEditDialog;
use super::dialogs::server_add_item_dlg;
use super::dialogs::server_add_item_dlg::ServerAddItemDialog;
use super::dialogs::server_poi_add_edit_dlg;
use super::dialogs::standard_dialogs;
use super::project_items_list::ProjectItem;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Project, Server, ServerAccessType, ServerDatabase, ServerWebsite};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActionTypes {
    Edit,
    Copy,
    Delete,
    AddItem,
}

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    HeaderActionClicked((ActionTypes, String)),
    ServerUpdated(Server),
    ServerDeleted(Server),
    DeleteCurrentServer(Server),
    ServerAddItemActionCompleted,
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<Server, (&'static str, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ProjectPoiHeader>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_item: Option<ProjectItem>,
    header_popover: gtk::Popover,
    title: gtk::Label,
    server_add_edit_dialog: Option<relm::Component<ServerAddEditDialog>>,
    server_add_item_dialog_component: Option<relm::Component<ServerAddItemDialog>>,
    server_add_item_dialog: Option<gtk::Dialog>,
    _server_deleted_channel: relm::Channel<DeleteResult>,
    server_deleted_sender: relm::Sender<DeleteResult>,
}

#[derive(Debug)]
pub struct GridItem {
    pub label_name: &'static str,
    pub icon: Option<Icon>,
    pub markup: String,
    pub raw_value: String,
}

pub enum LabelText {
    PlainText(String),
    Markup(String),
}

impl GridItem {
    pub fn new(
        label_name: &'static str,
        icon: Option<Icon>,
        label_text: LabelText,
        raw_value: String,
    ) -> GridItem {
        GridItem {
            label_name,
            icon,
            markup: match label_text {
                LabelText::PlainText(t) => glib::markup_escape_text(&t).to_string(),
                LabelText::Markup(m) => m,
            },
            raw_value,
        }
    }
}

pub fn populate_popover<T: Copy + PartialEq + Eq>(
    actions_popover: &gtk::Popover,
    copy_action_type: T,
    extra_btns: &[(gtk::ModelButton, T)],
    fields: &[GridItem],
    register_btn: &dyn Fn(&gtk::ModelButton, T, String),
) {
    for child in actions_popover.get_children() {
        actions_popover.remove(&child);
    }
    let popover_vbox = gtk::BoxBuilder::new()
        .margin(10)
        .orientation(gtk::Orientation::Vertical)
        .build();
    for (extra_btn, extra_btn_action_type) in extra_btns {
        popover_vbox.add(extra_btn);
        register_btn(
            &extra_btn,
            *extra_btn_action_type,
            extra_btn.get_label().unwrap().to_string(),
        );
    }
    for item in fields
        .iter()
        .filter(|cur_item| !cur_item.raw_value.is_empty())
    {
        let popover_btn = gtk::ModelButtonBuilder::new()
            .label(&format!("Copy {}", item.label_name))
            .build();
        register_btn(&popover_btn, copy_action_type, item.raw_value.clone());
        popover_vbox.add(&popover_btn);
    }
    popover_vbox.show_all();
    actions_popover.add(&popover_vbox);
}

pub fn populate_grid<T: Copy + PartialEq + Eq>(
    header_grid: gtk::Grid,
    actions_popover: gtk::Popover,
    fields: &[GridItem],
    copy_action_type: T,
    extra_btns: &[(gtk::ModelButton, T)],
    register_btn: &dyn Fn(&gtk::ModelButton, T, String),
) {
    for child in header_grid.get_children() {
        header_grid.remove(&child);
    }

    let mut i = 0;
    for item in fields {
        if !item.markup.is_empty() {
            let label = gtk::LabelBuilder::new()
                .label(&item.label_name)
                .halign(gtk::Align::End) // right align as per gnome HIG
                .build();
            header_grid.attach(&label, 0, i, 1, 1);
            label.get_style_context().add_class("item_label");

            let label = gtk::LabelBuilder::new()
                .use_markup(true) // for 'address' we put the link for instance
                .label(&item.markup)
                .xalign(0.0)
                .single_line_mode(true)
                .ellipsize(pango::EllipsizeMode::End)
                .build();

            if let Some(icon) = &item.icon {
                let gbox = gtk::BoxBuilder::new().build();
                // https://github.com/gtk-rs/gtk/issues/837
                // property_icon_size: 1, // gtk::IconSize::Menu,
                gbox.add(
                    &gtk::ImageBuilder::new()
                        .icon_name(&icon.name())
                        .icon_size(1)
                        .margin_end(5)
                        .build(),
                );
                gbox.add(&label);
                header_grid.attach(&gbox, 1, i, 1, 1);
            } else {
                header_grid.attach(&label, 1, i, 1, 1);
            }

            i += 1;
        }
    }
    header_grid.show_all();
    populate_popover(
        &actions_popover,
        copy_action_type,
        &extra_btns,
        fields,
        register_btn,
    );
}

pub fn get_project_item_fields(project_item: &ProjectItem) -> Vec<GridItem> {
    match project_item {
        ProjectItem::Server(srv) => vec![
            GridItem::new(
                "Address",
                Some(server_access_icon(&srv)),
                server_ip_display(&srv),
                srv.ip.clone(),
            ),
            GridItem::new(
                "Username",
                None,
                LabelText::PlainText(srv.username.clone()),
                srv.username.clone(),
            ),
            GridItem::new(
                "Password",
                None,
                LabelText::PlainText(if srv.password.is_empty() {
                    "".to_string()
                } else {
                    "●●●●●".to_string()
                }),
                srv.password.clone(),
            ),
        ],
        ProjectItem::ProjectPointOfInterest(poi) => vec![
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
                poi.path.clone(),
            ),
        ],
        _ => vec![],
    }
}

fn server_ip_display(srv: &Server) -> LabelText {
    if srv.access_type == ServerAccessType::SrvAccessWww {
        LabelText::Markup(format!(
            "<a href=\"{}\">{}</a>",
            &glib::markup_escape_text(&srv.ip),
            &glib::markup_escape_text(&srv.ip)
        ))
    } else {
        LabelText::PlainText(srv.ip.clone())
    }
}

fn server_access_icon(srv: &Server) -> Icon {
    match srv.access_type {
        ServerAccessType::SrvAccessSsh => Icon::SSH,
        ServerAccessType::SrvAccessSshTunnel => Icon::SSH,
        ServerAccessType::SrvAccessRdp => Icon::WINDOWS,
        ServerAccessType::SrvAccessWww => Icon::HTTP,
    }
}

pub enum AddEditServerInfo<'a> {
    EditServer(&'a Server),
    AddServer(&'a Project),
}

impl AddEditServerInfo<'_> {
    fn is_edit(&self) -> bool {
        match self {
            AddEditServerInfo::EditServer(_) => true,
            _ => false,
        }
    }

    fn project_id(&self) -> i32 {
        match self {
            AddEditServerInfo::EditServer(srv) => srv.project_id,
            AddEditServerInfo::AddServer(prj) => prj.id,
        }
    }

    fn server(&self) -> Option<&Server> {
        match self {
            AddEditServerInfo::EditServer(srv) => Some(srv),
            _ => None,
        }
    }
}

/// you must keep a reference to the component in your model,
/// otherwise event processing will die when the component gets dropped
pub fn prepare_add_edit_server_dialog(
    widget_for_window: gtk::Widget,
    db_sender: mpsc::Sender<SqlFunc>,
    add_edit_info: AddEditServerInfo,
) -> (gtk::Dialog, relm::Component<ServerAddEditDialog>) {
    let dialog_contents = relm::init::<ServerAddEditDialog>((
        db_sender,
        add_edit_info.project_id(),
        add_edit_info.server().cloned(),
    ))
    .expect("error initializing the server add edit modal");
    let d_c = dialog_contents.clone();
    let (dialog, component, _) = standard_dialogs::prepare_custom_dialog(
        widget_for_window,
        600,
        350,
        if add_edit_info.is_edit() {
            "Edit server"
        } else {
            "Add server"
        },
        dialog_contents,
        move |_| {
            d_c.emit(MsgServerAddEditDialog::OkPressed);
            standard_dialogs::DialogActionResult::CloseDialog
        },
    );
    (dialog, component)
}

#[widget]
impl Widget for ProjectPoiHeader {
    fn init_view(&mut self) {
        self.load_project_item();
        self.items_frame
            .get_style_context()
            .add_class("items_frame");

        self.model
            .title
            .get_style_context()
            .add_class("header_frame_title");
        self.model.title.show_all();

        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, Option<ProjectItem>),
    ) -> Model {
        let (db_sender, project_item) = params;
        let stream = relm.stream().clone();
        let (_server_deleted_channel, server_deleted_sender) =
            relm::Channel::new(move |r: DeleteResult| match r {
                Ok(srv) => stream.emit(Msg::ServerDeleted(srv)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        Model {
            relm: relm.clone(),
            db_sender,
            project_item,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
            title: gtk::LabelBuilder::new()
                .margin_top(8)
                .margin_bottom(8)
                .hexpand(true)
                .build(),
            server_add_edit_dialog: None,
            server_add_item_dialog: None,
            server_add_item_dialog_component: None,
            _server_deleted_channel,
            server_deleted_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.project_item = pi;
                self.load_project_item();
            }
            Msg::HeaderActionClicked((ActionTypes::Copy, val)) => {
                if let Some(clip) = gtk::Clipboard::get_default(&self.header_grid.get_display()) {
                    clip.set_text(&val);
                }
            }
            Msg::HeaderActionClicked((ActionTypes::Edit, _)) => {
                match self.model.project_item.clone() {
                    Some(ProjectItem::Server(ref srv)) => {
                        let (dialog, component) = prepare_add_edit_server_dialog(
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            self.model.db_sender.clone(),
                            AddEditServerInfo::EditServer(srv),
                        );
                        relm::connect!(
                            component@MsgServerAddEditDialog::ServerUpdated(ref srv),
                            self.model.relm,
                            Msg::ServerUpdated(srv.clone())
                        );
                        self.model.server_add_edit_dialog = Some(component);
                        dialog.show();
                    }
                    Some(_) => {
                        eprintln!("TODO");
                    }
                    None => {}
                };
            }
            Msg::HeaderActionClicked((ActionTypes::Delete, _)) => {
                match self.model.project_item.as_ref() {
                    Some(ProjectItem::Server(srv)) => {
                        let relm = self.model.relm.clone();
                        let server = srv.clone();
                        standard_dialogs::confirm_deletion(
                            "Delete server",
                            &format!("Are you sure you want to delete the server {}? This action cannot be undone.", srv.desc),
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            move || relm.stream().emit(Msg::DeleteCurrentServer(server.clone()))
                        );
                    }
                    _ => {
                        eprintln!("TODO");
                    }
                }
            }
            Msg::HeaderActionClicked((ActionTypes::AddItem, _)) => {
                self.show_server_add_item_dialog();
            }
            Msg::ServerUpdated(server) => match self.model.project_item.as_ref() {
                Some(ProjectItem::Server(srv)) => {
                    self.model.project_item = Some(ProjectItem::Server(server));
                    self.load_project_item();
                }
                _ => {}
            },
            Msg::DeleteCurrentServer(srv) => {
                self.delete_current_server(srv);
            }
            // for my parent
            Msg::ServerDeleted(_) => {}
            Msg::ServerAddItemActionCompleted => {
                self.model.server_add_item_dialog.as_ref().unwrap().close();
                self.model.server_add_item_dialog = None;
                self.model.server_add_item_dialog_component = None;
                // TODO refresh
            }
        }
    }

    fn show_server_add_item_dialog(&mut self) {
        let dialog_contents = relm::init::<ServerAddItemDialog>((
            self.model.db_sender.clone(),
            match self.model.project_item {
                Some(ProjectItem::Server(ref srv)) => srv.id,
                _ => panic!(),
            },
        ))
        .expect("error initializing the server add item modal");
        let d_c = dialog_contents.clone();
        let (dialog, component, ok_btn) = standard_dialogs::prepare_custom_dialog(
            self.items_frame.clone().upcast::<gtk::Widget>(),
            server_poi_add_edit_dlg::SERVER_POI_ADD_EDIT_WIDTH,
            server_poi_add_edit_dlg::SERVER_POI_ADD_EDIT_HEIGHT,
            "Add server item",
            dialog_contents,
            move |ok_btn| {
                if ok_btn.get_label() == Some("Next".into()) {
                    d_c.emit(server_add_item_dlg::Msg::ShowSecondTab);
                    ok_btn.set_label("Done");
                } else {
                    d_c.emit(server_add_item_dlg::Msg::OkPressed);
                }
                standard_dialogs::DialogActionResult::DontCloseDialog
            },
        );
        ok_btn.set_label("Next");
        relm::connect!(
            component@server_add_item_dlg::Msg::ActionCompleted,
            self.model.relm,
            Msg::ServerAddItemActionCompleted
        );
        self.model.server_add_item_dialog_component = Some(component);
        self.model.server_add_item_dialog = Some(dialog.clone());
        dialog.show();
    }

    fn delete_current_server(&self, server: Server) {
        let server_id = server.id;
        let s = self.model.server_deleted_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server::dsl as srv;
                use projectpadsql::schema::server_database::dsl as db;
                use projectpadsql::schema::server_website::dsl as srvw;

                // we cannot delete a server if a database under it
                // is being used elsewhere
                let dependent_websites = srvw::server_website
                    .inner_join(db::server_database)
                    .filter(db::server_id.eq(server_id))
                    .load::<(ServerWebsite, ServerDatabase)>(sql_conn)
                    .unwrap();
                if !dependent_websites.is_empty() {
                    s.send(Err((
                        "Cannot delete server",
                        Some(format!(
                            "databases {} on that server are used by websites {}",
                            itertools::join(dependent_websites.iter().map(|(_, d)| &d.name), ", "),
                            itertools::join(dependent_websites.iter().map(|(w, _)| &w.desc), ", ")
                        )),
                    )))
                    .unwrap();
                } else {
                    s.send(
                        match diesel::delete(srv::server.filter(srv::id.eq(server_id)))
                            .execute(sql_conn)
                        {
                            Ok(1) => Ok(server.clone()),
                            Ok(x) => Err((
                                "Server deletion failed",
                                Some(format!(
                                    "Expected 1 row to be modified, but {} rows were modified",
                                    x
                                )),
                            )),
                            Err(e) => Err(("Server deletion failed", Some(e.to_string()))),
                        },
                    )
                    .unwrap();
                }
            }))
            .unwrap();
    }

    fn load_project_item(&self) {
        self.populate_header();
        self.model.title.set_markup(
            self.model
                .project_item
                .as_ref()
                .map(Self::project_item_desc)
                .as_deref()
                .unwrap_or(""),
        );
    }

    fn project_item_desc(pi: &ProjectItem) -> &str {
        match pi {
            ProjectItem::Server(srv) => &srv.desc,
            ProjectItem::ServerLink(srv) => &srv.desc,
            ProjectItem::ProjectNote(note) => &note.title,
            ProjectItem::ProjectPointOfInterest(poi) => &poi.desc,
        }
    }

    fn populate_header(&self) {
        let fields = self
            .model
            .project_item
            .as_ref()
            .map(get_project_item_fields)
            .unwrap_or_else(|| vec![]);
        let extra_btns = [
            (
                gtk::ModelButtonBuilder::new().label("Edit").build(),
                ActionTypes::Edit,
            ),
            (
                gtk::ModelButtonBuilder::new().label("Add...").build(),
                ActionTypes::AddItem,
            ),
            (
                gtk::ModelButtonBuilder::new().label("Delete").build(),
                ActionTypes::Delete,
            ),
        ];
        populate_grid(
            self.header_grid.clone(),
            self.model.header_popover.clone(),
            &fields,
            ActionTypes::Copy,
            &extra_btns,
            &|btn: &gtk::ModelButton, action_type: ActionTypes, str_val: String| {
                relm::connect!(
                    self.model.relm,
                    btn,
                    connect_clicked(_),
                    Msg::HeaderActionClicked((action_type, str_val.clone()))
                );
            },
        );
    }

    view! {
        #[name="items_frame"]
        gtk::Frame {
            hexpand: true,
            margin_start: 10,
            margin_end: 10,
            margin_top: 10,
            gtk::Box {
                hexpand: true,
                orientation: gtk::Orientation::Vertical,
                #[name="titlebox"]
                gtk::Box {
                    hexpand: true,
                    orientation: gtk::Orientation::Horizontal,
                    center_widget: Some(&self.model.title),
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
                        margin_end: 10,
                    },
                },
                #[name="header_grid"]
                gtk::Grid {
                    margin_start: 30,
                    margin_end: 30,
                    margin_top: 10,
                    margin_bottom: 5,
                    row_spacing: 5,
                    column_spacing: 10,
                },
            }
        }
    }
}
