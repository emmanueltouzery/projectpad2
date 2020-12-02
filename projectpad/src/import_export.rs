use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, Server, ServerDatabase, ServerExtraUserAccount,
    ServerNote, ServerWebsite,
};
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    servers: Vec<ServerImportExport>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    servers_in_groups: HashMap<String, Vec<ServerImportExport>>,
}

#[derive(Serialize)]
struct ServerImportExport {
    server: Server,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_websites: Vec<ServerWebsite>, // <--- how to tie DB to website???
    // server_databases: Vec<ServerDatabase>,
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
    println!("{}", serde_yaml::to_string(&project_importexport).unwrap());
}

fn export_env(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    env: EnvironmentType,
    group_names: &[String],
) -> ProjectEnvImportExport {
    use projectpadsql::schema::server::dsl as srv;

    let srvs = srv::server
        .filter(
            srv::project_id
                .eq(project.id)
                .and(srv::environment.eq(env))
                .and(srv::group_name.is_null()),
        )
        .order((srv::group_name.asc(), srv::desc.asc()))
        .load::<Server>(sql_conn)
        .unwrap();
    let servers = srvs
        .into_iter()
        .map(|s| export_server(sql_conn, s))
        .collect();

    // project notes

    // server links

    // project POIs

    let servers_in_groups = group_names
        .iter()
        .map(|gn| {
            let srvs = srv::server
                .filter(
                    srv::project_id
                        .eq(project.id)
                        .and(srv::environment.eq(env))
                        .and(srv::group_name.eq(gn)),
                )
                .order((srv::group_name.asc(), srv::desc.asc()))
                .load::<Server>(sql_conn)
                .unwrap();
            (
                gn.clone(),
                srvs.into_iter()
                    .map(|s| export_server(sql_conn, s))
                    .collect(),
            )

            // project notes

            // server links

            // project POIs
        })
        .collect();

    ProjectEnvImportExport {
        servers,
        servers_in_groups,
    }
}

fn export_server(sql_conn: &diesel::SqliteConnection, server: Server) -> ServerImportExport {
    use projectpadsql::schema::server_database::dsl as srv_db;
    use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
    use projectpadsql::schema::server_note::dsl as srv_note;
    use projectpadsql::schema::server_website::dsl as srv_www;

    // server websites
    let server_websites = srv_www::server_website
        .filter(srv_www::server_id.eq(server.id))
        .order(srv_www::desc.asc())
        .load::<ServerWebsite>(sql_conn)
        .unwrap();

    let server_dbs = srv_db::server_database
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
        server_websites,
        // server_databases,
        server_notes,
        server_extra_users,
    }
}
