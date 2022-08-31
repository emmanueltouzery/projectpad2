use super::dialogs::dialog_helpers;
use super::dialogs::project_add_edit_dlg::Msg as MsgProjectAddEditDialog;
use super::dialogs::project_add_edit_dlg::ProjectAddEditDialog;
use super::dialogs::project_item_move_dlg;
use super::dialogs::standard_dialogs;
use super::dialogs::unlock_db_dlg;
use super::dialogs::unlock_db_dlg::Msg as MsgUnlockDbDlg;
use super::dialogs::unlock_db_dlg::UnlockDbDialog;
use super::keyring_helpers;
use super::project_items_list::Msg as ProjectItemsListMsg;
use super::project_items_list::{ProjectItem, ProjectItemsList};
use super::project_list::Msg as ProjectListMsg;
use super::project_list::{
    Msg::AddProject, Msg::ProjectActivated, Msg::UpdateProjectTooltip, ProjectList, UpdateParents,
};
use super::project_poi_contents::Msg as ProjectPoiContentsMsg;
use super::project_poi_contents::Msg::RequestDisplayServerItem as ProjectPoiContentsMsgRequestDisplayServerItem;
use super::project_poi_contents::Msg::ShowInfoBar as ProjectPoiContentsMsgShowInfoBar;
use super::project_poi_contents::ProjectPoiContents;
use super::project_poi_header::Msg as ProjectPoiHeaderMsg;
use super::project_poi_header::Msg::GotoItem as ProjectPoiHeaderGotoItemMsg;
use super::project_poi_header::Msg::MoveApplied as ProjectPoiHeaderMoveApplied;
use super::project_poi_header::Msg::OpenSingleWebsiteLink as ProjectPoiHeaderOpenSingleWebsiteLink;
use super::project_poi_header::Msg::ProjectItemDeleted as ProjectPoiHeaderProjectItemDeletedMsg;
use super::project_poi_header::Msg::ProjectItemRefresh as ProjectPoiHeaderProjectItemRefreshMsg;
use super::project_poi_header::Msg::ProjectItemUpdated as ProjectPoiHeaderProjectItemUpdatedMsg;
use super::project_poi_header::Msg::ShowInfoBar as ProjectPoiHeaderShowInfoBar;
use super::project_poi_header::ProjectPoiHeader;
use super::project_summary::Msg as ProjectSummaryMsg;
use super::project_summary::Msg::ProjectDeleted as ProjectSummaryProjectDeleted;
use super::project_summary::Msg::ProjectItemAdded as ProjectSummaryItemAddedMsg;
use super::project_summary::Msg::ProjectUpdated as ProjectSummaryProjectUpdated;
use super::project_summary::ProjectSummary;
use super::search_view::Msg as SearchViewMsg;
use super::search_view::Msg::OpenItemFull as SearchViewOpenItemFull;
use super::search_view::Msg::SearchResultsModified as SearchViewSearchResultsModified;
use super::search_view::Msg::ShowInfoBar as SearchViewShowInfoBar;
use super::search_view::{OperationMode, SearchItemsType, SearchView};
use super::server_poi_contents::ServerItem;
use super::tooltips_overlay;
use super::tooltips_overlay::TooltipsOverlay;
use super::wintitlebar::Msg as WinTitleBarMsg;
use super::wintitlebar::WinTitleBar;
use crate::config::Config;
use crate::sql_thread::migrate_db_if_needed;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::Msg::ProjectItemSelected;
use crate::widgets::project_summary::Msg::EnvironmentChanged;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use gdk::ModifierType;
use gtk::prelude::*;
use gtk::traits::SettingsExt;
use projectpadsql::models::{EnvironmentType, Project, Server};
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

pub fn is_plaintext_key(e: &gdk::EventKey) -> bool {
    // return false if control and others were pressed
    // (then the state won't be empty)
    // could be ctrl-c on notes for instance
    // whitelist MOD2 (num lock) and LOCK (shift or caps lock)
    let mut state = e.state();
    state.remove(ModifierType::MOD2_MASK);
    state.remove(ModifierType::LOCK_MASK);
    state.is_empty()
        && e.keyval() != gdk::keys::constants::Return
        && e.keyval() != gdk::keys::constants::KP_Enter
}

const CSS_DATA: &[u8] = include_bytes!("../../resources/style.css");

type DisplayItemParams = (Project, Option<ProjectItem>, Option<ServerItem>);

#[derive(Msg)]
pub enum Msg {
    Quit,
    CloseUnlockDb,
    DbUnlockAttempted(bool),
    DbUnlocked,
    DbPrepared,
    DarkThemeToggled,
    ProjectActivated(Project),
    EnvironmentChanged(EnvironmentType),
    ProjectItemSelected(Option<ProjectItem>),
    SearchActiveChanged(bool),
    SearchTextChanged(String),
    DisplayItem(Box<DisplayItemParams>), // large enum variant, hence boxed
    KeyPress(gdk::EventKey),
    KeyRelease(gdk::EventKey),
    ProjectItemUpdated(ProjectItem),
    ProjectItemDeleted(ProjectItem),
    RequestDisplayItem(ServerItem),
    AddProject,
    ProjectListChanged,
    ProjectCountChanged(usize),
    UpdateProjectTooltip(Option<(String, i32)>),
    ShowInfoBar(String),
    HideInfobar,
    SearchResultsModified,
    OpenSingleWebsiteLink,
    ImportApplied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPoiItem {
    pub name: String,
    // TODO groups
}
pub struct Model {
    relm: relm::Relm<Win>,
    db_sender: mpsc::Sender<SqlFunc>,
    titlebar: Component<WinTitleBar>,
    is_new_db: bool,
    is_db_unlocked: bool,
    _display_item_channel: relm::Channel<DisplayItemParams>,
    display_item_sender: relm::Sender<DisplayItemParams>,
    project_add_dialog: Option<(relm::Component<ProjectAddEditDialog>, gtk::Dialog)>,
    tooltips_overlay: Component<TooltipsOverlay>,
    _db_unlock_attempted_channel: relm::Channel<bool>,
    db_unlock_attempted_sender: relm::Sender<bool>,
    _db_prepared_channel: relm::Channel<()>,
    db_prepared_sender: relm::Sender<()>,
    _project_count_channel: relm::Channel<usize>,
    project_count_sender: relm::Sender<usize>,
    unlock_db_component_dialog: Option<(gtk::Dialog, Component<UnlockDbDialog>)>,
    infobar: gtk::InfoBar,
    infobar_label: gtk::Label,
}

const CHILD_NAME_NORMAL: &str = "normal";
const CHILD_NAME_SEARCH: &str = "search";
const CHILD_NAME_WELCOME: &str = "welcome";

#[widget]
impl Widget for Win {
    fn init_view(&mut self) {
        if let Err(err) = self.load_style() {
            println!("Error loading the CSS: {}", err);
        }
        let titlebar = &self.model.titlebar;
        let overlay_widget = self.model.tooltips_overlay.widget();
        self.widgets.tooltip_overlay.add_overlay(overlay_widget);
        self.widgets
            .tooltip_overlay
            .set_overlay_pass_through(overlay_widget, true);
        overlay_widget.window().unwrap().set_pass_through(true);
        relm::connect!(titlebar@WinTitleBarMsg::SearchActiveChanged(is_active),
                               self.model.relm, Msg::SearchActiveChanged(is_active));
        relm::connect!(titlebar@WinTitleBarMsg::SearchTextChanged(ref search_text),
                               self.model.relm, Msg::SearchTextChanged(search_text.clone()));
        relm::connect!(titlebar@WinTitleBarMsg::DarkThemeToggled,
                               self.model.relm, Msg::DarkThemeToggled);
        relm::connect!(titlebar@WinTitleBarMsg::ImportApplied,
                               self.model.relm, Msg::ImportApplied);
        self.init_infobar_overlay();

        self.unlock_db();

        self.streams
            .project_poi_contents
            .emit(ProjectPoiContentsMsg::GotHeaderBarHeight(
                self.widgets
                    .infobar_overlay
                    .translate_coordinates(
                        &self
                            .widgets
                            .infobar_overlay
                            .toplevel()
                            .unwrap()
                            .upcast::<gtk::Widget>(),
                        0,
                        0,
                    )
                    .unwrap()
                    .1,
            ));
    }

    fn init_infobar_overlay(&self) {
        self.widgets
            .infobar_overlay
            .add_overlay(&self.model.infobar);
        self.widgets
            .infobar_overlay
            .set_overlay_pass_through(&self.model.infobar, true);
    }

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, bool)) -> Model {
        let (db_sender, is_new_db) = params;
        gtk::IconTheme::default()
            .unwrap()
            .add_resource_path("/icons");
        let config = Config::read_config();
        gtk::Settings::default()
            .unwrap()
            .set_gtk_application_prefer_dark_theme(config.prefer_dark_theme);
        let titlebar = relm::init::<WinTitleBar>(db_sender.clone()).expect("win title bar init");
        let tooltips_overlay = relm::init::<TooltipsOverlay>(()).expect("tooltips overlay init");

        let stream = relm.stream().clone();
        let (display_item_channel, display_item_sender) =
            relm::Channel::new(move |ch_data: DisplayItemParams| {
                stream.emit(Msg::DisplayItem(Box::new(ch_data)));
            });
        let stream2 = relm.stream().clone();
        let (db_unlock_attempted_channel, db_unlock_attempted_sender) =
            relm::Channel::new(move |val| {
                stream2.emit(Msg::DbUnlockAttempted(val));
            });
        let stream3 = relm.stream().clone();
        let (db_prepared_channel, db_prepared_sender) = relm::Channel::new(move |_| {
            stream3.emit(Msg::DbPrepared);
        });
        let stream4 = relm.stream().clone();
        let (project_count_channel, project_count_sender) = relm::Channel::new(move |count| {
            stream4.emit(Msg::ProjectCountChanged(count));
        });
        let infobar = gtk::builders::InfoBarBuilder::new()
            .revealed(false)
            .message_type(gtk::MessageType::Info)
            .valign(gtk::Align::Start)
            .build();

        let infobar_label = gtk::builders::LabelBuilder::new().label("").build();
        infobar_label.show();
        infobar.content_area().add(&infobar_label);
        infobar.show();
        Model {
            relm: relm.clone(),
            db_sender,
            is_new_db,
            is_db_unlocked: false,
            titlebar,
            tooltips_overlay,
            display_item_sender,
            _display_item_channel: display_item_channel,
            db_unlock_attempted_sender,
            _db_unlock_attempted_channel: db_unlock_attempted_channel,
            db_prepared_sender,
            _db_prepared_channel: db_prepared_channel,
            project_count_sender,
            _project_count_channel: project_count_channel,
            project_add_dialog: None,
            unlock_db_component_dialog: None,
            infobar,
            infobar_label,
        }
    }

    fn unlock_db(&mut self) {
        if let Some(pass) = keyring_helpers::get_pass_from_keyring() {
            let s = self.model.db_unlock_attempted_sender.clone();
            self.model
                .db_sender
                .send(SqlFunc::new(move |sql_conn| {
                    let unlock_success = projectpadsql::try_unlock_db(sql_conn, &pass).is_ok();
                    s.send(unlock_success).unwrap();
                }))
                .unwrap();
        } else {
            self.display_unlock_dialog();
        }
    }

    fn display_unlock_dialog(&mut self) {
        let dialog = standard_dialogs::modal_dialog(
            self.widgets.window.clone().upcast::<gtk::Widget>(),
            600,
            100,
            "Projectpad".to_string(),
        );
        relm::connect!(
            self.model.relm,
            &dialog,
            connect_delete_event(_, _),
            return (Msg::CloseUnlockDb, Inhibit(false))
        );

        let dialog_contents =
            relm::init::<UnlockDbDialog>((self.model.is_new_db, self.model.db_sender.clone()))
                .expect("error initializing the unlock db modal");
        relm::connect!(
            dialog_contents@MsgUnlockDbDlg::CheckedPassword(Ok(_)),
            self.model.relm,
            Msg::DbUnlocked
        );

        let unlock_btn = dialog
            .add_button(
                if self.model.is_new_db {
                    "Start"
                } else {
                    "Unlock"
                },
                gtk::ResponseType::Ok,
            )
            .downcast::<gtk::Button>()
            .expect("error reading the dialog save button");
        unlock_btn.style_context().add_class("suggested-action");
        dialog_contents.widget().show();
        dialog
            .content_area()
            .pack_start(dialog_contents.widget(), true, true, 0);
        let s = dialog_contents.stream();
        dialog.connect_response(move |d, r| {
            if r == gtk::ResponseType::Ok {
                s.emit(unlock_db_dlg::Msg::OkPressed);
            } else {
                d.close();
            }
        });
        self.model.unlock_db_component_dialog = Some((dialog.clone(), dialog_contents));
        dialog.show();
        unlock_btn.grab_default();
    }

    fn run_prepare_db(&self) {
        let s = self.model.db_prepared_sender.clone();

        self.model
            .db_sender
            .send(SqlFunc::new(move |db_conn| {
                migrate_db_if_needed(db_conn).unwrap();
                db_conn.batch_execute("PRAGMA foreign_keys = ON").unwrap();
                s.send(()).unwrap();
            }))
            .unwrap();
    }

    fn request_update_welcome_status(&self) {
        let s = self.model.project_count_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                s.send(
                    prj::project
                        .select(diesel::dsl::count(prj::id))
                        .first::<i64>(sql_conn)
                        .unwrap() as usize,
                )
                .unwrap()
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
            Msg::DbUnlockAttempted(true) => {
                // simpler for other components to just tie
                // to DbUnlocked. We really want to reemit not
                // make a simple function call.
                self.model.relm.stream().emit(Msg::DbUnlocked);
            }
            Msg::DbUnlockAttempted(false) => {
                self.display_unlock_dialog();
            }
            Msg::DbUnlocked => {
                self.run_prepare_db();
            }
            Msg::DbPrepared => {
                self.model.is_db_unlocked = true;
                if let Some((dialog, _)) = &self.model.unlock_db_component_dialog {
                    dialog.close();
                    self.model.unlock_db_component_dialog = None;
                }
                self.streams.project_list.emit(ProjectListMsg::DbPrepared);
                self.request_update_welcome_status();
            }
            Msg::CloseUnlockDb => {
                if !self.model.is_db_unlocked {
                    gtk::main_quit();
                }
            }
            Msg::ProjectActivated(project) => {
                self.streams
                    .project_items_list
                    .emit(ProjectItemsListMsg::ActiveProjectChanged(project.clone()));
                self.streams
                    .project_summary
                    .emit(ProjectSummaryMsg::ProjectActivated(project));
            }
            Msg::EnvironmentChanged(env) => {
                self.streams
                    .project_items_list
                    .emit(ProjectItemsListMsg::ActiveEnvironmentChanged(env));
            }
            Msg::ProjectItemSelected(pi) => {
                self.widgets
                    .normal_or_project_welcome_stack
                    .set_visible_child_name(if pi.is_some() {
                        CHILD_NAME_NORMAL
                    } else {
                        CHILD_NAME_WELCOME
                    });
                self.streams
                    .project_poi_header
                    .emit(ProjectPoiHeaderMsg::ProjectItemSelected(pi.clone()));
                self.streams
                    .project_poi_contents
                    .emit(ProjectPoiContentsMsg::ProjectItemSelected(Box::new(pi)));
            }
            Msg::SearchActiveChanged(is_active) => {
                self.widgets
                    .normal_or_search_stack
                    .set_visible_child_name(if is_active {
                        CHILD_NAME_SEARCH
                    } else {
                        CHILD_NAME_NORMAL
                    });
            }
            Msg::SearchTextChanged(search_text) => {
                self.components
                    .search_view
                    .emit(SearchViewMsg::FilterChanged(Some(search_text)));
            }
            Msg::DisplayItem(di) => {
                self.display_item(di);
            }
            Msg::RequestDisplayItem(server_item) => {
                self.request_display_item(server_item);
            }
            Msg::KeyPress(e) => {
                self.handle_keypress(e);
            }
            Msg::KeyRelease(e) => {
                if self.is_search_mode() {
                    self.streams.search_view.emit(SearchViewMsg::KeyRelease(e));
                }
            }
            Msg::ProjectItemUpdated(ref project_item) => {
                self.streams
                    .project_items_list
                    .emit(ProjectItemsListMsg::RefreshItemList(Some(
                        project_item.clone(),
                    )));
            }
            Msg::ProjectItemDeleted(ref _srv) => {
                self.streams
                    .project_items_list
                    .emit(ProjectItemsListMsg::RefreshItemList(None));
            }
            Msg::ProjectListChanged => {
                if let Some((_, dlg)) = &self.model.project_add_dialog {
                    dlg.close();
                    self.model.project_add_dialog = None;
                }
                self.streams
                    .project_list
                    .emit(ProjectListMsg::ProjectListChanged);
                self.request_update_welcome_status();
            }
            Msg::ProjectCountChanged(count) => {
                self.widgets
                    .normal_or_welcome_stack
                    .set_visible_child_name(if count > 0 {
                        CHILD_NAME_NORMAL
                    } else {
                        CHILD_NAME_WELCOME
                    });
            }
            Msg::AddProject => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets.window.clone().upcast::<gtk::Widget>(),
                    (self.model.db_sender.clone(), None, gtk::AccelGroup::new()),
                    MsgProjectAddEditDialog::OkPressed,
                    "Project",
                );
                relm::connect!(
                    component@MsgProjectAddEditDialog::ProjectUpdated(ref _project),
                    self.model.relm,
                    Msg::ProjectListChanged
                );
                self.model.project_add_dialog = Some((component, dialog.clone()));
                dialog.show();
            }
            Msg::UpdateProjectTooltip(params) => {
                self.model
                    .tooltips_overlay
                    .stream()
                    .emit(tooltips_overlay::Msg::UpdateProjectTooltip(params));
            }
            Msg::ShowInfoBar(msg) => {
                self.model.infobar_label.set_text(&msg);
                self.model.infobar.set_revealed(true);
                relm::timeout(self.model.relm.stream(), 1500, || Msg::HideInfobar);
            }
            Msg::HideInfobar => {
                self.model.infobar.set_revealed(false);
            }
            Msg::DarkThemeToggled => {
                self.streams
                    .project_list
                    .emit(ProjectListMsg::DarkThemeToggled);
            }
            Msg::SearchResultsModified => {
                // the user modified search results
                // we should refresh the main view else it
                // could show outdated contents
                self.streams.project_list.emit(ProjectListMsg::ForceReload);
            }
            Msg::OpenSingleWebsiteLink => {
                self.streams
                    .project_poi_contents
                    .emit(ProjectPoiContentsMsg::OpenSingleWebsiteLink);
            }
            Msg::ImportApplied => {
                // the user imported data, maybe new projects were added
                self.streams.project_list.emit(ProjectListMsg::ForceReload);
                self.request_update_welcome_status();
            }
        }
    }

    fn request_display_item(&self, server_item: ServerItem) {
        let s = self.model.display_item_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                use projectpadsql::schema::server::dsl as srv;
                let (server, project) = srv::server
                    .inner_join(prj::project)
                    .filter(srv::id.eq(server_item.server_id()))
                    .first::<(Server, Project)>(sql_conn)
                    .unwrap();
                s.send((
                    project,
                    Some(ProjectItem::Server(server)),
                    Some(server_item.clone()),
                ))
                .unwrap();
            }))
            .unwrap();
    }

    fn display_item(&self, di: Box<DisplayItemParams>) {
        let (project, project_item, server_item) = *di;
        self.components
            .project_list
            .emit(ProjectListMsg::ProjectSelectedFromElsewhere(project.id));
        let env = match &project_item {
            Some(ProjectItem::Server(s)) => Some(s.environment),
            Some(ProjectItem::ServerLink(s)) => Some(s.environment),
            Some(ProjectItem::ProjectNote(n)) if n.has_prod => Some(EnvironmentType::EnvProd),
            Some(ProjectItem::ProjectNote(n)) if n.has_uat => Some(EnvironmentType::EnvUat),
            Some(ProjectItem::ProjectNote(n)) if n.has_stage => Some(EnvironmentType::EnvStage),
            Some(ProjectItem::ProjectNote(n)) if n.has_dev => Some(EnvironmentType::EnvDevelopment),
            _ => None,
        };
        if let Some(e) = env {
            self.streams.project_summary.emit(
                ProjectSummaryMsg::ProjectEnvironmentSelectedFromElsewhere((project.clone(), e)),
            );
        } else {
            self.components
                .project_summary
                .emit(ProjectSummaryMsg::ProjectActivated(project.clone()));
        }
        self.streams.project_items_list.emit(
            ProjectItemsListMsg::ProjectItemSelectedFromElsewhere((project, env, project_item)),
        );
        if let Some(sitem) = server_item {
            self.streams
                .project_poi_contents
                .emit(ProjectPoiContentsMsg::ScrollToServerItem(sitem));
        }
        self.model
            .relm
            .stream()
            .emit(Msg::SearchActiveChanged(false));
        self.model
            .titlebar
            .stream()
            .emit(WinTitleBarMsg::SearchActiveChanged(false));
    }

    fn load_style(&self) -> Result<(), Box<dyn std::error::Error>> {
        let screen = self.widgets.window.screen().unwrap();
        let css = gtk::CssProvider::new();
        css.load_from_data(CSS_DATA)?;
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        Ok(())
    }

    fn is_search_mode(&self) -> bool {
        self.widgets
            .normal_or_search_stack
            .visible_child_name()
            .filter(|s| s.as_str() == CHILD_NAME_SEARCH)
            .is_some()
    }

    fn handle_keypress(&self, e: gdk::EventKey) {
        if e.keyval() == gdk::keys::constants::Escape {
            self.streams
                .project_poi_contents
                .emit(ProjectPoiContentsMsg::KeyboardEscape);
            self.model
                .relm
                .stream()
                .emit(Msg::SearchActiveChanged(false));
            self.model
                .titlebar
                .stream()
                .emit(WinTitleBarMsg::SearchActiveChanged(false));
            return;
        }
        if self.is_search_mode() {
            self.streams.search_view.emit(SearchViewMsg::KeyPress(e));
            return;
        }
        if !(e.state() & gdk::ModifierType::CONTROL_MASK).is_empty() {
            match e.keyval().to_unicode() {
                Some('y') => {
                    self.streams
                        .project_poi_header
                        .emit(ProjectPoiHeaderMsg::CopyPassword);
                }
                Some('e') => {
                    self.streams
                        .project_poi_header
                        .emit(ProjectPoiHeaderMsg::OpenLinkOrEditProjectNote);
                }
                Some('s') => {
                    self.model
                        .titlebar
                        .stream()
                        .emit(WinTitleBarMsg::SearchEnable);
                }
                Some('f') => {
                    self.streams
                        .project_poi_contents
                        .emit(ProjectPoiContentsMsg::KeyboardCtrlF);
                }
                Some('n') => {
                    self.streams
                        .project_poi_contents
                        .emit(ProjectPoiContentsMsg::KeyboardCtrlN);
                }
                Some('p') => {
                    self.streams
                        .project_poi_contents
                        .emit(ProjectPoiContentsMsg::KeyboardCtrlP);
                }
                Some('k') => {
                    self.components
                        .search_view
                        .emit(SearchViewMsg::FilterChanged(Some("".to_string())));
                    self.model
                        .titlebar
                        .emit(WinTitleBarMsg::SearchTextChangedFromElsewhere((
                            "".to_string(),
                            e,
                        )));
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::SearchActiveChanged(true));
                    self.model
                        .titlebar
                        .stream()
                        .emit(WinTitleBarMsg::EnterOrUpdateSearchProject);
                }
                Some('K') => {
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::SearchActiveChanged(true));
                    self.model
                        .titlebar
                        .stream()
                        .emit(WinTitleBarMsg::EnterOrUpdateSearchProject);
                }
                _ => {}
            }
        } else if e.keyval() == gdk::keys::constants::Tab {
        } else if e.keyval() == gdk::keys::constants::Return
            || e.keyval() == gdk::keys::constants::KP_Enter
        {
            self.streams
                .project_poi_contents
                .emit(ProjectPoiContentsMsg::KeyboardCtrlN);
        } else if let Some(k) = e.keyval().to_unicode() {
            // do nothing if control and others were pressed
            // (then the state won't be empty)
            // could be ctrl-c on notes for instance
            // whitelist MOD2 (num lock) and LOCK (shift or caps lock)
            if is_plaintext_key(&e) {
                // we don't want to trigger the global search if the
                // note search text entry is focused.
                if self
                    .widgets
                    .window
                    .focused_widget()
                    // is an entry focused?
                    .and_then(|w| w.downcast::<gtk::Entry>().ok())
                    // is it visible? (because when global search is off,
                    // the global search entry can be focused but invisible)
                    .filter(|w| w.get_visible())
                    .is_some()
                {
                    // the focused widget is a visible entry, and
                    // we're not in search mode => don't grab this
                    // key event, this is likely a note search
                    return;
                }

                self.model
                    .relm
                    .stream()
                    .emit(Msg::SearchActiveChanged(true));
                self.components
                    .search_view
                    .emit(SearchViewMsg::FilterChanged(Some(k.to_string())));
                self.model
                    .titlebar
                    .emit(WinTitleBarMsg::SearchTextChangedFromElsewhere((
                        k.to_string(),
                        e,
                    )));
            }
        }
    }

    view! {
        #[name="window"]
        gtk::Window {
            titlebar: Some(self.model.titlebar.widget()),
            default_width: 1000,
            default_height: 650,
            #[name="infobar_overlay"]
            gtk::Overlay {
                #[name="normal_or_search_stack"]
                gtk::Stack {
                    gtk::Box {
                        child: {
                            name: Some(CHILD_NAME_NORMAL)
                        },
                        #[name="project_list"]
                        ProjectList(self.model.db_sender.clone()) {
                            width_request: 60,
                            ProjectActivated((ref prj, UpdateParents::Yes)) => Msg::ProjectActivated(prj.clone()),
                            AddProject => Msg::AddProject,
                            UpdateProjectTooltip(ref nfo) => Msg::UpdateProjectTooltip(nfo.clone())
                        },
                        #[name="normal_or_welcome_stack"]
                        gtk::Stack {
                            gtk::Box {
                                child: {
                                    name: Some(CHILD_NAME_NORMAL)
                                },
                                // we use the overlay to display a tooltip with the name
                                // of the project from the project_list that the mouse
                                // currently hovers.
                                #[name="tooltip_overlay"]
                                gtk::Overlay {
                                    gtk::Box {
                                        orientation: gtk::Orientation::Vertical,
                                        #[name="project_summary"]
                                        ProjectSummary(self.model.db_sender.clone()) {
                                            EnvironmentChanged(env) => Msg::EnvironmentChanged(env),
                                            ProjectSummaryItemAddedMsg(ref pi) => Msg::ProjectItemUpdated(pi.clone()),
                                            ProjectSummaryProjectUpdated(_) => Msg::ProjectListChanged,
                                            ProjectSummaryProjectDeleted(_) => Msg::ProjectListChanged
                                        },
                                        gtk::Separator {},
                                        gtk::Box {
                                            child: {
                                                fill: true,
                                                expand: true,
                                            },
                                            gtk::Separator {
                                                orientation: gtk::Orientation::Vertical,
                                            },
                                            #[name="project_items_list"]
                                            #[style_class="sidebar"]
                                            ProjectItemsList(self.model.db_sender.clone()) {
                                                width_request: 260,
                                                child: {
                                                    fill: true,
                                                    expand: true,
                                                },
                                                ProjectItemSelected(ref pi) => Msg::ProjectItemSelected(pi.clone())
                                            },
                                        }
                                    },
                                },
                                #[name="normal_or_project_welcome_stack"]
                                gtk::Stack {
                                    child: {
                                        fill: true,
                                        expand: true,
                                    },
                                    gtk::Box {
                                        child: {
                                            name: Some(CHILD_NAME_WELCOME)
                                        },
                                        gtk::ScrolledWindow {
                                            gtk::Label {
                                                hexpand: true,
                                                margin_start: 10,
                                                margin_end: 10,
                                                margin_top: 10,
                                                xalign: 0.1,
                                                yalign: 0.1,
                                                line_wrap: true,
                                                markup: "<big><b>Empty project</b></big>\n\n\
                                                         Let's add items to this project. For that use the 'gear' icon next to \
                                                         the project name. The gear icon allows you to edit the project, but also \
                                                         to add elements to it.\n\n\
                                                         A project may contain:\n\n\
                                                         • <u>Server</u> - These are machines or virtual machines, with their own \
                                                         IP. Projectpad knows several types of servers like Application servers, \
                                                         Database, Reporting, Proxy... And a server may contain more elements, \
                                                         such as point of interests (like folders on the filesystem), websites, \
                                                         databases and so on - you'll be able to add these with the gear icon \
                                                         that'll appear next to the server name on the right of the screen;\n\n\
                                                         • <u>Point of interest</u> - These are commands to run or relevant files \
                                                         or folders. Project point of interests have to be located on your computer. If you're \
                                                         interested in point of interests on another machine then create a <tt>server</tt> for \
                                                         that machine and add a Server point of interest on that server;\n\n\
                                                         • <u>Project note</u> - Notes are markdown-formatted text containing \
                                                         free-form text. Project notes are tied to the whole project, you can \
                                                         also create server notes if they're tied to a specific server;\n\n\
                                                         • <u>Server link</u> - Sometimes a specific server is shared between \
                                                         different projects. Since we don't want to enter that server multiple \
                                                         times in projectpad, we can enter it just once and 'link' to it from \
                                                         the various projects making use of it. It's also possible to link to \
                                                         a specific group on that server."
                                            }
                                        }
                                    },
                                    gtk::Box {
                                        child: {
                                            name: Some(CHILD_NAME_NORMAL)
                                        },
                                        orientation: gtk::Orientation::Vertical,
                                        spacing: 10,
                                        #[name="project_poi_header"]
                                        ProjectPoiHeader((self.model.db_sender.clone(), None)) {
                                            ProjectPoiHeaderProjectItemRefreshMsg(ref pi) => Msg::ProjectItemUpdated(pi.clone()),
                                            ProjectPoiHeaderProjectItemDeletedMsg(ref pi) => Msg::ProjectItemDeleted(pi.clone()),
                                            ProjectPoiHeaderProjectItemUpdatedMsg(ref pi) => Msg::ProjectItemSelected(pi.clone()),
                                            ProjectPoiHeaderGotoItemMsg(ref project, ref pi) => Msg::DisplayItem(Box::new(
                                                (project.clone(), Some(pi.clone()), None))),
                                            ProjectPoiHeaderShowInfoBar(ref msg) =>
                                                Msg::ShowInfoBar(msg.clone()),
                                            ProjectPoiHeaderOpenSingleWebsiteLink => Msg::OpenSingleWebsiteLink,
                                            ProjectPoiHeaderMoveApplied(Ok((_, _, project_item_move_dlg::ProjectUpdated::Yes))) => Msg::ProjectListChanged,
                                        },
                                        #[name="project_poi_contents"]
                                        ProjectPoiContents(self.model.db_sender.clone()) {
                                            child: {
                                                fill: true,
                                                expand: true,
                                            },
                                            ProjectPoiContentsMsgRequestDisplayServerItem(ref item_info) =>
                                                Msg::RequestDisplayItem(item_info.clone()),
                                            ProjectPoiContentsMsgShowInfoBar(ref msg) =>
                                                Msg::ShowInfoBar(msg.clone()),
                                        }
                                    },
                                }
                            },
                            gtk::Box {
                                child: {
                                    name: Some(CHILD_NAME_WELCOME)
                                },
                                gtk::Label {
                                    xalign: 0.1,
                                    yalign: 0.1,
                                    line_wrap: true,
                                    markup: "<big><b>Welcome to Projectpad!</b></big>\n\n\nTo get started, you must create your first project. Use the <tt>+</tt> button on the top-left.\n\n\
                                             Projects get subdivided in environments, and specifically:\n\n\
                                             • <u>Prod</u> - the production environment;\n\
                                             • <u>Uat</u> - User Acceptance Testing, an environment used by the customer, which is not Prod;\n\
                                             • <u>Stg</u> - Staging, the last testing environment before showing the product to the customer;\n\
                                             • <u>Dev</u> - the development environment.\n\n\
                                             A project should have at least one environment. If unsure, use Prod.\n\n\
                                             Once you have a project and environments for it, you'll be able to manage notes, points of interests, servers, and so on, for that project,\n\
                                             for each environment."
                                }
                            }
                        }
                    },
                    #[name="search_view"]
                    SearchView((self.model.db_sender.clone(), None,
                                SearchItemsType::All, OperationMode::ItemActions, None, None)) {
                        child: {
                            name: Some(CHILD_NAME_SEARCH)
                        },
                        SearchViewOpenItemFull(ref item) => Msg::DisplayItem(Box::new((**item).clone())),
                        SearchViewSearchResultsModified => Msg::SearchResultsModified,
                        SearchViewShowInfoBar(ref msg) => Msg::ShowInfoBar(msg.clone()),
                    }
                },
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
            key_press_event(_, event) => (Msg::KeyPress(event.clone()), Inhibit(false)),
            key_release_event(_, event) => (Msg::KeyRelease(event.clone()), Inhibit(false)),
        }
    }
}
