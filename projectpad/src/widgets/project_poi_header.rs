use super::dialogs;
use super::dialogs::dialog_helpers;
use super::dialogs::project_note_add_edit_dlg;
use super::dialogs::project_note_add_edit_dlg::Msg as MsgProjectNoteAddEditDialog;
use super::dialogs::project_poi_add_edit_dlg;
use super::dialogs::project_poi_add_edit_dlg::Msg as MsgProjectPoiAddEditDialog;
use super::dialogs::server_add_edit_dlg;
use super::dialogs::server_add_edit_dlg::Msg as MsgServerAddEditDialog;
use super::dialogs::server_add_item_dlg;
use super::dialogs::server_add_item_dlg::ServerAddItemDialog;
use super::dialogs::server_link_add_edit_dlg;
use super::dialogs::server_link_add_edit_dlg::Msg as MsgServerLinkAddEditDialog;
use super::dialogs::standard_dialogs;
use super::project_items_list::ProjectItem;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerAccessType, ServerDatabase,
    ServerLink, ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActionTypes {
    Edit,
    Copy,
    Delete,
    AddItem,
    GotoItem,
}

#[derive(Msg, Clone)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    HeaderActionClicked((ActionTypes, String)),
    ProjectItemRefresh(ProjectItem),
    ProjectItemDeleted(ProjectItem),
    DeleteCurrentServer(Server),
    DeleteCurrentProjectPoi(ProjectPointOfInterest),
    DeleteCurrentProjectNote(ProjectNote),
    DeleteCurrentServerLink(ServerLink),
    ServerAddItemActionCompleted,
    ServerAddItemChangeTitleTitle(&'static str),
    ProjectItemUpdated(Option<ProjectItem>),
    GotoItem(Project, Server),
    ShowInfoBar(String),
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<ProjectItem, (&'static str, Option<String>)>;

type GotoResult = (Project, Server);

pub struct Model {
    relm: relm::Relm<ProjectPoiHeader>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_item: Option<ProjectItem>,
    header_popover: gtk::Popover,
    title: gtk::Label,
    project_add_edit_dialog: Option<(dialogs::ProjectAddEditDialogComponent, gtk::Dialog)>,
    server_add_item_dialog_component: Option<relm::Component<ServerAddItemDialog>>,
    server_add_item_dialog: Option<gtk::Dialog>,
    _project_item_deleted_channel: relm::Channel<DeleteResult>,
    project_item_deleted_sender: relm::Sender<DeleteResult>,
    _goto_server_channel: relm::Channel<GotoResult>,
    goto_server_sender: relm::Sender<GotoResult>,
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

pub fn populate_popover(
    actions_popover: &gtk::Popover,
    extra_btns: &[gtk::ModelButton],
    fields: &[GridItem],
    register_copy_btn: &dyn Fn(&gtk::ModelButton, String),
) {
    for child in actions_popover.get_children() {
        actions_popover.remove(&child);
    }
    let popover_vbox = gtk::BoxBuilder::new()
        .margin(10)
        .orientation(gtk::Orientation::Vertical)
        .build();
    for extra_btn in extra_btns {
        popover_vbox.add(extra_btn);
    }
    for item in fields
        .iter()
        .filter(|cur_item| !cur_item.raw_value.is_empty())
    {
        let popover_btn = gtk::ModelButtonBuilder::new()
            .label(&format!("Copy {}", item.label_name))
            .build();
        register_copy_btn(&popover_btn, item.raw_value.clone());
        popover_vbox.add(&popover_btn);
    }
    popover_vbox.show_all();
    actions_popover.add(&popover_vbox);
}

pub fn populate_grid(
    header_grid: gtk::Grid,
    actions_popover: gtk::Popover,
    fields: &[GridItem],
    extra_btns: &[gtk::ModelButton],
    register_copy_btn: &dyn Fn(&gtk::ModelButton, String),
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
    populate_popover(&actions_popover, extra_btns, fields, register_copy_btn);
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

#[widget]
impl Widget for ProjectPoiHeader {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.header_grid);
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
        let (_project_item_deleted_channel, project_item_deleted_sender) =
            relm::Channel::new(move |r: DeleteResult| match r {
                Ok(pi) => stream.emit(Msg::ProjectItemDeleted(pi)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let stream2 = relm.stream().clone();
        let (_goto_server_channel, goto_server_sender) =
            relm::Channel::new(move |r: GotoResult| {
                stream2.emit(Msg::GotoItem(r.0.clone(), r.1.clone()))
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
            project_add_edit_dialog: None,
            server_add_item_dialog: None,
            server_add_item_dialog_component: None,
            _project_item_deleted_channel,
            project_item_deleted_sender,
            _goto_server_channel,
            goto_server_sender,
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
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ShowInfoBar("Copied to the clipboard".to_string()));
            }
            Msg::HeaderActionClicked((ActionTypes::GotoItem, val)) => {
                match &self.model.project_item {
                    Some(ProjectItem::ServerLink(l)) => {
                        let s = self.model.goto_server_sender.clone();
                        let linked_server_id = l.linked_server_id;
                        self.model
                            .db_sender
                            .send(SqlFunc::new(move |sql_conn| {
                                use projectpadsql::schema::project::dsl as prj;
                                use projectpadsql::schema::server::dsl as srv;
                                let (srv, prj) = srv::server
                                    .inner_join(prj::project)
                                    .filter(srv::id.eq(linked_server_id))
                                    .first::<(Server, Project)>(sql_conn)
                                    .unwrap();
                                s.send((prj, srv)).unwrap();
                            }))
                            .unwrap();
                    }
                    _ => {}
                }
            }
            Msg::HeaderActionClicked((ActionTypes::Edit, _)) => {
                match self.model.project_item.clone() {
                    Some(ProjectItem::Server(ref srv)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            dialog_helpers::prepare_dialog_param(
                                self.model.db_sender.clone(),
                                srv.project_id,
                                Some(srv.clone()),
                            ),
                            server_add_edit_dlg::Msg::OkPressed,
                            "Server",
                        );
                        relm::connect!(
                            component@MsgServerAddEditDialog::ServerUpdated(ref srv),
                            self.model.relm,
                            Msg::ProjectItemRefresh(ProjectItem::Server(srv.clone()))
                        );
                        self.model.project_add_edit_dialog = Some((
                            dialogs::ProjectAddEditDialogComponent::Server(component),
                            dialog.clone(),
                        ));
                        dialog.show();
                    }
                    Some(ProjectItem::ProjectPointOfInterest(ref poi)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            dialog_helpers::prepare_dialog_param(
                                self.model.db_sender.clone(),
                                poi.project_id,
                                Some(poi.clone()),
                            ),
                            project_poi_add_edit_dlg::Msg::OkPressed,
                            "Project POI",
                        );
                        relm::connect!(
                            component@MsgProjectPoiAddEditDialog::PoiUpdated(ref poi),
                            self.model.relm,
                            Msg::ProjectItemRefresh(ProjectItem::ProjectPointOfInterest(poi.clone()))
                        );
                        self.model.project_add_edit_dialog = Some((
                            dialogs::ProjectAddEditDialogComponent::ProjectPoi(component),
                            dialog.clone(),
                        ));
                        dialog.show();
                    }
                    Some(ProjectItem::ProjectNote(ref note)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            dialog_helpers::prepare_dialog_param(
                                self.model.db_sender.clone(),
                                note.project_id,
                                Some(note.clone()),
                            ),
                            project_note_add_edit_dlg::Msg::OkPressed,
                            "Project note",
                        );
                        relm::connect!(
                            component@MsgProjectNoteAddEditDialog::ProjectNoteUpdated(ref note),
                            self.model.relm,
                            Msg::ProjectItemRefresh(ProjectItem::ProjectNote(note.clone()))
                        );
                        self.model.project_add_edit_dialog = Some((
                            dialogs::ProjectAddEditDialogComponent::ProjectNote(component),
                            dialog.clone(),
                        ));
                        dialog.show();
                    }
                    Some(ProjectItem::ServerLink(ref link)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.items_frame.clone().upcast::<gtk::Widget>(),
                            dialog_helpers::prepare_dialog_param(
                                self.model.db_sender.clone(),
                                link.project_id,
                                Some(link.clone()),
                            ),
                            server_link_add_edit_dlg::Msg::OkPressed,
                            "Server link",
                        );
                        relm::connect!(
                            component@MsgServerLinkAddEditDialog::ServerLinkUpdated(ref srv_link),
                            self.model.relm,
                            Msg::ProjectItemRefresh(ProjectItem::ServerLink(srv_link.clone()))
                        );
                        self.model.project_add_edit_dialog = Some((
                            dialogs::ProjectAddEditDialogComponent::ServerLink(component),
                            dialog.clone(),
                        ));
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
                        self.ask_deletion(
                            "server",
                            srv.desc.as_str(),
                            Msg::DeleteCurrentServer(srv.clone()),
                        );
                    }
                    Some(ProjectItem::ProjectPointOfInterest(poi)) => {
                        self.ask_deletion(
                            "project point of interest",
                            poi.desc.as_str(),
                            Msg::DeleteCurrentProjectPoi(poi.clone()),
                        );
                    }
                    Some(ProjectItem::ProjectNote(note)) => {
                        self.ask_deletion(
                            "project note",
                            note.title.as_str(),
                            Msg::DeleteCurrentProjectNote(note.clone()),
                        );
                    }
                    Some(ProjectItem::ServerLink(srv_link)) => {
                        self.ask_deletion(
                            "server link",
                            srv_link.desc.as_str(),
                            Msg::DeleteCurrentServerLink(srv_link.clone()),
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
            Msg::ProjectItemRefresh(project_item) => {
                if let Some((_, dialog)) = self.model.project_add_edit_dialog.as_ref() {
                    dialog.close();
                    self.model.project_add_edit_dialog = None;
                }
                self.model.project_item = Some(project_item);
                self.load_project_item();
            }
            Msg::DeleteCurrentServer(srv) => {
                self.delete_current_server(srv);
            }
            Msg::DeleteCurrentProjectPoi(poi) => {
                let poi_id = poi.id;
                let s = self.model.project_item_deleted_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
                        s.send(
                            dialog_helpers::delete_row(
                                sql_conn,
                                prj_poi::project_point_of_interest,
                                poi_id,
                            )
                            .map(|_| ProjectItem::ProjectPointOfInterest(poi.clone())),
                        )
                        .unwrap();
                    }))
                    .unwrap();
            }
            Msg::DeleteCurrentProjectNote(note) => {
                let note_id = note.id;
                let s = self.model.project_item_deleted_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        use projectpadsql::schema::project_note::dsl as prj_note;
                        s.send(
                            dialog_helpers::delete_row(sql_conn, prj_note::project_note, note_id)
                                .map(|_| ProjectItem::ProjectNote(note.clone())),
                        )
                        .unwrap();
                    }))
                    .unwrap();
            }
            Msg::DeleteCurrentServerLink(srv_link) => {
                let link_id = srv_link.id;
                let s = self.model.project_item_deleted_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        use projectpadsql::schema::server_link::dsl as srv_link;
                        s.send(
                            dialog_helpers::delete_row(sql_conn, srv_link::server_link, link_id)
                                .map(|_| ProjectItem::ServerLink(srv_link.clone())),
                        )
                        .unwrap();
                    }))
                    .unwrap();
            }
            // for my parent
            Msg::ProjectItemDeleted(_) => {}
            Msg::ServerAddItemActionCompleted => {
                self.model.server_add_item_dialog.as_ref().unwrap().close();
                self.model.server_add_item_dialog = None;
                self.model.server_add_item_dialog_component = None;
                // refresh
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ProjectItemUpdated(self.model.project_item.clone()));
            }
            Msg::ServerAddItemChangeTitleTitle(title) => {
                self.model
                    .server_add_item_dialog
                    .as_ref()
                    .unwrap()
                    .set_title(title);
            }
            // meant for my parent
            Msg::ShowInfoBar(_) => {}
            Msg::ProjectItemUpdated(pi) => {}
            Msg::GotoItem(_, _) => {}
        }
    }

    fn ask_deletion(&self, item_type_desc: &str, item_desc: &str, msg: Msg) {
        let relm = self.model.relm.clone();
        standard_dialogs::confirm_deletion(
            &format!("Delete {}", item_type_desc),
            &format!(
                "Are you sure you want to delete the {} {}? This action cannot be undone.",
                item_type_desc, item_desc
            ),
            self.items_frame.clone().upcast::<gtk::Widget>(),
            move || relm.stream().emit(msg.clone()),
        );
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
        let dialog = standard_dialogs::modal_dialog(
            self.items_frame.clone().upcast::<gtk::Widget>(),
            600,
            200,
            "Add server item".to_string(),
        );
        let (dialog, component, ok_btn) = standard_dialogs::prepare_custom_dialog(
            dialog.clone(),
            dialog_contents,
            move |ok_btn| {
                if ok_btn.get_label() == Some("Next".into()) {
                    d_c.emit(server_add_item_dlg::Msg::ShowSecondTab(dialog.clone()));
                    ok_btn.set_label("Done");
                } else {
                    d_c.emit(server_add_item_dlg::Msg::OkPressed);
                }
            },
        );
        ok_btn.set_label("Next");
        relm::connect!(
            component@server_add_item_dlg::Msg::ActionCompleted(ref si),
            self.model.relm,
            Msg::ServerAddItemActionCompleted
        );
        relm::connect!(
            component@server_add_item_dlg::Msg::ChangeDialogTitle(title),
            self.model.relm,
            Msg::ServerAddItemChangeTitleTitle(title)
        );
        self.model.server_add_item_dialog_component = Some(component);
        self.model.server_add_item_dialog = Some(dialog.clone());
        dialog.show();
    }

    fn delete_current_server(&self, server: Server) {
        let server_id = server.id;
        let s = self.model.project_item_deleted_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server::dsl as srv;
                use projectpadsql::schema::server_database::dsl as db;
                use projectpadsql::schema::server_link::dsl as srv_link;
                use projectpadsql::schema::server_website::dsl as srvw;

                // we cannot delete a server if a database under it
                // is being used elsewhere
                let dependent_websites = srvw::server_website
                    .inner_join(db::server_database)
                    .filter(db::server_id.eq(server_id))
                    .load::<(ServerWebsite, ServerDatabase)>(sql_conn)
                    .unwrap();
                let dependent_serverlinks = srv_link::server_link
                    .filter(srv_link::linked_server_id.eq(server_id))
                    .load::<ServerLink>(sql_conn)
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
                } else if !dependent_serverlinks.is_empty() {
                    s.send(Err((
                        "Cannot delete server",
                        Some(format!(
                            "server links {} are tied to this server",
                            itertools::join(dependent_serverlinks.iter().map(|l| &l.desc), ", "),
                        )),
                    )))
                } else {
                    s.send(
                        dialog_helpers::delete_row(sql_conn, srv::server, server_id)
                            .map(|_| ProjectItem::Server(server.clone())),
                    )
                }
                .unwrap();
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
        let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
        relm::connect!(
            self.model.relm,
            &edit_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::Edit, "".to_string()))
        );
        let add_btn = gtk::ModelButtonBuilder::new().label("Add...").build();
        relm::connect!(
            self.model.relm,
            &add_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::AddItem, "".to_string()))
        );
        let goto_btn = gtk::ModelButtonBuilder::new().label("Go to").build();
        relm::connect!(
            self.model.relm,
            &goto_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::GotoItem, "".to_string()))
        );
        let delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
        relm::connect!(
            self.model.relm,
            &delete_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::Delete, "".to_string()))
        );
        let extra_btns = match &self.model.project_item {
            Some(ProjectItem::Server(_)) => vec![edit_btn, add_btn, delete_btn],
            Some(ProjectItem::ServerLink(_)) => vec![edit_btn, goto_btn, delete_btn],
            Some(_) => vec![edit_btn, delete_btn],
            _ => vec![],
        };
        populate_grid(
            self.header_grid.clone(),
            self.model.header_popover.clone(),
            &fields,
            &extra_btns,
            &|btn: &gtk::ModelButton, str_val: String| {
                relm::connect!(
                    self.model.relm,
                    btn,
                    connect_clicked(_),
                    Msg::HeaderActionClicked((ActionTypes::Copy, str_val.clone()))
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
                },
            }
        }
    }
}
