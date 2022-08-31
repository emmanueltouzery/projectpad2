use diesel::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use std::collections::HashSet;

pub const PROJECT_FILTER_PREFIX: &str = "prj:";

#[derive(PartialEq, Clone, Copy)]
pub enum SearchItemsType {
    All,
    ServerDbsOnly,
    ServersOnly,
}

pub struct SearchResult {
    pub projects: Vec<Project>,
    pub project_notes: Vec<ProjectNote>,
    pub project_pois: Vec<ProjectPointOfInterest>,
    pub server_links: Vec<ServerLink>,
    pub servers: Vec<Server>,
    pub server_databases: Vec<ServerDatabase>,
    pub server_extra_users: Vec<ServerExtraUserAccount>,
    pub server_notes: Vec<ServerNote>,
    pub server_pois: Vec<ServerPointOfInterest>,
    pub server_websites: Vec<ServerWebsite>,
    pub reset_scroll: bool,
}

pub fn run_search_filter(
    sql_conn: &mut SqliteConnection,
    search_item_types: SearchItemsType,
    search_pattern: &str,
    project_pattern: &Option<String>,
    reset_scroll: bool,
) -> SearchResult {
    // find all the leaves...
    let servers = if search_item_types == SearchItemsType::ServersOnly
        || search_item_types == SearchItemsType::All
    {
        filter_servers(sql_conn, search_pattern)
    } else {
        vec![]
    };
    let server_databases = if search_item_types == SearchItemsType::ServerDbsOnly
        || search_item_types == SearchItemsType::All
    {
        filter_server_databases(sql_conn, search_pattern)
    } else {
        vec![]
    };

    let (
        prjs,
        project_pois,
        project_notes,
        server_notes,
        server_links,
        server_pois,
        server_extra_users,
        server_websites,
    ) = if search_item_types == SearchItemsType::All {
        (
            filter_projects(sql_conn, search_pattern),
            filter_project_pois(sql_conn, search_pattern),
            filter_project_notes(sql_conn, search_pattern),
            filter_server_notes(sql_conn, search_pattern),
            filter_server_links(sql_conn, search_pattern),
            filter_server_pois(sql_conn, search_pattern),
            filter_server_extra_users(sql_conn, search_pattern),
            filter_server_websites(sql_conn, search_pattern)
                .into_iter()
                .map(|p| p.0)
                .collect::<Vec<_>>(),
        )
    } else {
        (
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
    };

    // bubble up to the toplevel...
    let mut all_server_ids = servers.iter().map(|s| s.id).collect::<HashSet<_>>();
    all_server_ids.extend(server_websites.iter().map(|sw| sw.server_id));
    all_server_ids.extend(server_notes.iter().map(|sn| sn.server_id));
    all_server_ids.extend(server_links.iter().map(|sl| sl.linked_server_id));
    all_server_ids.extend(server_extra_users.iter().map(|sl| sl.server_id));
    all_server_ids.extend(server_pois.iter().map(|sl| sl.server_id));
    all_server_ids.extend(server_databases.iter().map(|sl| sl.server_id));
    let all_servers = load_servers_by_id(sql_conn, &all_server_ids);

    let mut all_project_ids = all_servers
        .iter()
        .map(|s| s.project_id)
        .collect::<HashSet<_>>();
    all_project_ids.extend(prjs.iter().map(|p| p.id));
    all_project_ids.extend(project_pois.iter().map(|ppoi| ppoi.project_id));
    all_project_ids.extend(project_notes.iter().map(|pn| pn.project_id));
    all_project_ids.extend(server_links.iter().map(|pn| pn.project_id));
    let all_projects = load_projects_by_id(sql_conn, &all_project_ids);
    let filtered_projects = match &project_pattern {
        None => all_projects,
        Some(prj) => all_projects
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(prj))
            .collect(),
    };
    SearchResult {
        projects: filtered_projects,
        project_notes,
        project_pois,
        servers: all_servers,
        server_notes,
        server_links,
        server_pois,
        server_databases,
        server_extra_users,
        server_websites,
        reset_scroll,
    }
}

fn filter_projects(db_conn: &mut SqliteConnection, filter: &str) -> Vec<Project> {
    use projectpadsql::schema::project::dsl::*;
    project
        .filter(name.like(filter).escape('\\'))
        .load::<Project>(db_conn)
        .unwrap()
}

fn filter_project_pois(
    db_conn: &mut SqliteConnection,
    filter: &str,
) -> Vec<ProjectPointOfInterest> {
    use projectpadsql::schema::project_point_of_interest::dsl::*;
    project_point_of_interest
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(text.like(filter).escape('\\'))
                .or(path.like(filter).escape('\\')),
        )
        .load::<ProjectPointOfInterest>(db_conn)
        .unwrap()
}

fn filter_project_notes(db_conn: &mut SqliteConnection, filter: &str) -> Vec<ProjectNote> {
    use projectpadsql::schema::project_note::dsl::*;
    project_note
        .filter(
            title
                .like(filter)
                .escape('\\')
                .or(contents.like(filter).escape('\\')),
        )
        .load::<ProjectNote>(db_conn)
        .unwrap()
}

fn filter_server_notes(db_conn: &mut SqliteConnection, filter: &str) -> Vec<ServerNote> {
    use projectpadsql::schema::server_note::dsl::*;
    server_note
        .filter(
            title
                .like(filter)
                .escape('\\')
                .or(contents.like(filter).escape('\\')),
        )
        .load::<ServerNote>(db_conn)
        .unwrap()
}

fn filter_server_links(db_conn: &mut SqliteConnection, filter: &str) -> Vec<ServerLink> {
    use projectpadsql::schema::server_link::dsl::*;
    server_link
        .filter(desc.like(filter).escape('\\'))
        .load::<ServerLink>(db_conn)
        .unwrap()
}

fn filter_server_extra_users(
    db_conn: &mut SqliteConnection,
    filter: &str,
) -> Vec<ServerExtraUserAccount> {
    use projectpadsql::schema::server_extra_user_account::dsl::*;
    server_extra_user_account
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(username.like(filter).escape('\\')),
        )
        .load::<ServerExtraUserAccount>(db_conn)
        .unwrap()
}

fn filter_server_pois(db_conn: &mut SqliteConnection, filter: &str) -> Vec<ServerPointOfInterest> {
    use projectpadsql::schema::server_point_of_interest::dsl::*;
    server_point_of_interest
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(path.like(filter).escape('\\'))
                .or(text.like(filter).escape('\\')),
        )
        .load::<ServerPointOfInterest>(db_conn)
        .unwrap()
}

fn filter_server_databases(db_conn: &mut SqliteConnection, filter: &str) -> Vec<ServerDatabase> {
    use projectpadsql::schema::server_database::dsl::*;
    server_database
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(name.like(filter).escape('\\'))
                .or(text.like(filter).escape('\\')),
        )
        .load::<ServerDatabase>(db_conn)
        .unwrap()
}

fn filter_servers(db_conn: &mut SqliteConnection, filter: &str) -> Vec<Server> {
    use projectpadsql::schema::server::dsl::*;
    server
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(ip.like(filter).escape('\\'))
                .or(text.like(filter).escape('\\')),
        )
        .load::<Server>(db_conn)
        .unwrap()
}

fn filter_server_websites(
    db_conn: &mut SqliteConnection,
    filter: &str,
) -> Vec<(ServerWebsite, Option<ServerDatabase>)> {
    use projectpadsql::schema::server_database::dsl as db;
    use projectpadsql::schema::server_website::dsl::*;
    server_website
        .left_outer_join(db::server_database)
        .filter(
            desc.like(filter)
                .escape('\\')
                .or(url.like(filter).escape('\\'))
                .or(text.like(filter).escape('\\'))
                .or(db::desc.like(filter).escape('\\'))
                .or(db::name.like(filter).escape('\\')),
        )
        .load::<(ServerWebsite, Option<ServerDatabase>)>(db_conn)
        .unwrap()
}

fn load_projects_by_id(db_conn: &mut SqliteConnection, ids: &HashSet<i32>) -> Vec<Project> {
    use projectpadsql::schema::project::dsl::*;
    project
        .filter(id.eq_any(ids))
        .order(name.asc())
        .load::<Project>(db_conn)
        .unwrap()
}

fn load_servers_by_id(db_conn: &mut SqliteConnection, ids: &HashSet<i32>) -> Vec<Server> {
    use projectpadsql::schema::server::dsl::*;
    server
        .filter(id.eq_any(ids))
        .load::<Server>(db_conn)
        .unwrap()
}

#[derive(PartialEq, Eq, Debug)]
pub struct SearchSpec {
    pub search_pattern: String,
    pub project_pattern: Option<String>,
}

#[derive(PartialEq, Eq)]
enum SearchParseState {
    InProject,
    Normal,
}

pub fn search_parse(search: &str) -> SearchSpec {
    let fmt = |t: &str| format!("%{}%", t.replace('%', "\\%"));
    if search.starts_with(PROJECT_FILTER_PREFIX)
        || search.contains(&(" ".to_string() + PROJECT_FILTER_PREFIX))
    {
        let (search, project, _) = search.split(' ').fold(
            ("".to_string(), None, SearchParseState::Normal),
            |(search, project, parse_state), fragment| match parse_state {
                SearchParseState::Normal if fragment.starts_with(PROJECT_FILTER_PREFIX) => (
                    search,
                    Some(fragment[4..].to_lowercase()),
                    if fragment.chars().filter(|c| *c == '"').count() % 2 != 0 {
                        SearchParseState::InProject
                    } else {
                        SearchParseState::Normal
                    },
                ),
                SearchParseState::Normal => (
                    if search.is_empty() {
                        fragment.to_owned()
                    } else {
                        search + " " + fragment
                    },
                    project,
                    SearchParseState::Normal,
                ),
                SearchParseState::InProject => (
                    search,
                    Some(project.unwrap() + " " + &fragment.to_lowercase()[..]),
                    if fragment.contains('\"') {
                        SearchParseState::Normal
                    } else {
                        SearchParseState::InProject
                    },
                ),
            },
        );
        SearchSpec {
            search_pattern: fmt(&search),
            project_pattern: match project {
                Some(p) if p.starts_with('"') => Some(p.replace('"', "")),
                _ => project,
            },
        }
    } else {
        SearchSpec {
            search_pattern: fmt(search),
            project_pattern: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::tests::{tests_load_yaml, SAMPLE_YAML_PROJECT};

    #[test]
    fn search_parse_no_project() {
        assert_eq!(
            SearchSpec {
                search_pattern: "%test no project%".to_string(),
                project_pattern: None
            },
            search_parse("test no project")
        );
    }

    #[test]
    fn search_parse_with_project() {
        assert_eq!(
            SearchSpec {
                search_pattern: "%item1 test item3%".to_string(),
                project_pattern: Some("project".to_string())
            },
            search_parse("item1 test prj:prOject item3")
        );
    }

    #[test]
    fn search_parse_with_quoted_project() {
        assert_eq!(
            SearchSpec {
                search_pattern: "%item1 test item3%".to_string(),
                project_pattern: Some("project with spaces".to_string())
            },
            search_parse("item1 test prj:\"prOject with spaces\" item3")
        );
    }

    #[test]
    fn search_parse_with_unnecessarily_quoted_project() {
        assert_eq!(
            SearchSpec {
                search_pattern: "%item1 test item3%".to_string(),
                project_pattern: Some("project".to_string())
            },
            search_parse("item1 test prj:\"prOject\" item3")
        );
    }

    #[test]
    fn search_finds_users() {
        let mut db_conn = tests_load_yaml(SAMPLE_YAML_PROJECT);
        let search_result =
            run_search_filter(&mut db_conn, SearchItemsType::All, "monitor", &None, false);
        // we should find the user...
        assert_eq!(1, search_result.server_extra_users.len());
        assert_eq!(
            "monpass",
            search_result.server_extra_users.get(0).unwrap().password
        );

        // should also include the server on which the user is...
        assert_eq!(1, search_result.servers.len());
        assert_eq!("My server", search_result.servers.get(0).unwrap().desc);

        // should also include the project on which the server is...
        assert_eq!(1, search_result.projects.len());
        assert_eq!("Demo", search_result.projects.get(0).unwrap().name);
    }
}
