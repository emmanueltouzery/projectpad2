use crate::sql_util::insert_row;
use diesel::dsl::count;
use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase,
    ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use projectpadsql::sqlite_is;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

type ImportResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Deserialize)]
struct ProjectImportExport {
    project_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    development_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    staging_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    uat_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    prod_environment: Option<ProjectEnvImportExport>,
}

#[derive(Serialize, Deserialize)]
struct ProjectEnvImportExport {
    items: ProjectEnvGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    items_in_groups: HashMap<String, ProjectEnvGroupImportExport>,
}

#[derive(Serialize, Deserialize)]
struct ProjectEnvGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    servers: Vec<ServerImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_links: Vec<ServerLink>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    project_pois: Vec<ProjectPointOfInterest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    project_notes: Vec<ProjectNote>,
}

#[derive(Serialize, Deserialize)]
struct ServerImportExport {
    server: Server,
    items: ServerGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    items_in_groups: HashMap<String, ServerGroupImportExport>,
}

#[derive(Serialize, Deserialize)]
struct ServerGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_pois: Vec<ServerPointOfInterest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_websites: Vec<ServerWebsite>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_databases: Vec<ServerDatabase>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_notes: Vec<ServerNote>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_extra_users: Vec<ServerExtraUserAccount>,
}

fn to_boxed_stderr(err: (String, Option<String>)) -> Box<dyn std::error::Error> {
    (err.0 + " - " + err.1.as_deref().unwrap_or("")).into()
}

pub fn do_import(sql_conn: &diesel::SqliteConnection, fname: &str) -> ImportResult<()> {
    use projectpadsql::schema::project::dsl as prj;
    let contents = fs::read_to_string(fname)?;
    let decoded: ProjectImportExport = serde_yaml::from_str(&contents)?;
    if prj::project
        .filter(prj::name.eq(&decoded.project_name))
        .select(count(prj::id))
        .first::<i64>(sql_conn)
        .unwrap()
        >= 1
    {
        return Err("A project with this name already exists".into());
    }
    let changeset = (
        prj::name.eq(decoded.project_name),
        prj::has_dev.eq(decoded.development_environment.is_some()),
        prj::has_stage.eq(decoded.staging_environment.is_some()),
        prj::has_uat.eq(decoded.uat_environment.is_some()),
        prj::has_prod.eq(decoded.prod_environment.is_some()),
        // TODO load the icon from the import 7zip
        prj::icon.eq(Some(Vec::<u8>::new())),
    );
    let project_id = insert_row(sql_conn, changeset).map_err(to_boxed_stderr)?;
    if let Some(dev_env) = decoded.development_environment {
        import_project_env(
            sql_conn,
            project_id,
            EnvironmentType::EnvDevelopment,
            &dev_env,
        )?;
    }
    if let Some(stg_env) = decoded.staging_environment {
        import_project_env(sql_conn, project_id, EnvironmentType::EnvStage, &stg_env)?;
    }
    if let Some(uat_env) = decoded.uat_environment {
        import_project_env(sql_conn, project_id, EnvironmentType::EnvUat, &uat_env)?;
    }
    if let Some(prod_env) = decoded.prod_environment {
        import_project_env(sql_conn, project_id, EnvironmentType::EnvProd, &prod_env)?;
    }
    Ok(())
}

fn import_project_env(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    project_env: &ProjectEnvImportExport,
) -> ImportResult<()> {
    for server in &project_env.items.servers {
        import_server(sql_conn, project_id, env, None, server)?;
    }
    Ok(())
}

fn import_server(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    group_name: Option<&str>,
    server: &ServerImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::server::dsl as srv;
    let changeset = (
        srv::desc.eq(&server.server.desc),
        srv::is_retired.eq(server.server.is_retired),
        srv::ip.eq(&server.server.ip),
        srv::text.eq(&server.server.text),
        srv::group_name.eq(group_name),
        srv::username.eq(&server.server.username),
        srv::password.eq(&server.server.password),
        srv::auth_key.eq(server.server.auth_key.as_ref()), // TODO probably stored elsewhere
        srv::auth_key_filename.eq(server.server.auth_key_filename.as_ref()),
        srv::server_type.eq(server.server.server_type),
        srv::access_type.eq(server.server.access_type),
        srv::environment.eq(env),
        srv::project_id.eq(project_id),
    );
    let server_id = insert_row(sql_conn, changeset).map_err(to_boxed_stderr)?;

    import_server_items(sql_conn, server_id, None, &server.items)?;
    for (group_name, items) in &server.items_in_groups {
        import_server_items(sql_conn, server_id, Some(group_name), items)?;
    }

    Ok(())
}

fn import_server_items(
    sql_conn: &diesel::SqliteConnection,
    server_id: i32,
    group_name: Option<&str>,
    items: &ServerGroupImportExport,
) -> ImportResult<()> {
    for db in &items.server_databases {
        use projectpadsql::schema::server_database::dsl as srv_db;
        let changeset = (
            srv_db::desc.eq(&db.desc),
            srv_db::name.eq(&db.name),
            srv_db::group_name.eq(group_name),
            srv_db::text.eq(&db.text),
            srv_db::username.eq(&db.username),
            srv_db::password.eq(&db.password),
            srv_db::server_id.eq(server_id),
        );
        let db_id = insert_row(sql_conn, changeset).map_err(to_boxed_stderr)?;
    }
    Ok(())
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
    // println!("RAW: {:?}", raw_output);
    yaml_fix_multiline_strings(&raw_output)
}

fn yaml_fix_multiline_strings(raw_output: &str) -> String {
    // (?m) => enable regex multiline (^ and $ match beginning and end of line)
    // ^ beginning of line
    // (\s*) leading spaces, and we capture them in the first capture group
    // ([^\s\n][^"\n]*) a non-space character, followed by any non-quote characters,
    //         excluding newlines; meaning the field name in yaml
    //         (eg in 'name: "value"' this captures 'name: '), second capture group
    // " quote, beginning of the string, that we want to maybe modify
    // ([^\n]+) any quote contents, excluding \n (may include ", if escaped), third capture group
    // " end of the string
    // $ end of the line
    let re = Regex::new(r#"(?m)^(\s*)([^\s\n][^"\n]*)"([^\n]+)"$"#).unwrap();
    re.replace_all(raw_output, |item: &regex::Captures| {
        let line_start = item.get(1).unwrap().as_str().to_string() + item.get(2).unwrap().as_str();
        let contents = item.get(3).unwrap().as_str();
        if contents.contains("\\n") {
            // add extra spaces in the separator for the deeper indentation
            let separator = format!("\n  {}", item.get(1).unwrap().as_str());
            format!(
                "{}|{}{}",
                line_start,
                separator,
                itertools::join(
                    contents.split("\\n").map(|l| format!(
                        "{}",
                        l.replace(r#"\""#, r#"""#).replace(r#"\\"#, r#"\"#)
                    )),
                    &separator
                )
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
    use projectpadsql::schema::project_note::dsl as prj_note;
    use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
    use projectpadsql::schema::server::dsl as srv;
    use projectpadsql::schema::server_link::dsl as srvl;
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

    let mut notes_query = prj_note::project_note
        .filter(
            prj_note::project_id
                .eq(project.id)
                .and(sqlite_is(prj_note::group_name, group_name)),
        )
        .into_boxed();
    notes_query = match env {
        EnvironmentType::EnvDevelopment => notes_query.filter(prj_note::has_dev.eq(true)),
        EnvironmentType::EnvStage => notes_query.filter(prj_note::has_stage.eq(true)),
        EnvironmentType::EnvUat => notes_query.filter(prj_note::has_uat.eq(true)),
        EnvironmentType::EnvProd => notes_query.filter(prj_note::has_prod.eq(true)),
    };

    let project_notes = notes_query
        .order(prj_note::title.asc())
        .load::<ProjectNote>(sql_conn)
        .unwrap();

    let server_links = srvl::server_link
        .filter(
            srvl::project_id
                .eq(project.id)
                .and(srvl::environment.eq(env))
                .and(sqlite_is(srvl::group_name, group_name)),
        )
        .order(srvl::desc.asc())
        .load::<ServerLink>(sql_conn)
        .unwrap();

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
        server_links,
        project_pois,
        project_notes,
    }
}

fn export_server(sql_conn: &diesel::SqliteConnection, server: Server) -> ServerImportExport {
    let items = export_server_items(sql_conn, &server, None);
    let group_names = projectpadsql::get_server_group_names(sql_conn, server.id);
    let items_in_groups = group_names
        .into_iter()
        .map(|gn| {
            (
                gn.clone(),
                export_server_items(sql_conn, &server, Some(&gn)),
            )
        })
        .collect();
    ServerImportExport {
        server,
        items,
        items_in_groups,
    }
}

fn export_server_items(
    sql_conn: &diesel::SqliteConnection,
    server: &Server,
    group_name: Option<&str>,
) -> ServerGroupImportExport {
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

    ServerGroupImportExport {
        server_pois,
        server_websites,
        server_databases,
        server_notes,
        server_extra_users,
    }
}

#[test]
fn fix_yaml_strings_nochange() {
    assert_eq!(
        r#"test: "no newlines""#,
        yaml_fix_multiline_strings(r#"test: "no newlines""#)
    );
}

#[test]
fn fix_yaml_strings_simple_newlines() {
    assert_eq!(
        "test: |\n  first line\n  second line",
        yaml_fix_multiline_strings("test: \"first line\\nsecond line\"")
    );
}

#[test]
fn fix_yaml_strings_newlines_and_quotes_in_string() {
    assert_eq!(
        "test: |\n  first \"line\"\n  second \\line",
        yaml_fix_multiline_strings("test: \"first \\\"line\\\"\\nsecond \\\\line\"")
    );
}
