use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::widgets::project_items::server;
use crate::widgets::{project_item_model::ProjectItemType, project_items::note};
use projectpadsql::get_project_group_names;
use projectpadsql::models::{Project, ProjectNote, ProjectPointOfInterest, Server, ServerLink};

use super::project_items::note::{Note, NoteInfo};
use super::project_items::server_items::server_item_copy_dialog;
use super::project_items::{project_poi, server_link};
use super::{project_items::common, search::search_item_model::SearchItemType};

use diesel::prelude::*;

mod imp {
    use std::{cell::Cell, sync::OnceLock};

    use super::*;
    use glib::subclass::Signal;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectItem)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item.ui")]
    pub struct ProjectItem {
        #[template_child]
        pub clamp: TemplateChild<adw::Clamp>,

        #[template_child]
        pub project_item: TemplateChild<adw::Bin>,

        // these properties are meant to be set all at once
        // using GObjectExt.set_properties START
        #[property(get, set)]
        pub item_id: Cell<i32>,

        #[property(get, set)]
        pub project_item_type: Cell<u8>,

        #[property(get, set)]
        pub sub_item_id: Cell<i32>,
        // these properties are meant to be set all at once
        // using GObjectExt.set_properties END
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItem {
        const NAME: &'static str = "ProjectItem";
        type ParentType = adw::Bin;
        type Type = super::ProjectItem;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItem {
        fn constructed(&self) {
            let _ = self
                .obj()
                .connect_item_id_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.refresh_item();
                });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("request-scroll")
                    .param_types([f32::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ProjectItem {}

    impl adw::subclass::prelude::BinImpl for ProjectItem {}
}

glib::wrapper! {
    pub struct ProjectItem(ObjectSubclass<imp::ProjectItem>)
        @extends gtk::Widget, adw::Bin;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetMode {
    Show,
    Edit,
}

impl WidgetMode {
    pub fn get_edit_mode(&self) -> bool {
        !matches!(&self, WidgetMode::Show)
    }
}

impl ProjectItem {
    pub fn refresh_item(&self) {
        let item_id = self.imp().item_id.get();

        if item_id == -1 {
            // empty project item
            let label = gtk::Label::builder().label(
                              "A project may contain:\n\n\
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
                ).wrap(true).use_markup(true).build();
            let status_page = adw::StatusPage::builder()
                .icon_name("cube")
                .title("Empty project")
                .description(
                    "To add items to this project, use the '+' icon at the \
                              bottom of the sidebar.",
                )
                .child(&label)
                .build();
            self.imp().project_item.set_child(Some(&status_page));
        } else {
            let sub_item_id = Some(self.imp().sub_item_id.get());
            let item_type = ProjectItemType::from_repr(self.imp().project_item_type.get());
            // TODO receive the item type besides the item_id and switch on item type here
            // also possibly receive the ProjectItem, telling me much more than the id
            let db_sender = common::app().get_sql_channel();

            // reset the scroll, who knows what we were displaying before
            self.emit_by_name::<()>("request-scroll", &[&0f32]);

            match item_type {
                Some(ProjectItemType::Server) => {
                    self.imp().clamp.set_maximum_size(750);
                    super::project_items::server::load_and_display_server(
                        &self.imp().project_item,
                        db_sender,
                        item_id,
                        sub_item_id,
                        self,
                    )
                }
                Some(ProjectItemType::ProjectNote) => {
                    self.imp().clamp.set_maximum_size(1100);
                    let note = note::Note::new();
                    // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
                    // it feels even worse (would like not to rebuild the widget every time...)
                    // move to set_properties with freeze_notify
                    note.set_project_note_id(item_id);
                    note.set_edit_mode(false);
                    self.imp().project_item.set_child(Some(
                        // &note::Note::new().set_note_id(&glib::Value::from(item_id)),
                        &note,
                    ));
                    //     db_sender,
                    //     item_id,
                    //     widget_mode,
                    // )
                }
                Some(ProjectItemType::ProjectPointOfInterest) => {
                    self.imp().clamp.set_maximum_size(750);
                    super::project_items::project_poi::load_and_display_project_poi(
                        &self.imp().project_item,
                        db_sender,
                        item_id,
                    )
                }
                Some(ProjectItemType::ServerLink) => {
                    self.imp().clamp.set_maximum_size(750);
                    super::project_items::server_link::load_and_display_server_link(
                        &self.imp().project_item,
                        db_sender,
                        item_id,
                        self,
                    )
                }
                None => panic!(),
            }
        }
    }

    pub fn trigger_item_edit(&self) {
        let w = common::main_win();
        let project_state = glib::VariantDict::new(w.action_state("select-project-item").as_ref());
        let search_item_type = project_state
            .lookup::<Option<u8>>("item_type")
            .unwrap()
            .and_then(std::convert::identity)
            .and_then(SearchItemType::from_repr);

        let project_id = project_state.lookup::<i32>("project_id").unwrap().unwrap();

        let item_id = project_state
            .lookup::<Option<i32>>("item_id")
            .unwrap()
            .unwrap();

        match search_item_type {
            Some(SearchItemType::Server) => {
                self.trigger_edit_server(project_id, item_id.unwrap());
            }
            Some(SearchItemType::ProjectNote) => {
                self.trigger_edit_project_note(project_id, item_id.unwrap());
            }
            Some(SearchItemType::ProjectPointOfInterest) => {
                self.trigger_edit_project_poi(project_id, item_id.unwrap());
            }
            Some(SearchItemType::ServerLink) => {
                self.trigger_edit_server_link(project_id, item_id.unwrap());
            }
            _ => {}
        }
    }

    fn trigger_edit_server_link(&self, project_id: i32, server_link_id: i32) {
        let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::server_link::dsl as lnk;
            let project = prj::project
                .filter(prj::id.eq(project_id))
                .first::<Project>(sql_conn)
                .unwrap();
            let project_group_names = get_project_group_names(sql_conn, project_id);
            let server_link = lnk::server_link
                .filter(lnk::id.eq(server_link_id))
                .first::<ServerLink>(sql_conn)
                .unwrap();
            (project.allowed_envs(), project_group_names, server_link)
        }));

        glib::spawn_future_local(async move {
            let (ae, pgn, server_link) = recv.recv().await.unwrap();
            server_link::open_server_link_edit(&pgn, &ae, &server_link);
        });
    }

    fn trigger_edit_project_poi(&self, project_id: i32, poi_id: i32) {
        let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
            let project = prj::project
                .filter(prj::id.eq(project_id))
                .first::<Project>(sql_conn)
                .unwrap();
            let project_group_names = get_project_group_names(sql_conn, project_id);
            let poi = ppoi::project_point_of_interest
                .filter(ppoi::id.eq(poi_id))
                .first::<ProjectPointOfInterest>(sql_conn)
                .unwrap();
            (project.allowed_envs(), project_group_names, poi)
        }));

        glib::spawn_future_local(async move {
            let (ae, pgn, poi) = recv.recv().await.unwrap();
            project_poi::open_project_poi_edit(&pgn, &ae, &poi);
        });
    }

    fn trigger_edit_server(&self, project_id: i32, server_id: i32) {
        let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::server::dsl as srv;
            let project = prj::project
                .filter(prj::id.eq(project_id))
                .first::<Project>(sql_conn)
                .unwrap();
            let project_group_names = get_project_group_names(sql_conn, project_id);
            let server = srv::server
                .filter(srv::id.eq(server_id))
                .first::<Server>(sql_conn)
                .unwrap();
            (project, project_group_names, server)
        }));

        glib::spawn_future_local(async move {
            let (project, pgn, server) = recv.recv().await.unwrap();
            server::open_server_edit(&project, &pgn, &server);
        });
    }

    fn trigger_edit_project_note(&self, project_id: i32, item_id: i32) {
        if let Some(pi_child) = self.imp().project_item.child() {
            if let Ok(note) = pi_child.downcast::<Note>() {
                let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                    use projectpadsql::schema::project::dsl as prj;
                    use projectpadsql::schema::project_note::dsl as pnote;

                    let project = prj::project
                        .filter(prj::id.eq(project_id))
                        .first::<Project>(sql_conn)
                        .unwrap();

                    let prj_note = pnote::project_note
                        .filter(pnote::id.eq(item_id))
                        .first::<ProjectNote>(sql_conn)
                        .unwrap();
                    (
                        get_project_group_names(sql_conn, project_id),
                        project.allowed_envs(),
                        prj_note,
                    )
                }));

                glib::spawn_future_local(async move {
                    let (pgn, ae, prj_note) = recv.recv().await.unwrap();
                    note.trigger_edit_server_note(
                        &pgn,
                        &ae,
                        &NoteInfo::from_project_note(&prj_note),
                    );
                });
            }
        }
    }

    pub fn trigger_copy_visible_pass(&self) {
        let w = common::main_win();
        let project_state = glib::VariantDict::new(w.action_state("select-project-item").as_ref());
        let search_item_type = project_state
            .lookup::<Option<u8>>("item_type")
            .unwrap()
            .and_then(std::convert::identity)
            .and_then(SearchItemType::from_repr);

        let m_item_id = project_state
            .lookup::<Option<i32>>("item_id")
            .unwrap()
            .unwrap();

        if let Some(item_id) = m_item_id {
            if let Some(SearchItemType::Server) = search_item_type {
                let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                    use projectpadsql::schema::server::dsl as srv;
                    use projectpadsql::schema::server_database::dsl as db;
                    use projectpadsql::schema::server_extra_user_account::dsl as usr;
                    use projectpadsql::schema::server_website::dsl as www;
                    let server = srv::server
                        .filter(srv::id.eq(item_id))
                        .first::<Server>(sql_conn)
                        .unwrap();

                    let server_www_passwords = www::server_website
                        .filter(www::server_id.eq(item_id).and(www::password.is_not_null()))
                        .select((www::desc, www::password))
                        .load::<(String, String)>(sql_conn)
                        .unwrap();

                    let server_db_passwords = db::server_database
                        .filter(db::server_id.eq(item_id).and(db::password.is_not_null()))
                        .select((db::desc, db::password))
                        .load::<(String, String)>(sql_conn)
                        .unwrap();

                    let server_usr_passwords = usr::server_extra_user_account
                        .filter(usr::server_id.eq(item_id).and(usr::password.is_not_null()))
                        .select((usr::desc, usr::password))
                        .load::<(String, String)>(sql_conn)
                        .unwrap();

                    (
                        server.password,
                        server_www_passwords,
                        server_db_passwords,
                        server_usr_passwords,
                    )
                }));
                glib::spawn_future_local(async move {
                    let (srv_pass, www_passes, db_passes, usr_passes) = recv.recv().await.unwrap();
                    let pass_count = if srv_pass.is_empty() { 0 } else { 1 }
                        + www_passes.len()
                        + db_passes.len()
                        + usr_passes.len();
                    if pass_count == 0 {
                        return;
                    }
                    if pass_count == 1 {
                        if !srv_pass.is_empty() {
                            common::copy_to_clipboard(&srv_pass);
                        }
                        if www_passes.len() == 1 {
                            let pass_info = &www_passes[0];
                            common::copy_to_clipboard_msg(
                                &pass_info.1,
                                &format!("Copied website password: {}", pass_info.0),
                            );
                        }
                        if db_passes.len() == 1 {
                            let pass_info = &db_passes[0];
                            common::copy_to_clipboard_msg(
                                &pass_info.1,
                                &format!("Copied database password: {}", pass_info.0),
                            );
                        }
                        if usr_passes.len() == 1 {
                            let pass_info = &usr_passes[0];
                            common::copy_to_clipboard_msg(
                                &pass_info.1,
                                &format!("Copied extra user password: {}", pass_info.0),
                            );
                        }
                        return;
                    }
                    // more than one password to copy, ask the user which one
                    server_item_copy_dialog::display_copy_server_password_dialog(
                        &srv_pass,
                        &www_passes,
                        &db_passes,
                        &usr_passes,
                    );
                });
            }
        }
    }
}
