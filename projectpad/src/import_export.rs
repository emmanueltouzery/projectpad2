use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase,
    ServerExtraUserAccount, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use projectpadsql::sqlite_is;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
struct ProjectImportExport {
    project_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    development_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    staging_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uat_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prod_environment: Option<ProjectEnvImportExport>,
}

#[derive(Serialize)]
struct ProjectEnvImportExport {
    items: ProjectEnvGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    items_in_groups: HashMap<String, ProjectEnvGroupImportExport>,
}

#[derive(Serialize)]
struct ProjectEnvGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    servers: Vec<ServerImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    project_pois: Vec<ProjectPointOfInterest>,
}

#[derive(Serialize)]
struct ServerImportExport {
    server: Server,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_pois: Vec<ServerPointOfInterest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_websites: Vec<ServerWebsite>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_databases: Vec<ServerDatabase>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_notes: Vec<ServerNote>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_extra_users: Vec<ServerExtraUserAccount>,
}

pub fn export_project(sql_conn: &diesel::SqliteConnection, project: &Project) {
    // if I export a 7zip i can export project icons and attachments in the zip too...
    let group_names = projectpadsql::get_project_group_names(sql_conn, project.id);

    let development_environment = if project.has_dev {
        Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvDevelopment,
            &group_names,
        ))
    } else {
        None
    };

    let staging_environment = if project.has_stage {
        Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvStage,
            &group_names,
        ))
    } else {
        None
    };

    let uat_environment = if project.has_uat {
        Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvUat,
            &group_names,
        ))
    } else {
        None
    };

    let prod_environment = if project.has_prod {
        Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvProd,
            &group_names,
        ))
    } else {
        None
    };

    let project_importexport = ProjectImportExport {
        project_name: project.name.clone(),
        development_environment,
        staging_environment,
        uat_environment,
        prod_environment,
    };
    println!("{}", generate_yaml(&project_importexport));
}

fn generate_yaml<T: ?Sized + serde::Serialize>(value: &T) -> String {
    // https://github.com/dtolnay/serde-yaml/issues/174
    // I really want notes to be exported with literal blocks,
    // but the library doesn't do that yet, so i'll post-process.
    let raw_output = serde_yaml::to_string(value).unwrap();
    // println!("{}", raw_output);
    let re = Regex::new(r#"(?m)^(\s*)([^"]*)"([^"]+)""#).unwrap();
    re.replace_all(&raw_output, |item: &regex::Captures| {
        let line_start = item.get(1).unwrap().as_str().to_string() + item.get(2).unwrap().as_str();
        let contents = item.get(3).unwrap().as_str();
        if contents.contains("\\n") {
            // add extra spaces in the separator for the deeper indentation
            let separator = format!("\n    {}", item.get(1).unwrap().as_str());
            format!(
                "{}|{}{}",
                line_start,
                separator,
                itertools::join(contents.split("\\n").map(|l| format!("{}", l)), &separator)
            )
        } else {
            format!("{}\"{}\"", line_start, contents)
        }
    })
    .to_string()
}

fn export_env(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    env: EnvironmentType,
    group_names: &[String],
) -> ProjectEnvImportExport {
    let items = export_env_group(sql_conn, project, env, None);

    let items_in_groups = group_names
        .iter()
        .map(|gn| {
            let group = export_env_group(sql_conn, project, env, Some(gn));
            (gn.clone(), group)
        })
        .collect();

    ProjectEnvImportExport {
        items,
        items_in_groups,
    }
}

fn export_env_group(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    env: EnvironmentType,
    group_name: Option<&str>,
) -> ProjectEnvGroupImportExport {
    use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
    use projectpadsql::schema::server::dsl as srv;
    let srvs = srv::server
        .filter(
            srv::project_id
                .eq(project.id)
                .and(srv::environment.eq(env))
                .and(sqlite_is(srv::group_name, group_name)),
        )
        .order((srv::group_name.asc(), srv::desc.asc()))
        .load::<Server>(sql_conn)
        .unwrap();

    // project notes

    // server links

    let project_pois = prj_poi::project_point_of_interest
        .filter(
            prj_poi::project_id
                .eq(project.id)
                .and(sqlite_is(prj_poi::group_name, group_name)),
        )
        .order((prj_poi::desc.asc(), prj_poi::path.asc()))
        .load::<ProjectPointOfInterest>(sql_conn)
        .unwrap();

    ProjectEnvGroupImportExport {
        servers: srvs
            .into_iter()
            .map(|s| export_server(sql_conn, s))
            .collect(),
        project_pois,
    }
}

fn export_server(sql_conn: &diesel::SqliteConnection, server: Server) -> ServerImportExport {
    use projectpadsql::schema::server_database::dsl as srv_db;
    use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
    use projectpadsql::schema::server_note::dsl as srv_note;
    use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
    use projectpadsql::schema::server_website::dsl as srv_www;

    let server_pois = srv_poi::server_point_of_interest
        .filter(srv_poi::server_id.eq(server.id))
        .order(srv_poi::desc.asc())
        .load::<ServerPointOfInterest>(sql_conn)
        .unwrap();

    let server_websites = srv_www::server_website
        .filter(srv_www::server_id.eq(server.id))
        .order(srv_www::desc.asc())
        .load::<ServerWebsite>(sql_conn)
        .unwrap();

    let server_databases = srv_db::server_database
        .filter(srv_db::server_id.eq(server.id))
        .order(srv_db::desc.asc())
        .load::<ServerDatabase>(sql_conn)
        .unwrap();

    let server_notes = srv_note::server_note
        .filter(srv_note::server_id.eq(server.id))
        .order(srv_note::title.asc())
        .load::<ServerNote>(sql_conn)
        .unwrap();

    let server_extra_users = srv_usr::server_extra_user_account
        .filter(srv_usr::server_id.eq(server.id))
        .order(srv_usr::username.asc())
        .load::<ServerExtraUserAccount>(sql_conn)
        .unwrap();

    ServerImportExport {
        server,
        server_pois,
        server_websites,
        server_databases,
        server_notes,
        server_extra_users,
    }
}
