use super::dialogs;
use super::dialogs::dialog_helpers;
use super::dialogs::project_item_move_dlg;
use super::dialogs::project_item_move_dlg::Msg as ProjectItemMoveDlgMsg;
use super::dialogs::project_item_move_dlg::ProjectItemMoveDialog;
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
use super::dialogs::server_poi_add_edit_dlg;
use super::dialogs::standard_dialogs;
use super::project_items_list::ProjectItem;
use super::wintitlebar::left_align_menu;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use crate::sql_util;
use diesel::prelude::*;
use gdk::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerAccessType, ServerDatabase,
    ServerLink, ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ActionTypes {
    Edit,
    Copy,
    Delete,
    Move,
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
    GotoItem(Project, ProjectItem),
    ShowInfoBar(String),
    CopyPassword,
    OpenLinkOrEditProjectNote,
    OpenSingleWebsiteLink,
    GotLinkedServer(Server),
    MoveApplied(Result<(Project, ProjectItem, project_item_move_dlg::ProjectUpdated), String>),
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<ProjectItem, (&'static str, Option<String>)>;

type GotoResult = (Project, Server);

pub struct Model {
    relm: relm::Relm<ProjectPoiHeader>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_item: Option<ProjectItem>,
    server_link_target: Option<Server>, // this is only populated when we display a ServerLink..
    header_popover: gtk::Popover,
    title: gtk::Label,
    project_item_move_dialog: Option<(relm::Component<ProjectItemMoveDialog>, gtk::Dialog)>,
    project_add_edit_dialog: Option<(dialogs::ProjectAddEditDialogComponent, gtk::Dialog)>,
    server_add_item_dialog_component: Option<relm::Component<ServerAddItemDialog>>,
    server_add_item_dialog: Option<gtk::Dialog>,
    _project_item_deleted_channel: relm::Channel<DeleteResult>,
    project_item_deleted_sender: relm::Sender<DeleteResult>,
    _goto_server_channel: relm::Channel<GotoResult>,
    goto_server_sender: relm::Sender<GotoResult>,
    _load_linkedserver_channel: relm::Channel<Server>,
    load_linkedserver_sender: relm::Sender<Server>,
}

#[derive(Debug)]
pub struct GridItem {
    pub label_name: &'static str,
    pub icon: Option<Icon>,
    pub markup: String,
    pub raw_value: String,
    pub shortcut: Option<(gdk::keys::Key, gdk::ModifierType)>,
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
        shortcut: Option<(gdk::keys::Key, gdk::ModifierType)>,
    ) -> GridItem {
        GridItem {
            label_name,
            icon,
            markup: match label_text {
                LabelText::PlainText(t) => glib::markup_escape_text(&t).to_string(),
                LabelText::Markup(m) => m,
            },
            raw_value,
            shortcut,
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
        .width_request(180) // without that the "copy password" accelerator isn't shown!
        .orientation(gtk::Orientation::Vertical)
        .build();
    for extra_btn in extra_btns {
        popover_vbox.add(extra_btn);
        left_align_menu(extra_btn);
    }
    let fields_to_copy: Vec<_> = fields
        .iter()
        .filter(|cur_item| !cur_item.raw_value.is_empty())
        .collect();
    if !fields_to_copy.is_empty() {
        popover_vbox.add(&gtk::SeparatorBuilder::new().build());
    }
    for item in fields_to_copy.iter() {
        let label = &format!("Copy {}", item.label_name);
        let popover_btn = match &item.shortcut {
            None => gtk::ModelButtonBuilder::new().label(label).build(),
            Some((key, modifiers)) => label_with_accelerator(label, key, *modifiers),
        };
        left_align_menu(&popover_btn);
        register_copy_btn(&popover_btn, item.raw_value.clone());
        popover_vbox.add(&popover_btn);
    }
    popover_vbox.show_all();
    actions_popover.add(&popover_vbox);
}

fn label_with_accelerator(
    label: &str,
    key: &gdk::keys::Key,
    modifiers: gdk::ModifierType,
) -> gtk::ModelButton {
    let lbl = gtk::ModelButtonBuilder::new().build();
    let accel_lbl = gtk::AccelLabelBuilder::new().label(label).build();
    accel_lbl.set_accel(**key, modifiers);
    accel_lbl.set_hexpand(true);
    accel_lbl.set_xalign(0.0);
    accel_lbl.show_all();
    lbl.get_child()
        .unwrap()
        .downcast::<gtk::Box>()
        .unwrap()
        .add(&accel_lbl);
    lbl
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
                .label(item.label_name)
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
                        .icon_name(icon.name())
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
    header_grid.set_visible(!fields.is_empty());
}

// i don't like bool parameters... well, just this once.
pub fn get_project_item_fields(
    project_item: &ProjectItem,
    server_link_target: Option<&Server>,
    is_search_view: bool,
) -> Vec<GridItem> {
    match project_item {
        ProjectItem::Server(srv) => vec![
            GridItem::new(
                "Address",
                Some(server_access_icon(srv)),
                server_ip_display(srv),
                srv.ip.clone(),
                None,
            ),
            GridItem::new(
                "Username",
                None,
                LabelText::PlainText(srv.username.clone()),
                srv.username.clone(),
                None,
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
                // don't display the shortcut info in search mode, because in search mode
                // we may display several server headers, so the shortcut wouldn't know
                // which one to pick.
                Some((gdk::keys::constants::Y, gdk::ModifierType::CONTROL_MASK))
                    .filter(|_| !is_search_view),
            ),
            GridItem::new(
                "Text",
                None,
                LabelText::PlainText(srv.text.clone()),
                srv.text.clone(),
                None,
            ),
        ],
        ProjectItem::ServerLink(_link) if server_link_target.is_some() => {
            let link_target = server_link_target.as_ref().unwrap();
            let mut items = vec![GridItem::new(
                "Links to",
                None,
                LabelText::PlainText(link_target.desc.clone()),
                link_target.desc.clone(),
                None,
            )];
            items.extend(
                server_link_target
                    .as_ref()
                    .map(|s| {
                        get_project_item_fields(
                            &ProjectItem::Server((*s).clone()),
                            server_link_target,
                            is_search_view,
                        )
                    })
                    .unwrap_or_else(Vec::new),
            );
            items
        }
        ProjectItem::ProjectPointOfInterest(poi) => vec![
            GridItem::new(
                "Interest Type",
                None,
                LabelText::PlainText(
                    server_poi_add_edit_dlg::interest_type_desc(poi.interest_type).to_string(),
                ),
                poi.path.clone(),
                None,
            ),
            GridItem::new(
                "Path",
                None,
                LabelText::PlainText(poi.path.clone()),
                poi.path.clone(),
                None,
            ),
            GridItem::new(
                server_poi_add_edit_dlg::poi_get_text_label(poi.interest_type),
                None,
                LabelText::PlainText(poi.text.clone()),
                poi.text.clone(),
                None,
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
        dialog_helpers::style_grid(&self.widgets.header_grid);
        self.load_project_item();

        self.model
            .title
            .get_style_context()
            .add_class("header_frame_title");
        self.model.title.show_all();

        self.widgets
            .header_actions_btn
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
                Err((msg, e)) => standard_dialogs::display_error_str(msg, e),
            });
        let stream2 = relm.stream().clone();
        let (_goto_server_channel, goto_server_sender) =
            relm::Channel::new(move |r: GotoResult| {
                stream2.emit(Msg::GotoItem(r.0.clone(), ProjectItem::Server(r.1)))
            });
        let stream3 = relm.stream().clone();
        let (_load_linkedserver_channel, load_linkedserver_sender) =
            relm::Channel::new(move |s: Server| stream3.emit(Msg::GotLinkedServer(s)));
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
            project_item_move_dialog: None,
            project_add_edit_dialog: None,
            server_add_item_dialog: None,
            server_add_item_dialog_component: None,
            _project_item_deleted_channel,
            project_item_deleted_sender,
            _goto_server_channel,
            goto_server_sender,
            _load_linkedserver_channel,
            load_linkedserver_sender,
            server_link_target: None,
        }
    }

    fn copy_to_clipboard(&self, val: &str) {
        if let Some(clip) = gtk::Clipboard::get_default(&self.widgets.header_grid.get_display()) {
            clip.set_text(val);
        }
        self.model
            .relm
            .stream()
            .emit(Msg::ShowInfoBar("Copied to the clipboard".to_string()));
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.project_item = pi;
                self.load_project_item();
            }
            Msg::HeaderActionClicked((ActionTypes::Copy, val)) => {
                self.copy_to_clipboard(&val);
            }
            Msg::HeaderActionClicked((ActionTypes::GotoItem, _val)) => {
                if let Some(ProjectItem::ServerLink(l)) = &self.model.project_item {
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
            }
            Msg::HeaderActionClicked((ActionTypes::Edit, _)) => {
                match self.model.project_item.clone() {
                    Some(ProjectItem::Server(ref srv)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
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
                            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
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
                        let note_copy = note.clone();
                        self.edit_project_note(note_copy);
                    }
                    Some(ProjectItem::ServerLink(ref link)) => {
                        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
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
                    None => {}
                };
            }
            Msg::HeaderActionClicked((ActionTypes::Move, _))
                if self.model.project_item.is_some() =>
            {
                // unwrap: if is_some() in the if in the match
                self.display_projectitem_move_dialog(self.model.project_item.clone().unwrap());
            }
            Msg::HeaderActionClicked((ActionTypes::Move, _)) => {}
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
                    None => {}
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
                            sql_util::delete_row(
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
                            sql_util::delete_row(sql_conn, prj_note::project_note, note_id)
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
                            sql_util::delete_row(sql_conn, srv_link::server_link, link_id)
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
            Msg::CopyPassword => {
                if let Some(ProjectItem::Server(srv)) = self.model.project_item.as_ref() {
                    if !srv.password.is_empty() {
                        self.copy_to_clipboard(&srv.password);
                    }
                }
            }
            Msg::OpenLinkOrEditProjectNote => {
                match self.model.project_item.as_ref() {
                    Some(ProjectItem::Server(srv)) => {
                        if srv.access_type == ServerAccessType::SrvAccessWww && !srv.ip.is_empty() {
                            if let Result::Err(e) =
                                gtk::show_uri_on_window(None::<&gtk::Window>, &srv.ip, 0)
                            {
                                eprintln!("Error opening link: {}", e);
                            }
                        } else {
                            // ok, the server has no link. we could still open it, if
                            // there's a single website with an address under that server
                            self.model.relm.stream().emit(Msg::OpenSingleWebsiteLink);
                        }
                    }
                    Some(ProjectItem::ProjectNote(note)) => {
                        let note_copy = note.clone();
                        self.edit_project_note(note_copy);
                    }
                    _ => {}
                }
            }
            Msg::GotLinkedServer(srv) => {
                self.model.server_link_target = Some(srv);
                self.populate_header();
            }
            Msg::MoveApplied(Ok((p, pi, _project_updated))) => {
                self.model.relm.stream().emit(Msg::GotoItem(p, pi));
            }
            Msg::MoveApplied(Err(e)) => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ShowInfoBar(format!("Error moving item: {:?}", e)));
            }
            // meant for my parent
            Msg::OpenSingleWebsiteLink => {}
            Msg::ShowInfoBar(_) => {}
            Msg::ProjectItemUpdated(_pi) => {}
            Msg::GotoItem(_, _) => {}
        }
    }

    fn display_projectitem_move_dialog(&mut self, project_item: ProjectItem) {
        let dialog = standard_dialogs::modal_dialog(
            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
            600,
            450,
            "Move project item".to_string(),
        );
        let dialog_contents =
            relm::init::<ProjectItemMoveDialog>((self.model.db_sender.clone(), project_item))
                .expect("error initializing the move project item modal");
        let d_c = dialog_contents.stream();

        let move_btn = dialog
            .add_button("Move", gtk::ResponseType::Ok)
            .downcast::<gtk::Button>()
            .expect("error reading the dialog move button");
        move_btn.set_property_has_default(true);
        move_btn.get_style_context().add_class("suggested-action");

        standard_dialogs::prepare_custom_dialog_component_ref(&dialog, &dialog_contents);

        relm::connect!(d_c@ProjectItemMoveDlgMsg::MoveApplied(ref p), self.model.relm, Msg::MoveApplied(p.as_ref().clone()));

        self.model.project_item_move_dialog = Some((dialog_contents, dialog.clone()));
        dialog.connect_response(move |d, r| {
            if r == gtk::ResponseType::Ok {
                d_c.emit(ProjectItemMoveDlgMsg::MoveActionTriggered);
            }
            d.close();
        });
        dialog.show();
    }

    fn edit_project_note(&mut self, note: ProjectNote) {
        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
            dialog_helpers::prepare_dialog_param(
                self.model.db_sender.clone(),
                note.project_id,
                Some(note),
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

    fn ask_deletion(&self, item_type_desc: &str, item_desc: &str, msg: Msg) {
        let relm = self.model.relm.clone();
        standard_dialogs::confirm_deletion(
            &format!("Delete {}", item_type_desc),
            &format!(
                "Are you sure you want to delete the {} {}? This action cannot be undone.",
                item_type_desc, item_desc
            ),
            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
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
        let d_c = dialog_contents.stream();
        let dialog = standard_dialogs::modal_dialog(
            self.widgets.items_frame.clone().upcast::<gtk::Widget>(),
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
            component@server_add_item_dlg::Msg::ActionCompleted(ref _si),
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
                        sql_util::delete_row(sql_conn, srv::server, server_id)
                            .map(|_| ProjectItem::Server(server.clone())),
                    )
                }
                .unwrap();
            }))
            .unwrap();
    }

    fn load_project_item(&self) {
        self.populate_header();
        self.model.title.set_text(
            self.model
                .project_item
                .as_ref()
                .map(Self::project_item_desc)
                .as_deref()
                .unwrap_or(""),
        );
        if let Some(ProjectItem::ServerLink(ref link)) = self.model.project_item {
            let s = self.model.load_linkedserver_sender.clone();
            let linked_server_id = link.linked_server_id;
            self.model
                .db_sender
                .send(SqlFunc::new(move |sql_conn| {
                    use projectpadsql::schema::server::dsl as srv;
                    let server: Server =
                        srv::server.find(linked_server_id).first(sql_conn).unwrap();
                    s.send(server).unwrap();
                }))
                .unwrap();
        }
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
            .map(|pi| get_project_item_fields(pi, self.model.server_link_target.as_ref(), false))
            .unwrap_or_else(Vec::new);
        let edit_btn = if let Some(ProjectItem::ProjectNote(_)) = self.model.project_item.as_ref() {
            label_with_accelerator(
                "Edit",
                &gdk::keys::constants::E,
                gdk::ModifierType::CONTROL_MASK,
            )
        } else {
            gtk::ModelButtonBuilder::new().label("Edit").build()
        };
        relm::connect!(
            self.model.relm,
            &edit_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::Edit, "".to_string()))
        );
        let add_btn = gtk::ModelButtonBuilder::new()
            .label("Add server item...")
            .build();
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
        let move_btn = gtk::ModelButtonBuilder::new().label("Move...").build();
        relm::connect!(
            self.model.relm,
            &move_btn,
            connect_clicked(_),
            Msg::HeaderActionClicked((ActionTypes::Move, "".to_string()))
        );
        let extra_btns = match &self.model.project_item {
            Some(ProjectItem::Server(_)) => vec![add_btn, edit_btn, delete_btn, move_btn],
            Some(ProjectItem::ServerLink(_)) => vec![edit_btn, goto_btn, delete_btn, move_btn],
            Some(_) => vec![edit_btn, delete_btn, move_btn],
            _ => vec![],
        };
        match &self.model.project_item {
            Some(ProjectItem::Server(srv)) if srv.is_retired => {
                self.widgets
                    .titlebox
                    .get_style_context()
                    .add_class("project_poi_header_titlebox_retired");
            }
            _ => {
                self.widgets
                    .titlebox
                    .get_style_context()
                    .remove_class("project_poi_header_titlebox_retired");
            }
        };
        populate_grid(
            self.widgets.header_grid.clone(),
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
        #[style_class="items_frame"]
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
