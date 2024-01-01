use std::collections::{BTreeSet, HashMap};

use diesel::prelude::*;
use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use itertools::Itertools;
use projectpadsql::models::{
    ServerDatabase, ServerExtraUserAccount, ServerNote, ServerPointOfInterest, ServerWebsite,
};

use crate::app::ProjectpadApplication;
use crate::sql_thread::SqlFunc;

mod imp {
    use std::cell::Cell;

    use super::*;
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
        pub project_item: TemplateChild<adw::Bin>,

        #[property(get, set)]
        edit_mode: Cell<bool>,

        #[property(get, set)]
        pub item_id: Cell<i32>,
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
            //     self.obj().init_list();
            let _ = self
                .obj()
                .connect_edit_mode_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.display_item();
                });
            let _ = self
                .obj()
                .connect_item_id_notify(|project_item: &super::ProjectItem| {
                    // println!("edit mode changed: {}", project_item.edit_mode());
                    project_item.display_item();
                });
        }
    }

    impl WidgetImpl for ProjectItem {}

    impl adw::subclass::prelude::BinImpl for ProjectItem {}
}

glib::wrapper! {
    pub struct ProjectItem(ObjectSubclass<imp::ProjectItem>)
        @extends gtk::Widget, adw::Bin;
}

// TODO server specific
#[derive(Clone, Debug)]
pub enum ServerItem {
    Website(ServerWebsite),
    PointOfInterest(ServerPointOfInterest),
    Note(ServerNote),
    ExtraUserAccount(ServerExtraUserAccount),
    Database(ServerDatabase),
}

impl ServerItem {
    pub fn group_name(&self) -> Option<&str> {
        match self {
            ServerItem::Website(w) => w.group_name.as_deref(),
            ServerItem::PointOfInterest(p) => p.group_name.as_deref(),
            ServerItem::Note(n) => n.group_name.as_deref(),
            ServerItem::ExtraUserAccount(u) => u.group_name.as_deref(),
            ServerItem::Database(d) => d.group_name.as_deref(),
        }
    }

    pub fn get_id(&self) -> i32 {
        match self {
            ServerItem::Website(w) => w.id,
            ServerItem::PointOfInterest(p) => p.id,
            ServerItem::Note(n) => n.id,
            ServerItem::ExtraUserAccount(u) => u.id,
            ServerItem::Database(d) => d.id,
        }
    }

    pub fn server_id(&self) -> i32 {
        match self {
            ServerItem::Website(w) => w.server_id,
            ServerItem::PointOfInterest(p) => p.server_id,
            ServerItem::Note(n) => n.server_id,
            ServerItem::ExtraUserAccount(u) => u.server_id,
            ServerItem::Database(d) => d.server_id,
        }
    }
}

#[derive(Debug)]
pub struct ChannelData {
    server_items: Vec<ServerItem>,
    group_start_indexes: HashMap<i32, String>,
    databases_for_websites: HashMap<i32, ServerDatabase>,
    websites_for_databases: HashMap<i32, Vec<ServerWebsite>>,
}

impl ProjectItem {
    fn display_item(&self) {
        println!("projectitem::display_item_id({})", self.imp().item_id.get());
        let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
        // TODO receive the item type besides the item_id and switch on item type here
        // also possibly receive the ProjectItem, telling me much more than the id
        let db_sender = app.unwrap().get_sql_channel();
        let cur_server_id = Some(self.imp().item_id.get());
        let (sender, receiver) = async_channel::bounded(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                // TODO this is server specific

                use projectpadsql::schema::server_database::dsl as srv_db;
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                use projectpadsql::schema::server_note::dsl as srv_note;
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                use projectpadsql::schema::server_website::dsl as srv_www;
                let (items, databases_for_websites, websites_for_databases) = match cur_server_id {
                    Some(sid) => {
                        let server_websites = srv_www::server_website
                            .filter(srv_www::server_id.eq(sid))
                            .order(srv_www::desc.asc())
                            .load::<ServerWebsite>(sql_conn)
                            .unwrap();

                        let databases_for_websites = srv_db::server_database
                            .filter(srv_db::id.eq_any(
                                server_websites.iter().filter_map(|w| w.server_database_id),
                            ))
                            .load::<ServerDatabase>(sql_conn)
                            .unwrap()
                            .into_iter()
                            .map(|db| (db.id, db))
                            .collect::<HashMap<_, _>>();

                        let mut servers = server_websites
                            .into_iter()
                            .map(ServerItem::Website)
                            .collect::<Vec<_>>();

                        servers.extend(
                            srv_poi::server_point_of_interest
                                .filter(srv_poi::server_id.eq(sid))
                                .order(srv_poi::desc.asc())
                                .load::<ServerPointOfInterest>(sql_conn)
                                .unwrap()
                                .into_iter()
                                .map(ServerItem::PointOfInterest),
                        );
                        servers.extend(
                            srv_note::server_note
                                .filter(srv_note::server_id.eq(sid))
                                .order(srv_note::title.asc())
                                .load::<ServerNote>(sql_conn)
                                .unwrap()
                                .into_iter()
                                .map(ServerItem::Note),
                        );
                        servers.extend(
                            &mut srv_usr::server_extra_user_account
                                .filter(srv_usr::server_id.eq(sid))
                                .order(srv_usr::desc.asc())
                                .load::<ServerExtraUserAccount>(sql_conn)
                                .unwrap()
                                .into_iter()
                                .map(ServerItem::ExtraUserAccount),
                        );

                        let databases = srv_db::server_database
                            .filter(srv_db::server_id.eq(sid))
                            .order(srv_db::desc.asc())
                            .load::<ServerDatabase>(sql_conn)
                            .unwrap();

                        let mut websites_for_databases = HashMap::new();
                        for (key, group) in &srv_www::server_website
                            .filter(
                                srv_www::server_database_id
                                    .eq_any(databases.iter().map(|db| db.id)),
                            )
                            .order(srv_www::server_database_id.asc())
                            .load::<ServerWebsite>(sql_conn)
                            .unwrap()
                            .into_iter()
                            .group_by(|www| www.server_database_id.unwrap())
                        {
                            websites_for_databases.insert(key, group.collect());
                        }

                        let mut dbs = databases.into_iter().map(ServerItem::Database);
                        servers.extend(&mut dbs);

                        (servers, databases_for_websites, websites_for_databases)
                    }
                    None => (vec![], HashMap::new(), HashMap::new()),
                };

                let group_names: BTreeSet<&str> =
                    items.iter().filter_map(|i| i.group_name()).collect();
                let mut group_start_indexes = HashMap::new();

                let mut grouped_items = vec![];
                grouped_items.extend(items.iter().filter(|i| i.group_name() == None));
                for group_name in &group_names {
                    group_start_indexes.insert(grouped_items.len() as i32, group_name.to_string());
                    grouped_items.extend(
                        items
                            .iter()
                            .filter(|i| i.group_name().as_ref() == Some(group_name)),
                    );
                }

                sender
                    .send_blocking(ChannelData {
                        server_items: grouped_items.into_iter().cloned().collect(),
                        group_start_indexes,
                        databases_for_websites,
                        websites_for_databases,
                    })
                    .unwrap();
            }))
            .unwrap();

        glib::spawn_future_local(async move {
            let channel_data = receiver.recv().await.unwrap();
            dbg!(&channel_data);
        });

        super::project_items::server::display_server(
            &self.imp().project_item,
            self.imp().item_id.get(),
            self.edit_mode(),
        );
    }
}
