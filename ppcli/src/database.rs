use super::actions;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use projectpadsql::models::*;
use skim::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use strum_macros::{Display, EnumString};

#[derive(Debug, PartialEq, Clone, PartialOrd, Ord, Eq)]
pub enum ItemType {
    // ppcli depends on the fact that servers are the first item
    // type for sorting of the display
    ServerItemType(ServerType),
    InterestItemType(InterestType),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerInfo {
    pub server_desc: String,
    pub server_username: String,
    pub server_ip: String,
    pub server_access_type: ServerAccessType,
}

#[derive(Debug, Clone)]
pub struct PoiInfo {
    pub path: PathBuf,
}

#[derive(PartialEq, Eq, Copy, Clone, PartialOrd, Ord, Hash, Debug)]
pub enum LinkedItemId {
    Server(i32),
    ServerPoi(i32),
    ProjectPoi(i32),
}

#[derive(Debug, Clone)]
pub struct ItemOfInterest {
    pub linked_item: LinkedItemId,
    pub project_name: String,
    pub env: Option<EnvironmentType>,
    pub item_type: ItemType,
    pub poi_desc: Option<String>,
    pub item_text: String,
    pub server_info: Option<ServerInfo>,
    pub poi_info: Option<PoiInfo>,
    pub run_on: Option<RunOn>,
}

fn filter_servers(db_conn: &SqliteConnection) -> Vec<ItemOfInterest> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::server::dsl as srv;
    srv::server
        .inner_join(prj::project)
        .select((
            srv::id,
            prj::name,
            srv::desc,
            srv::environment,
            srv::ip,
            srv::server_type,
            srv::username,
            srv::access_type,
        ))
        .filter(srv::access_type.ne_all(vec![
            ServerAccessType::SrvAccessRdp,
            ServerAccessType::SrvAccessWww,
        ]))
        .load::<(_, _, String, _, String, _, _, _)>(db_conn)
        .unwrap()
        .into_iter()
        .map(
            |(
                id,
                project_name,
                server_desc,
                srv_env,
                server_ip,
                server_type,
                server_username,
                server_access_type,
            )| {
                ItemOfInterest {
                    linked_item: LinkedItemId::Server(id),
                    project_name,
                    env: Some(srv_env),
                    item_type: ItemType::ServerItemType(server_type),
                    poi_desc: Some(server_desc.clone()),
                    item_text: server_ip.clone(),
                    server_info: Some(ServerInfo {
                        server_desc,
                        server_username,
                        server_ip,
                        server_access_type,
                    }),
                    poi_info: None,
                    run_on: None,
                }
            },
        )
        .collect()
}

fn filter_project_pois(db_conn: &SqliteConnection) -> Vec<ItemOfInterest> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
    prj_poi::project_point_of_interest
        .inner_join(prj::project)
        .select((
            prj_poi::id,
            prj::name,
            prj_poi::desc,
            prj_poi::text,
            prj_poi::interest_type,
            prj_poi::path,
        ))
        .load::<(_, _, _, _, _, String)>(db_conn)
        .unwrap()
        .into_iter()
        .map(
            |(id, project_name, prj_poi_desc, item_text, prj_poi_interest_type, prj_path)| {
                ItemOfInterest {
                    linked_item: LinkedItemId::ProjectPoi(id),
                    project_name,
                    env: None,
                    item_type: ItemType::InterestItemType(prj_poi_interest_type),
                    poi_desc: Some(prj_poi_desc),
                    item_text,
                    server_info: None,
                    poi_info: Some(PoiInfo {
                        path: prj_path.into(),
                    }),
                    run_on: None,
                }
            },
        )
        .collect()
}

fn filter_server_pois(db_conn: &SqliteConnection) -> Vec<ItemOfInterest> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::server::dsl as srv;
    use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
    srv_poi::server_point_of_interest
        .inner_join(srv::server.inner_join(prj::project))
        .select((
            srv_poi::id,
            prj::name,
            srv::desc,
            srv_poi::desc,
            srv::environment,
            srv_poi::text,
            srv_poi::interest_type,
            srv::username,
            srv_poi::path,
            srv::access_type,
            srv::ip,
            srv_poi::run_on,
        ))
        .filter(srv::access_type.ne_all(vec![
            ServerAccessType::SrvAccessRdp,
            ServerAccessType::SrvAccessWww,
        ]))
        .load::<(_, _, _, _, _, _, _, _, String, _, _, _)>(db_conn)
        .unwrap()
        .into_iter()
        .map(
            |(
                id,
                project_name,
                server_desc,
                server_poi_desc,
                srv_env,
                item_text,
                srv_poi_interest_type,
                server_username,
                srv_poi_path,
                server_access_type,
                server_ip,
                run_on_val,
            )| {
                ItemOfInterest {
                    linked_item: LinkedItemId::ServerPoi(id),
                    project_name,
                    env: Some(srv_env),
                    item_type: ItemType::InterestItemType(srv_poi_interest_type),
                    poi_desc: Some(server_poi_desc),
                    item_text,
                    server_info: Some(ServerInfo {
                        server_desc,
                        server_username,
                        server_ip,
                        server_access_type,
                    }),
                    poi_info: Some(PoiInfo {
                        path: srv_poi_path.into(),
                    }),
                    run_on: Some(run_on_val),
                }
            },
        )
        .collect()
}

#[derive(Hash, PartialEq, Eq, EnumString, Display, Clone, Copy, PartialOrd, Ord, Debug)]
pub enum ActionType {
    #[strum(serialize = "tail log")]
    TailLog,
    #[strum(serialize = "less log")]
    LessLog,
    #[strum(serialize = "fetch log")]
    FetchLog,
    #[strum(serialize = "ssh folder")]
    SshFolder,
    #[strum(serialize = "ssh shell")]
    SshShell,
    #[strum(serialize = "go folder")]
    GoFolder,
    #[strum(serialize = "run cmd")]
    RunCmd,
    #[strum(serialize = "edit cfg")]
    EditCfg,
    #[strum(serialize = "less cfg")]
    LessCfg,
    #[strum(serialize = "fetch cfg")]
    FetchCfg,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Debug)]
pub struct ExecutedAction {
    pub item: LinkedItemId,
    pub action_desc: ActionType,
}

impl ExecutedAction {
    pub fn new(item: LinkedItemId, action_desc: ActionType) -> ExecutedAction {
        ExecutedAction { item, action_desc }
    }
}

pub fn load_items<T>(
    conn: &SqliteConnection,
    display_mode: DisplayMode,
    item_sender: &Sender<Arc<dyn SkimItem>>,
    ranked_items: &HashMap<ExecutedAction, T>,
) where
    T: Ord,
{
    let mut items = filter_server_pois(conn);
    items.extend(filter_project_pois(conn));
    items.extend(filter_servers(conn));
    if items.is_empty() {
        println!("No items to display. Keep in mind that ppcli will only display non RDP/non WWW servers, and point of interests");
        std::process::exit(0);
    }
    // items.reverse();
    let cols_spec = vec![7, 3, 4, 30, 25, 10];
    let mut actions: Vec<_> = items.into_iter().flat_map(actions::get_value).collect();
    actions.sort_by(|a, b| {
        let a_rank = ranked_items.get(&ExecutedAction {
            item: a.item.linked_item,
            action_desc: a.desc,
        });
        let b_rank = ranked_items.get(&ExecutedAction {
            item: b.item.linked_item,
            action_desc: b.desc,
        });
        b_rank
            .cmp(&a_rank)
            .then(b.item.project_name.cmp(&a.item.project_name))
            .then(b.item.server_info.cmp(&a.item.server_info))
            .then(b.item.item_type.cmp(&a.item.item_type))
            .then(b.item.item_text.cmp(&a.item.item_text))
    });
    for action in actions {
        let _ = item_sender.send(Arc::new(crate::MyItem {
            display: render_row(&cols_spec, &action, display_mode),
            inner: action,
        }));
    }
}

fn render_row(cols_spec: &[usize], action: &actions::Action, display_mode: DisplayMode) -> String {
    let item = &action.item;
    let mut col1 = item.project_name.clone();
    col1.truncate(cols_spec[0]);
    let col2 = item
        .env
        .as_ref()
        .map(|env| display_env(env, display_mode))
        .unwrap_or(if display_mode == DisplayMode::Color {
            "-   "
        } else {
            "-  "
        })
        .to_string();
    // col2.truncate(cols_spec[1]);
    let mut col3 = render_type(&item.item_type).to_string();
    col3.truncate(cols_spec[2]);
    let mut col4 = item
        .server_info
        .as_ref()
        .map(|si| si.server_desc.clone())
        .unwrap_or_else(|| "-".to_string());
    col4.truncate(cols_spec[3]);
    let mut col5 = item
        .poi_desc
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "".to_string());
    col5.truncate(cols_spec[4]);
    let mut col6 = action.desc.to_string();
    col6.truncate(cols_spec[5]);
    format!(
        "{:<w1$} {:<w2$} {:<w3$} {:<w4$} {:<w5$}  {:<w6$}",
        col1,
        col2,
        col3,
        col4,
        col5,
        col6,
        w1 = cols_spec[0],
        w2 = cols_spec[1],
        w3 = cols_spec[2],
        w4 = cols_spec[3],
        w5 = cols_spec[4],
        w6 = cols_spec[5],
    )
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DisplayMode {
    Plain,
    Color,
}

fn display_env(env: &EnvironmentType, display_mode: DisplayMode) -> &'static str {
    match (env, display_mode) {
        (EnvironmentType::EnvDevelopment, DisplayMode::Color) => "\x1b[32m\x1b[1m❚DEV\x1b[0m",
        (EnvironmentType::EnvDevelopment, DisplayMode::Plain) => "DEV",
        (EnvironmentType::EnvStage, DisplayMode::Color) => "\x1b[34m\x1b[1m❚STG\x1b[0m",
        (EnvironmentType::EnvStage, DisplayMode::Plain) => "STG",
        (EnvironmentType::EnvUat, DisplayMode::Color) => "\x1b[33m\x1b[1m❚UAT\x1b[0m",
        (EnvironmentType::EnvUat, DisplayMode::Plain) => "UAT",
        (EnvironmentType::EnvProd, DisplayMode::Color) => "\x1b[31m\x1b[1m❚PRD\x1b[0m",
        (EnvironmentType::EnvProd, DisplayMode::Plain) => "PRD",
    }
}

fn render_type(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::InterestItemType(InterestType::PoiCommandToRun) => "CMD",
        ItemType::InterestItemType(InterestType::PoiCommandTerminal) => "CMD",
        ItemType::InterestItemType(InterestType::PoiConfigFile) => "CFG",
        ItemType::InterestItemType(InterestType::PoiLogFile) => "LOG",
        ItemType::InterestItemType(InterestType::PoiApplication) => "APP",
        ItemType::InterestItemType(InterestType::PoiBackupArchive) => "BKP",
        ItemType::ServerItemType(ServerType::SrvApplication) => "SRA",
        ItemType::ServerItemType(ServerType::SrvDatabase) => "DAT",
        ItemType::ServerItemType(ServerType::SrvHttpOrProxy) => "HTT",
        ItemType::ServerItemType(ServerType::SrvReporting) => "REP",
        ItemType::ServerItemType(ServerType::SrvMonitoring) => "MON",
    }
}
