use std::str::FromStr;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::mpsc,
};

use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use itertools::Itertools;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerAccessType, ServerLink, ServerType,
};

use crate::{
    app::ProjectpadApplication,
    sql_thread::SqlFunc,
    widgets::project_items::{self, server},
};

use super::project_items::project_item_header_edit::ProjectItemHeaderEdit;
use super::project_items::server_view_edit::ServerViewEdit;
use super::{
    project_item::WidgetMode,
    project_item_list_model::ProjectItemListModel,
    project_item_model::{ProjectItemModel, ProjectItemType},
    project_items::{
        common::EnvOrEnvs,
        note::{Note, NoteInfo},
        project_poi::project_poi_contents,
        server::server_contents,
    },
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ProjectItem {
    Server(Server),
    ServerLink(ServerLink),
    ProjectNote(ProjectNote),
    ProjectPointOfInterest(ProjectPointOfInterest),
}

// https://gtk-rs.org/gtk4-rs/stable/latest/book/todo_1.html
// https://gitlab.com/news-flash/news_flash_gtk/-/blob/master/src/article_list/models/article.rs?ref_type=heads
mod imp {
    use std::{cell::Cell, sync::OnceLock};

    use glib::subclass::Signal;
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectItemList)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_list.ui"
    )]
    pub struct ProjectItemList {
        #[template_child]
        pub project_item_list: TemplateChild<gtk::ListView>,

        #[template_child]
        pub add_project_item: TemplateChild<gtk::Button>,

        #[property(get, set)]
        edit_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemList {
        const NAME: &'static str = "ProjectItemList";
        type ParentType = gtk::Box;
        type Type = super::ProjectItemList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItemList {
        fn constructed(&self) {
            self.obj().init_list();

            self.add_project_item
                .connect_clicked(|_| super::ProjectItemList::display_add_project_item_dialog());
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("activate-item")
                    // item id + project_item_type + optionally server item + title
                    .param_types([
                        i32::static_type(),
                        u8::static_type(),
                        i32::static_type(),
                        String::static_type(),
                    ])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ProjectItemList {}

    impl BoxImpl for ProjectItemList {}
}

glib::wrapper! {
    pub struct ProjectItemList(ObjectSubclass<imp::ProjectItemList>)
        @extends gtk::Widget, gtk::Box;
}

impl ProjectItemList {
    pub fn init_list(&self) {
        self.imp().project_item_list.set_factory(Some(
            &gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_row.ui",
            ),
        ));
        self.imp().project_item_list.set_header_factory(Some(
            &gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_header_row.ui",
            ),
        ));
    }

    pub fn set_project_items(
        &self,
        project: &Project,
        project_items: &[ProjectItem],
        group_start_indices: HashMap<i32, String>,
        selected_item: Option<i32>,
        selected_sub_item: Option<i32>,
    ) {
        let mut list_store = ProjectItemListModel::new();
        list_store.set_group_start_indices(project_items.len(), group_start_indices);
        let mut idx = 0;
        let mut selected_index = None;

        for project_item in project_items {
            let item_model = Self::get_item_model(project, project_item);
            list_store.append(&item_model);
            if selected_item == Some(item_model.property("id")) {
                selected_index = Some(idx);
            }
            idx += 1;
        }
        if selected_item.is_none() && !project_items.is_empty() {
            if let Some(first_item) = list_store.item(0) {
                // None == select first item (if any)
                selected_index = Some(0);
                self.emit_by_name::<()>(
                    "activate-item",
                    &[
                        &first_item.property_value("id").get::<i32>().unwrap(),
                        &first_item
                            .property_value("project-item-type")
                            .get::<u8>()
                            .unwrap(),
                        &selected_sub_item.unwrap_or(-1),
                        &first_item.property_value("title").get::<String>().unwrap(),
                    ],
                );
            }
        }
        if let Some(s_model) = self.imp().project_item_list.model() {
            let _sel_model = s_model.downcast::<gtk::SingleSelection>().unwrap();
            _sel_model.set_model(Some(&list_store));
        } else {
            let selection_model = gtk::SingleSelection::new(Some(list_store.clone()));
            self.imp()
                .project_item_list
                .set_model(Some(&selection_model));
        }

        if let Some(idx) = selected_index {
            self.imp()
                .project_item_list
                .scroll_to(idx, gtk::ListScrollFlags::SELECT, None);

            if let Some(list_item) = list_store.item(idx) {
                self.emit_by_name::<()>(
                    "activate-item",
                    &[
                        &selected_item.unwrap_or_else(|| {
                            list_store
                                .item(0)
                                .unwrap()
                                .property_value("id")
                                .get::<i32>()
                                .unwrap()
                        }),
                        &list_item
                            .property_value("project-item-type")
                            .get::<u8>()
                            .unwrap(),
                        &selected_sub_item.unwrap_or(-1),
                        &list_item.property_value("title").get::<String>().unwrap(),
                    ],
                );
            }
        }

        self.imp()
            .project_item_list
            .model()
            .unwrap()
            .connect_selection_changed(
                glib::clone!(@weak self as s  => move |sel_model, _idx, _items_count| {
                let idx = sel_model
                    .downcast_ref::<gtk::SingleSelection>()
                    .unwrap()
                    .selected();
                let model = sel_model.item(idx).unwrap();
                s.emit_by_name::<()>(
                    "activate-item",
                    &[
                        &model.property_value("id").get::<i32>().unwrap(),
                        &model
                            .property_value("project-item-type")
                            .get::<u8>()
                            .unwrap(),
                            &glib::Value::from(-1),
                        &model.property_value("title").get::<String>().unwrap(),
                    ],
                )
                }),
            );
    }

    fn get_item_model(project: &Project, project_item: &ProjectItem) -> ProjectItemModel {
        match project_item {
            ProjectItem::Server(srv) => ProjectItemModel::new(
                project,
                srv.id,
                ProjectItemType::Server,
                srv.desc.clone(),
                HashSet::from([srv.environment]),
                srv.group_name.clone(),
            ),
            //     markup: if srv.is_retired {
            //         format!("<i>{}</i>", glib::markup_escape_text(&srv.desc))
            //     } else {
            //         glib::markup_escape_text(&srv.desc).to_string()
            //     },
            //     group_name: srv.group_name.as_ref().cloned(),
            //     icon: match (srv.server_type, srv.access_type) {
            //         (ServerType::SrvDatabase, _) => Icon::DATABASE,
            //         (ServerType::SrvReporting, _) => Icon::REPORTING,
            //         (ServerType::SrvMonitoring, _) => Icon::MONITORING,
            //         (ServerType::SrvHttpOrProxy, _) => Icon::HTTP,
            //         (_, ServerAccessType::SrvAccessRdp) => Icon::WINDOWS,
            //         (_, _) => Icon::SERVER,
            //     },
            // },
            ProjectItem::ServerLink(link) => ProjectItemModel::new(
                project,
                link.id,
                ProjectItemType::ServerLink,
                link.desc.clone(),
                HashSet::from([link.environment]),
                link.group_name.clone(),
            ),
            //     markup: glib::markup_escape_text(&link.desc).to_string(),
            //     group_name: link.group_name.as_ref().cloned(),
            //     icon: Icon::SERVER_LINK,
            // },
            ProjectItem::ProjectNote(note) => ProjectItemModel::new(
                project,
                note.id,
                ProjectItemType::ProjectNote,
                note.title.clone(),
                Note::get_envs(note),
                note.group_name.clone(),
            ),
            //     markup: glib::markup_escape_text(&note.title).to_string(),
            //     group_name: note.group_name.as_ref().cloned(),
            //     icon: Icon::NOTE,
            // },
            ProjectItem::ProjectPointOfInterest(poi) => ProjectItemModel::new(
                project,
                poi.id,
                ProjectItemType::ProjectPointOfInterest,
                poi.desc.clone(),
                HashSet::new(),
                poi.group_name.clone(),
            ), // markup: glib::markup_escape_text(&poi.desc).to_string(),
               // group_name: poi.group_name.as_ref().cloned(),
               // icon: match poi.interest_type {
               //     InterestType::PoiLogFile => Icon::LOG_FILE,
               //     InterestType::PoiConfigFile => Icon::CONFIG_FILE,
               //     InterestType::PoiApplication => Icon::COG,
               //     InterestType::PoiCommandToRun => Icon::TERMINAL,
               //     InterestType::PoiCommandTerminal => Icon::TERMINAL,
               //     InterestType::PoiBackupArchive => Icon::ARCHIVE,
               // },
        }
    }

    pub fn fetch_project_items(
        &mut self,
        db_sender: &mpsc::Sender<SqlFunc>,
        project: Project,
        selected_item: Option<i32>,
        selected_sub_item: Option<i32>,
    ) {
        let (sender, receiver) = async_channel::bounded(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                let (servers, lsrvs, prj_notes, prj_pois) =
                    Self::fetch_project_items_sql(sql_conn, Some(project.id));

                let mut group_names: BTreeSet<&String> = servers
                    .iter()
                    .filter_map(|s| s.group_name.as_ref())
                    .collect();
                group_names.extend(lsrvs.iter().filter_map(|s| s.group_name.as_ref()));
                group_names.extend(prj_notes.iter().filter_map(|s| s.group_name.as_ref()));
                group_names.extend(prj_pois.iter().filter_map(|s| s.group_name.as_ref()));
                let group_names: BTreeSet<String> =
                    group_names.iter().map(|s| s.to_string()).collect();

                let mut servers_iter = servers.into_iter();
                let mut lsrvs_iter = lsrvs.into_iter();
                let mut prj_notes_iter = prj_notes.into_iter();
                let mut prj_pois_iter = prj_pois.into_iter();

                let mut items = Vec::new();
                let mut group_start_indices = HashMap::new();
                // this code relies on the sort order from the SQL query
                // to be the same as the one we process the results in.
                // notably we must have the nulls (no group) first.
                Self::add_items(
                    &mut items,
                    &mut servers_iter,
                    &mut lsrvs_iter,
                    &mut prj_notes_iter,
                    &mut prj_pois_iter,
                    None,
                );
                for group_name in group_names {
                    group_start_indices.insert(items.len() as i32, group_name.clone());
                    Self::add_items(
                        &mut items,
                        &mut servers_iter,
                        &mut lsrvs_iter,
                        &mut prj_notes_iter,
                        &mut prj_pois_iter,
                        Some(group_name),
                    );
                }
                sender.send_blocking((items, group_start_indices)).unwrap();
            }))
            .unwrap();
        let s = self.clone();
        glib::spawn_future_local(async move {
            let (items, group_start_indices) = receiver.recv().await.unwrap();
            s.set_project_items(
                &project,
                &items,
                group_start_indices,
                selected_item,
                selected_sub_item,
            );
        });
    }

    fn add_items(
        items: &mut Vec<ProjectItem>,
        servers: &mut (impl Iterator<Item = Server> + Clone),
        lsrvs: &mut (impl Iterator<Item = ServerLink> + Clone),
        prj_notes: &mut (impl Iterator<Item = ProjectNote> + Clone),
        prj_pois: &mut (impl Iterator<Item = ProjectPointOfInterest> + Clone),
        group_name: Option<String>,
    ) {
        items.extend(
            servers
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::Server),
        );
        items.extend(
            lsrvs
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ServerLink),
        );
        items.extend(
            prj_notes
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectNote),
        );
        items.extend(
            prj_pois
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectPointOfInterest),
        );
    }

    fn fetch_project_items_sql(
        sql_conn: &mut diesel::SqliteConnection,
        cur_project_id: Option<i32>,
    ) -> (
        Vec<Server>,
        Vec<ServerLink>,
        Vec<ProjectNote>,
        Vec<ProjectPointOfInterest>,
    ) {
        use projectpadsql::schema::project_note::dsl as pnt;
        use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
        use projectpadsql::schema::server::dsl as srv;
        use projectpadsql::schema::server_link::dsl as lsrv;
        match cur_project_id {
            Some(pid) => {
                let srvs = srv::server
                    .filter(
                        srv::project_id.eq(pid), /*.and(srv::environment.eq(env))*/
                    )
                    .order((srv::group_name.asc(), srv::desc.asc()))
                    .load::<Server>(sql_conn)
                    .unwrap();
                let lsrvs = lsrv::server_link
                    .filter(
                        lsrv::project_id.eq(pid), /*.and(lsrv::environment.eq(env))*/
                    )
                    .order((lsrv::group_name.asc(), lsrv::desc.asc()))
                    .load::<ServerLink>(sql_conn)
                    .unwrap();
                let prj_query = pnt::project_note
                    .filter(pnt::project_id.eq(pid))
                    .into_boxed();
                // prj_query = match env {
                //     EnvironmentType::EnvProd => prj_query.filter(pnt::has_prod.eq(true)),
                //     EnvironmentType::EnvUat => prj_query.filter(pnt::has_uat.eq(true)),
                //     EnvironmentType::EnvStage => prj_query.filter(pnt::has_stage.eq(true)),
                //     EnvironmentType::EnvDevelopment => prj_query.filter(pnt::has_dev.eq(true)),
                // };
                let prj_notes = prj_query
                    .order((pnt::group_name.asc(), pnt::title.asc()))
                    .load::<ProjectNote>(sql_conn)
                    .unwrap();
                let prj_pois = ppoi::project_point_of_interest
                    .filter(ppoi::project_id.eq(pid))
                    .order((ppoi::group_name.asc(), ppoi::desc.asc()))
                    .load::<ProjectPointOfInterest>(sql_conn)
                    .unwrap();
                (srvs, lsrvs, prj_notes, prj_pois)
            }
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        }
    }

    fn create_project_item_box(
        icon_name: &'static str,
        title: &'static str,
        subtitle: &'static str,
    ) -> gtk::Box {
        let btn_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        btn_vbox.append(
            &gtk::Label::builder()
                .css_classes(["header"])
                .label(title)
                .halign(gtk::Align::Start)
                .build(),
        );
        btn_vbox.append(
            &gtk::Label::builder()
                .css_classes(["dim-label"])
                .label(subtitle)
                .halign(gtk::Align::Start)
                .build(),
        );

        let btn_hbox = gtk::Box::builder().spacing(10).build();
        btn_hbox.append(
            &gtk::Image::builder()
                .icon_name(icon_name)
                .icon_size(gtk::IconSize::Large)
                .build(),
        );
        btn_hbox.append(&btn_vbox);
        btn_hbox
    }

    // TODO for the love of god, split that function
    fn display_add_project_item_dialog() {
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header_bar = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(false)
            .build();

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        header_bar.pack_start(&cancel_btn);
        vbox.append(&header_bar);

        let cbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(15)
            .margin_start(15)
            .margin_end(15)
            .margin_bottom(15)
            .spacing(10)
            .build();

        let server_btn = gtk::Button::builder()
            .child(&Self::create_project_item_box(
                "server",
                "Add server",
                "machines or virtual machines, with their own IP.",
            ))
            .build();
        cbox.append(&server_btn);

        let poi_btn = gtk::Button::builder()
            .child(&Self::create_project_item_box(
                "cube",
                "Add point of interest",
                "commands to run or relevant files or folders.",
            ))
            .build();
        cbox.append(&poi_btn);

        let note_btn = gtk::Button::builder()
            .child(&Self::create_project_item_box(
                "clipboard",
                "Add project note",
                "markdown-formatted text containing free-form text.",
            ))
            .build();
        cbox.append(&note_btn);

        cbox.append(
            &gtk::Button::builder()
                .child(&Self::create_project_item_box(
                    "link",
                    "Add server link",
                    "when a server is shared, we can enter it just once and 'link' to it.",
                ))
                .build(),
        );

        let stack = gtk::Stack::builder().build();
        stack.add_child(&cbox);
        vbox.append(&stack);

        let dialog = adw::Dialog::builder()
            .title("Add project item")
            .child(&vbox)
            .build();

        let s = stack.clone();
        let dlg = dialog.clone();
        let (_, header_edit, server_contents_child, server_view_edit) =
            server_contents(&Server::default(), &[], WidgetMode::Edit);
        let hb = header_bar.clone();
        let he = header_edit.unwrap().clone();
        server_btn.connect_clicked(move |_| {
            Self::prepare_add_server_dlg(
                &dlg,
                &s,
                &hb,
                &he,
                &server_view_edit,
                &server_contents_child,
            );
        });

        let s = stack.clone();
        let dlg = dialog.clone();
        poi_btn.connect_clicked(move |_| Self::prepare_add_project_poi_dlg(&dlg, &s));

        let s = stack.clone();
        let dlg = dialog.clone();
        note_btn.connect_clicked(move |_| {
            let note = Note::new();
            let note_info = {
                let mut n = NoteInfo::default();
                n.env = EnvOrEnvs::Envs(HashSet::new());
                n.display_header = true;
                n
            };
            dlg.set_title("Add Project Note");
            dlg.set_content_width(6000);
            dlg.set_content_height(6000);
            let (_, dlg_child, note_header) = note.note_contents(note_info, WidgetMode::Edit);
            dlg_child.set_margin_start(30);
            dlg_child.set_margin_end(30);
            s.add_named(&dlg_child, Some("second"));
            s.set_visible_child_name("second");

            let save_btn = gtk::Button::builder()
                .label("Save")
                .css_classes(["suggested-action"])
                .build();
            let d = dlg.clone();
            save_btn.connect_clicked(move |_| {
                dbg!(note_header.as_ref().unwrap().title());
                let receiver = Note::save_project_note(
                    note.imp().text_edit.borrow().as_ref().unwrap(),
                    note_header.as_ref().unwrap(),
                    None,
                );
                let d = d.clone();
                glib::spawn_future_local(async move {
                    let project_note_after_result = receiver.recv().await.unwrap();
                    d.close();

                    if let Ok(note) = project_note_after_result {
                        Self::display_project_item(note.id, ProjectItemType::ProjectNote);
                    }
                });
            });
            header_bar.pack_end(&save_btn);
        });

        // TODO need to add server links too

        let dlg = dialog.clone();
        cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
            dlg.close();
        });

        let app = gio::Application::default()
            .and_downcast::<ProjectpadApplication>()
            .unwrap();
        dialog.present(&app.active_window().unwrap());
    }

    fn prepare_add_server_dlg(
        dlg: &adw::Dialog,
        s: &gtk::Stack,
        hb: &adw::HeaderBar,
        he: &ProjectItemHeaderEdit,
        server_view_edit: &ServerViewEdit,
        server_contents_child: &gtk::Box,
    ) {
        dlg.set_title("Add Server");
        dlg.set_content_width(600);
        dlg.set_content_height(600);
        s.add_named(
            &adw::Clamp::builder()
                .margin_top(10)
                .child(server_contents_child)
                .build(),
            Some("second"),
        );
        s.set_visible_child_name("second");

        let save_btn = gtk::Button::builder()
            .label("Save")
            .css_classes(["suggested-action"])
            .build();
        let d = dlg.clone();
        let server_view_edit = server_view_edit.clone();
        let he = he.clone();
        save_btn.connect_clicked(move |_| {
            dbg!(server_view_edit.property::<String>("ip"));
            let receiver = server::save_server(
                None,
                he.single_env(),
                server_view_edit.property("is_retired"),
                he.property("title"),
                server_view_edit.property("ip"),
                server_view_edit.property("username"),
                server_view_edit.property("password"),
                server_view_edit.property("text"),
                ServerType::from_str(&server_view_edit.property::<String>("server_type")).unwrap(),
                ServerAccessType::from_str(&server_view_edit.property::<String>("access_type"))
                    .unwrap(),
            );
            let d = d.clone();
            glib::spawn_future_local(async move {
                let server_after_result = receiver.recv().await.unwrap();
                d.close();

                if let Ok(server) = server_after_result {
                    Self::display_project_item(server.id, ProjectItemType::Server);
                }
            });
        });
        hb.pack_end(&save_btn);
    }

    fn prepare_add_project_poi_dlg(dlg: &adw::Dialog, s: &gtk::Stack) {
        dlg.set_title("Add Project POI");
        dlg.set_content_width(600);
        dlg.set_content_height(600);

        let vbox = gtk::Box::builder().build();

        let (_, poi_box) =
            project_poi_contents(&ProjectPointOfInterest::default(), &[], WidgetMode::Edit);

        vbox.append(&poi_box);

        s.add_named(
            &adw::Clamp::builder().margin_top(10).child(&vbox).build(),
            Some("second"),
        );
        s.set_visible_child_name("second");
    }

    fn display_project_item(project_item_id: i32, project_item_type: ProjectItemType) {
        let app = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<ProjectpadApplication>()
            .unwrap();

        let w = app.imp().window.get().unwrap().upgrade().unwrap();
        let select_project_variant = glib::VariantDict::new(None);
        select_project_variant.insert("project_id", app.project_id().unwrap());
        select_project_variant.insert("item_id", Some(project_item_id));
        select_project_variant.insert("item_type", Some(project_item_type as u8));
        select_project_variant.insert("search_item_type", None::<u8>);
        w.change_action_state("select-project-item", &dbg!(select_project_variant.end()));
    }
}
