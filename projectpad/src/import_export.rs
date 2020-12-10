use crate::sql_util::insert_row;
use diesel::dsl::count;
use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, InterestType, Project, ProjectNote, ProjectPointOfInterest, Server,
    ServerDatabase, ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
use projectpadsql::sqlite_is;
use regex::Regex;
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

fn serialize_if_present<T>(map: &mut T, key: &str, value: &str) -> Result<(), T::Error>
where
    T: SerializeMap,
{
    if !value.is_empty() {
        map.serialize_entry(key, value)
    } else {
        Ok(())
    }
}

fn serialize_if_some<T, V>(map: &mut T, key: &str, value: &Option<V>) -> Result<(), T::Error>
where
    T: SerializeMap,
    V: Serialize,
{
    if value.is_some() {
        map.serialize_entry(key, value)
    } else {
        Ok(())
    }
}

#[derive(Deserialize)]
struct ServerImportExport(Server);

impl Serialize for ServerImportExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = &self.0;
        let mut state = serializer.serialize_map(None)?;

        // we want to allow to link to any server (ServerLink may need to)
        // if there is a desc, we'll link to the desc. If there is no desc,
        // we'll link to the id.
        if s.desc.is_empty() {
            state.serialize_entry("id", &s.id)?;
        } else {
            state.serialize_entry("desc", &s.desc)?;
        }
        serialize_if_present(&mut state, "ip", &s.ip)?;
        serialize_if_present(&mut state, "text", &s.text)?;
        if s.is_retired {
            state.serialize_entry("is_retired", &s.is_retired)?;
        }
        serialize_if_present(&mut state, "username", &s.username)?;
        serialize_if_present(&mut state, "password", &s.password)?;
        // TODO auth_key
        serialize_if_some(&mut state, "auth_key_filename", &s.auth_key_filename)?;
        state.serialize_entry("server_type", &s.server_type)?;
        state.serialize_entry("access_type", &s.access_type)?;
        serialize_if_some(&mut state, "ssh_tunnel_port", &s.ssh_tunnel_port)?;
        // TODO through_server_id

        state.end()
    }
}

#[derive(Deserialize)]
struct ServerDatabaseImportExport(ServerDatabase);

impl Serialize for ServerDatabaseImportExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = &self.0;
        let mut state = serializer.serialize_map(None)?;

        // we want to allow to link to any server (ServerWebsite may need to)
        // if there is a desc, we'll link to the desc. If there is no desc,
        // we'll link to the id.
        if s.desc.is_empty() {
            state.serialize_entry("id", &s.id)?;
        } else {
            state.serialize_entry("desc", &s.desc)?;
        }
        serialize_if_present(&mut state, "name", &s.name)?;
        serialize_if_present(&mut state, "text", &s.text)?;
        serialize_if_present(&mut state, "username", &s.username)?;
        serialize_if_present(&mut state, "password", &s.password)?;

        state.end()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ServerDatabasePath {
    project_name: String,
    environment: EnvironmentType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    server_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    server_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    database_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    database_desc: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ServerWebsiteImportExport {
    #[serde(skip_serializing_if = "String::is_empty", default)]
    desc: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    url: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    text: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    password: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    server_database: Option<ServerDatabasePath>,
}

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

/// currently project POIs are present for all environments,
/// cannot be restricted. I don't want to export them only
/// in one environment, but i don't want to repeat them (verbose)
/// => the first time they appear i export them normally.
///    the following times, i export only the desc and
///    "shared_with_other_environments".
/// This also helps to import back only once, and keep in
/// mind, the project POIs are present for every *enabled*
/// environment. Eg UAT may not be active for that project.
#[derive(Deserialize)]
struct ProjectPoiImportExport {
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub interest_type: InterestType,
    #[serde(default)]
    shared_with_other_environments: Option<String>,
}

impl Serialize for ProjectPoiImportExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;
        if self.shared_with_other_environments.is_none() {
            serialize_if_present(&mut state, "desc", &self.desc)?;
            serialize_if_present(&mut state, "path", &self.path)?;
            serialize_if_present(&mut state, "text", &self.text)?;
            state.serialize_entry("interest_type", &self.interest_type)?;
        } else {
            state.serialize_entry(
                "shared_with_other_environments",
                if self.desc.is_empty() {
                    &self.text
                } else {
                    &self.desc
                },
            )?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
struct ProjectNoteImportExport {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub contents: String,
    #[serde(default)]
    shared_with_other_environments: Option<String>,
}

impl Serialize for ProjectNoteImportExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;
        if self.shared_with_other_environments.is_none() {
            serialize_if_present(&mut state, "title", &self.title)?;
            serialize_if_present(&mut state, "contents", &self.contents)?;
        } else {
            state.serialize_entry("shared_with_other_environments", &self.title)?;
        }
        state.end()
    }
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
    servers: Vec<ServerWithItemsImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_links: Vec<ServerLink>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    project_pois: Vec<ProjectPoiImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    project_notes: Vec<ProjectNoteImportExport>,
}

#[derive(Serialize, Deserialize)]
struct ServerWithItemsImportExport {
    server: ServerImportExport,
    items: ServerGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    items_in_groups: HashMap<String, ServerGroupImportExport>,
}

#[derive(Serialize, Deserialize)]
struct ServerGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_pois: Vec<ServerPointOfInterest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_websites: Vec<ServerWebsiteImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    server_databases: Vec<ServerDatabaseImportExport>,
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
    let project_id = insert_row(
        sql_conn,
        diesel::insert_into(prj::project).values(changeset),
    )
    .map_err(to_boxed_stderr)?;
    let mut unprocessed_websites = vec![];

    if let Some(dev_env) = decoded.development_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvDevelopment,
            &dev_env,
        )?);
    }
    if let Some(stg_env) = decoded.staging_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvStage,
            &stg_env,
        )?);
    }
    if let Some(uat_env) = decoded.uat_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvUat,
            &uat_env,
        )?);
    }
    if let Some(prod_env) = decoded.prod_environment {
        unprocessed_websites.extend(import_project_env_first_pass(
            sql_conn,
            project_id,
            EnvironmentType::EnvProd,
            &prod_env,
        )?);
    }
    for unprocessed_website in unprocessed_websites {
        import_server_website(sql_conn, &unprocessed_website)?;
    }
    Ok(())
}

struct UnprocessedWebsite {
    server_id: i32,
    group_name: Option<String>,
    website: ServerWebsiteImportExport,
}

/// in the first pass we don't do server links and
/// server websites. server links can link to other
/// servers and websites and link to server databases.
///
/// we want to import all the potential link targets
/// in the first pass so the links are resolved, if
/// at all possible, when we'll process the second pass.
fn import_project_env_first_pass(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    project_env: &ProjectEnvImportExport,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    let mut unprocessed_websites =
        import_project_env_group_first_pass(sql_conn, project_id, &project_env.items, env, None)?;

    for (group, items) in &project_env.items_in_groups {
        unprocessed_websites.append(&mut import_project_env_group_first_pass(
            sql_conn,
            project_id,
            &items,
            env,
            Some(group),
        )?);
    }

    Ok(unprocessed_websites)
}

fn import_project_env_group_first_pass(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    items: &ProjectEnvGroupImportExport,
    env: EnvironmentType,
    group_name: Option<&str>,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    for project_poi in &items.project_pois {
        import_project_poi(sql_conn, project_id, group_name, project_poi)?;
    }
    for project_note in &items.project_notes {
        import_project_note(sql_conn, project_id, group_name, env, project_note)?;
    }

    items
        .servers
        .iter()
        .map(|server| import_server(sql_conn, project_id, env, group_name, server))
        .collect::<ImportResult<Vec<_>>>()
        // TODO next line is horrible
        .map(|v| v.into_iter().flat_map(|x| x).collect::<Vec<_>>())
}

fn import_project_poi(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    group_name: Option<&str>,
    project_poi: &ProjectPoiImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
    if project_poi.shared_with_other_environments.is_some() {
        return Ok(());
    }
    let changeset = (
        prj_poi::desc.eq(&project_poi.desc),
        prj_poi::path.eq(&project_poi.path),
        prj_poi::text.eq(&project_poi.text),
        prj_poi::group_name.eq(group_name),
        prj_poi::interest_type.eq(project_poi.interest_type),
        prj_poi::project_id.eq(project_id),
    );
    insert_row(
        sql_conn,
        diesel::insert_into(prj_poi::project_point_of_interest).values(changeset),
    )
    .map_err(to_boxed_stderr)
    .map(|_| ())
}

fn import_project_note(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    group_name: Option<&str>,
    env: EnvironmentType,
    project_note: &ProjectNoteImportExport,
) -> ImportResult<()> {
    use projectpadsql::schema::project_note::dsl as prj_note;
    if let Some(shared_title) = project_note.shared_with_other_environments.as_ref() {
        // update the row to mark that it's active
        // also for this environment
        let note_id_to_update = prj_note::project_note
            .select(prj_note::id)
            .filter(
                prj_note::title
                    .eq(&shared_title)
                    .and(sqlite_is(prj_note::group_name, group_name))
                    .and(prj_note::project_id.eq(project_id)),
            )
            .first::<i32>(sql_conn)?;
        let what = prj_note::project_note.filter(prj_note::id.eq(note_id_to_update));

        let update = match env {
            // dev is the first, normally we come here at the 2nd
            // environment the earlier => skip it
            EnvironmentType::EnvStage => diesel::update(what)
                .set(prj_note::has_stage.eq(true))
                .execute(sql_conn),
            EnvironmentType::EnvUat => diesel::update(what)
                .set(prj_note::has_uat.eq(true))
                .execute(sql_conn),
            EnvironmentType::EnvProd => diesel::update(what)
                .set(prj_note::has_prod.eq(true))
                .execute(sql_conn),
            _ => unreachable!(),
        }
        .map(|_| ())?;
        Ok(update)
    } else {
        // this note was not imported yet, import it the first time
        let changeset = (
            prj_note::title.eq(&project_note.title),
            prj_note::contents.eq(&project_note.contents),
            prj_note::has_dev.eq(env == EnvironmentType::EnvDevelopment),
            prj_note::has_stage.eq(env == EnvironmentType::EnvStage),
            prj_note::has_uat.eq(env == EnvironmentType::EnvUat),
            prj_note::has_prod.eq(env == EnvironmentType::EnvProd),
            prj_note::group_name.eq(group_name),
            prj_note::project_id.eq(project_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(prj_note::project_note).values(changeset),
        )
        .map_err(to_boxed_stderr)
        .map(|_| ())
    }
}

fn import_server(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
    env: EnvironmentType,
    group_name: Option<&str>,
    server: &ServerWithItemsImportExport,
) -> ImportResult<Vec<UnprocessedWebsite>> {
    use projectpadsql::schema::server::dsl as srv;
    let changeset = (
        srv::desc.eq(&server.server.0.desc),
        srv::is_retired.eq(server.server.0.is_retired),
        srv::ip.eq(&server.server.0.ip),
        srv::text.eq(&server.server.0.text),
        srv::group_name.eq(group_name),
        srv::username.eq(&server.server.0.username),
        srv::password.eq(&server.server.0.password),
        srv::auth_key.eq(server.server.0.auth_key.as_ref()), // TODO probably stored elsewhere
        srv::auth_key_filename.eq(server.server.0.auth_key_filename.as_ref()),
        srv::server_type.eq(server.server.0.server_type),
        srv::access_type.eq(server.server.0.access_type),
        srv::environment.eq(env),
        srv::project_id.eq(project_id),
    );
    let server_id = insert_row(sql_conn, diesel::insert_into(srv::server).values(changeset))
        .map_err(to_boxed_stderr)?;

    import_server_items(sql_conn, server_id, None, &server.items)?;
    for (group_name, items) in &server.items_in_groups {
        import_server_items(sql_conn, server_id, Some(group_name), items)?;
    }

    let mut unprocessed_websites: Vec<_> = server
        .items
        .server_websites
        .iter()
        .map(|w| UnprocessedWebsite {
            server_id,
            group_name: None,
            website: w.clone(),
        })
        .collect();
    unprocessed_websites.extend(server.items_in_groups.iter().flat_map(|(k, v)| {
        v.server_websites.iter().map(move |www| UnprocessedWebsite {
            server_id,
            group_name: Some(k.to_string()),
            website: www.clone(),
        })
    }));
    Ok(unprocessed_websites)
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
            srv_db::desc.eq(&db.0.desc),
            srv_db::name.eq(&db.0.name),
            srv_db::group_name.eq(group_name),
            srv_db::text.eq(&db.0.text),
            srv_db::username.eq(&db.0.username),
            srv_db::password.eq(&db.0.password),
            srv_db::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_db::server_database).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for note in &items.server_notes {
        use projectpadsql::schema::server_note::dsl as srv_note;
        let changeset = (
            srv_note::title.eq(&note.title),
            srv_note::group_name.eq(group_name),
            srv_note::contents.eq(&note.contents),
            srv_note::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_note::server_note).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for poi in &items.server_pois {
        use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
        let changeset = (
            srv_poi::desc.eq(&poi.desc),
            srv_poi::path.eq(&poi.path),
            srv_poi::text.eq(&poi.text),
            srv_poi::group_name.eq(group_name),
            srv_poi::interest_type.eq(poi.interest_type),
            srv_poi::run_on.eq(poi.run_on),
            srv_poi::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_poi::server_point_of_interest).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    for user in &items.server_extra_users {
        use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
        let changeset = (
            srv_usr::desc.eq(&user.desc),
            srv_usr::group_name.eq(group_name),
            srv_usr::username.eq(&user.username),
            srv_usr::password.eq(&user.password),
            srv_usr::auth_key.eq(&user.auth_key), // TODO stored elsewhere?
            srv_usr::auth_key_filename.eq(&user.auth_key_filename),
            srv_usr::server_id.eq(server_id),
        );
        insert_row(
            sql_conn,
            diesel::insert_into(srv_usr::server_extra_user_account).values(changeset),
        )
        .map_err(to_boxed_stderr)?;
    }
    // server websites are handled in the second pass
    Ok(())
}

fn import_server_website(
    sql_conn: &diesel::SqliteConnection,
    website_info: &UnprocessedWebsite,
) -> ImportResult<()> {
    use projectpadsql::schema::server_website::dsl as srv_www;
    let new_databaseid = website_info
        .website
        .server_database
        .as_ref()
        .and_then(|db_path| get_new_databaseid(sql_conn, db_path).ok());
    let changeset = (
        srv_www::desc.eq(&website_info.website.desc),
        srv_www::url.eq(&website_info.website.url),
        srv_www::text.eq(&website_info.website.text),
        srv_www::group_name.eq(website_info.group_name.as_ref()),
        srv_www::username.eq(&website_info.website.username),
        srv_www::password.eq(&website_info.website.password),
        srv_www::server_database_id.eq(new_databaseid),
        srv_www::server_id.eq(website_info.server_id),
    );
    insert_row(
        sql_conn,
        diesel::insert_into(srv_www::server_website).values(changeset),
    )
    .map_err(to_boxed_stderr)?;
    Ok(())
}

fn get_new_databaseid(
    sql_conn: &diesel::SqliteConnection,
    db_path: &ServerDatabasePath,
) -> ImportResult<i32> {
    use projectpadsql::schema::server_database::dsl as srv_db;
    if let Some(db_id) = db_path.database_id {
        return Ok(db_id);
    }

    // since database_id is not defined, i know that database_desc is.

    // first find the server id
    let server_id: i32 = match db_path.server_id {
        Some(id) => id,
        None => {
            // no server id, must find the server using desc, environment and project name
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::server::dsl as srv;
            srv::server
                .inner_join(prj::project)
                .select(srv::id)
                .filter(
                    prj::name
                        .eq(&db_path.project_name)
                        .and(srv::environment.eq(db_path.environment))
                        // we know server_desc is present, because server_id is not.
                        .and(srv::desc.eq(db_path.server_desc.as_ref().unwrap())),
                )
                .first(sql_conn)
                .map_err(|e| e.to_string())?
        }
    };
    Ok(srv_db::server_database
        .select(srv_db::id)
        .filter(
            srv_db::desc
                .eq(db_path.database_desc.as_ref().unwrap())
                .and(srv_db::server_id.eq(server_id)),
        )
        .first(sql_conn)
        .map_err(|e| e.to_string())?)
}

pub fn export_project(sql_conn: &diesel::SqliteConnection, project: &Project) {
    // if I export a 7zip i can export project icons and attachments in the zip too...
    let group_names = projectpadsql::get_project_group_names(sql_conn, project.id);
    let mut is_first_env = true;

    let development_environment = if project.has_dev {
        let e = Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvDevelopment,
            is_first_env,
            &group_names,
        ));
        is_first_env = false;
        e
    } else {
        None
    };

    let staging_environment = if project.has_stage {
        let e = Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvStage,
            is_first_env,
            &group_names,
        ));
        is_first_env = false;
        e
    } else {
        None
    };

    let uat_environment = if project.has_uat {
        let e = Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvUat,
            is_first_env,
            &group_names,
        ));
        is_first_env = false;
        e
    } else {
        None
    };

    let prod_environment = if project.has_prod {
        Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvProd,
            is_first_env,
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
                        l.replace(r#"\""#, r#"""#)
                            .replace(r#"\\"#, r#"\"#)
                            .replace(r#"\t"#, "\t")
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
    is_first_env: bool,
    group_names: &[String],
) -> ProjectEnvImportExport {
    let items = export_env_group(sql_conn, project, env, is_first_env, None);

    let items_in_groups = group_names
        .iter()
        .map(|gn| {
            let group = export_env_group(sql_conn, project, env, is_first_env, Some(gn));
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
    is_first_env: bool,
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

    let project_notes_import_export = project_notes
        .into_iter()
        .map(|n| {
            // we don't want to repeat the same note, once for each environment.
            // is this the first time we export this note?
            // YES => we export the full note
            // NO => we will display just "shared"
            let is_first_env_for_this_note = match (
                n.has_dev && project.has_dev,
                n.has_stage && project.has_stage,
                n.has_uat && project.has_uat,
                env,
            ) {
                (_, _, _, EnvironmentType::EnvDevelopment) => true,
                (false, _, _, EnvironmentType::EnvStage) => true,
                (false, false, _, EnvironmentType::EnvUat) => true,
                (false, false, false, EnvironmentType::EnvProd) => true,
                _ => false,
            };
            ProjectNoteImportExport {
                title: n.title.clone(),
                contents: n.contents,
                shared_with_other_environments: if is_first_env_for_this_note {
                    None
                } else {
                    Some(n.title)
                },
            }
        })
        .collect();

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

    let project_pois_export = project_pois
        .into_iter()
        .map(|ppoi| ProjectPoiImportExport {
            desc: ppoi.desc.clone(),
            path: ppoi.path,
            text: ppoi.text.clone(),
            interest_type: ppoi.interest_type,
            shared_with_other_environments: if is_first_env {
                None
            } else {
                Some(if ppoi.desc.is_empty() {
                    ppoi.text
                } else {
                    ppoi.desc
                })
            },
        })
        .collect();

    ProjectEnvGroupImportExport {
        servers: srvs
            .into_iter()
            .map(|s| export_server(sql_conn, s))
            .collect(),
        server_links,
        project_pois: project_pois_export,
        project_notes: project_notes_import_export,
    }
}

fn export_server(
    sql_conn: &diesel::SqliteConnection,
    server: Server,
) -> ServerWithItemsImportExport {
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
    ServerWithItemsImportExport {
        server: ServerImportExport(server),
        items,
        items_in_groups,
    }
}

fn export_server_items(
    sql_conn: &diesel::SqliteConnection,
    server: &Server,
    group_name: Option<&str>,
) -> ServerGroupImportExport {
    // TODO should return Result instead of unwrapping
    use projectpadsql::schema::server_database::dsl as srv_db;
    use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
    use projectpadsql::schema::server_note::dsl as srv_note;
    use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
    use projectpadsql::schema::server_website::dsl as srv_www;
    let server_pois = srv_poi::server_point_of_interest
        .filter(
            srv_poi::server_id
                .eq(server.id)
                .and(sqlite_is(srv_poi::group_name, group_name)),
        )
        .order(srv_poi::desc.asc())
        .load::<ServerPointOfInterest>(sql_conn)
        .unwrap();

    let server_websites = srv_www::server_website
        .filter(
            srv_www::server_id
                .eq(server.id)
                .and(sqlite_is(srv_www::group_name, group_name)),
        )
        .order(srv_www::desc.asc())
        .load::<ServerWebsite>(sql_conn)
        .unwrap()
        .into_iter()
        .map(|www| to_server_website_import_export(sql_conn, www).unwrap())
        .collect();

    let server_databases = srv_db::server_database
        .filter(
            srv_db::server_id
                .eq(server.id)
                .and(sqlite_is(srv_db::group_name, group_name)),
        )
        .order(srv_db::desc.asc())
        .load::<ServerDatabase>(sql_conn)
        .unwrap()
        .into_iter()
        .map(ServerDatabaseImportExport)
        .collect();

    let server_notes = srv_note::server_note
        .filter(
            srv_note::server_id
                .eq(server.id)
                .and(sqlite_is(srv_note::group_name, group_name)),
        )
        .order(srv_note::title.asc())
        .load::<ServerNote>(sql_conn)
        .unwrap();

    let server_extra_users = srv_usr::server_extra_user_account
        .filter(
            srv_usr::server_id
                .eq(server.id)
                .and(sqlite_is(srv_usr::group_name, group_name)),
        )
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

fn to_server_website_import_export(
    sql_conn: &SqliteConnection,
    website: ServerWebsite,
) -> ImportResult<ServerWebsiteImportExport> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::server::dsl as srv;
    use projectpadsql::schema::server_database::dsl as srv_db;
    let server_database = match website.server_database_id {
        Some(id) => {
            let (db, (srv, prj)) = srv_db::server_database
                .inner_join(srv::server.inner_join(prj::project))
                .filter(srv_db::id.eq(id))
                .first::<(ServerDatabase, (Server, Project))>(sql_conn)?;
            Some(ServerDatabasePath {
                project_name: prj.name,
                environment: srv.environment,
                server_id: if srv.desc.is_empty() {
                    Some(srv.id)
                } else {
                    None
                },
                server_desc: Some(srv.desc).filter(|d| !d.is_empty()),
                database_id: if db.desc.is_empty() {
                    Some(db.id)
                } else {
                    None
                },
                database_desc: Some(db.desc).filter(|d| !d.is_empty()),
            })
        }
        None => None,
    };

    Ok(ServerWebsiteImportExport {
        desc: website.desc,
        url: website.url,
        text: website.text,
        username: website.username,
        password: website.password,
        server_database,
    })
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
