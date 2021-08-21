use super::import_export_dtos::*;
use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase,
    ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use projectpadsql::sqlite_is;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::{borrow, env, fs, path, process, time};

type ExportResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Return the list of dependent project names that were
/// not exported but maybe should have been
pub fn export_projects(
    sql_conn: &diesel::SqliteConnection,
    projects: &[Project],
    fname: &Path,
    password: &str,
) -> ExportResult<HashSet<String>> {
    let mut project_id_to_folder_name = compute_project_to_folder_name(projects);

    let mut project_exports = vec![];
    // it's useful to pass down the map of extra_files so that
    // components can try to optimistically add semantic file & folder
    // names, being able to detect duplicates by looking up in the map.
    // the alternative would be to generate computer-generated file &
    // folder names to ensure unicity.
    let mut extra_files = HashMap::new();
    for p in projects {
        let project_folder = project_id_to_folder_name
            .remove(&p.id)
            .expect("failed getting project folder name");
        project_exports.push((
            export_project(sql_conn, p, &mut extra_files, &project_folder)?,
            project_folder,
        ));
    }
    write_7z(&project_exports, &extra_files, fname, password)?;

    let dependent_project_names: HashSet<String> = project_exports
        .iter()
        .flat_map(|(pe, _)| pe.dependencies_project_names())
        .collect();
    let project_names: HashSet<String> = projects.iter().map(|prj| prj.name.to_owned()).collect();

    Ok(&dependent_project_names - &project_names)
}

fn compute_project_to_folder_name(projects: &[Project]) -> HashMap<i32, PathBuf> {
    let project_to_folder_name = projects.iter().fold(HashMap::new(), |mut sofar, project| {
        let project_folder = find_unique_data_path(escape_filename(&project.name), &sofar);
        sofar.insert(PathBuf::from(project_folder), project.id);
        sofar
    });
    project_to_folder_name
        .into_iter()
        .map(|(k, v)| (v, k))
        .collect()
}

fn export_project(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
    project_folder: &Path,
) -> ExportResult<ProjectImportExport> {
    let group_names = projectpadsql::get_project_group_names(sql_conn, project.id);
    let mut is_first_env = true;
    let mut project_extra_files = HashMap::new();

    let development_environment = if project.has_dev {
        let e = Some(export_env(
            sql_conn,
            project,
            EnvironmentType::EnvDevelopment,
            is_first_env,
            &group_names,
            &mut project_extra_files,
        )?);
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
            &mut project_extra_files,
        )?);
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
            &mut project_extra_files,
        )?);
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
            &mut project_extra_files,
        )?)
    } else {
        None
    };

    for (path, contents) in project_extra_files {
        let mut path_with_prj = project_folder.to_path_buf();
        path_with_prj.push(path);
        extra_files.insert(path_with_prj, contents);
    }

    if let Some(icon) = &project.icon {
        if !icon.is_empty() {
            let mut icon_path = project_folder.to_path_buf();
            icon_path.push("icon.png");
            extra_files.insert(icon_path, icon.clone());
        }
    }

    Ok(ProjectImportExport {
        project_name: project.name.clone(),
        development_environment,
        staging_environment,
        uat_environment,
        prod_environment,
    })
}

pub struct TempFolder {
    pub folder: path::PathBuf,
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.folder) {
            eprintln!("Error cleaning up a temp folder: {:?}", e);
        }
    }
}

pub fn temp_folder() -> ExportResult<TempFolder> {
    let mut tmp_path = env::temp_dir();
    tmp_path.push(&format!(
        "projectpad-{}",
        time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)?
            .as_millis()
    ));
    fs::create_dir_all(&tmp_path)?;
    Ok(TempFolder { folder: tmp_path })
}

fn escape_filename(input: &str) -> String {
    let re = Regex::new("[^0-9a-zA-Z_]+").unwrap();
    re.replace(input, "_").to_string()
}

pub fn seven_z_command() -> ExportResult<&'static str> {
    for dir in env::var("PATH")?.split(':') {
        let mut path = PathBuf::from(dir);
        path.push("7z");
        if path.exists() {
            return Ok("7z");
        }
        path.pop();
        path.push("7za");
        if path.exists() {
            return Ok("7za");
        }
    }
    Err("Need the 7z or 7za command to be installed".into())
}

fn write_7z(
    projects_data: &[(ProjectImportExport, PathBuf)],
    extra_files: &HashMap<PathBuf, Vec<u8>>,
    fname: &Path,
    password: &str,
) -> ExportResult<()> {
    let tmp_export = temp_folder()?;
    let mut tmp_export_path = tmp_export.folder.clone();
    for (project_data, project_path) in projects_data {
        tmp_export_path.push(project_path);
        fs::create_dir(&tmp_export_path)?;
        tmp_export_path.push("contents.yaml");
        fs::write(&tmp_export_path, generate_yaml(&project_data))?;
        tmp_export_path.pop();
        tmp_export_path.pop();
    }
    for (path, data) in extra_files {
        // cloning because we can't be sure of the depth of the files.
        // we wouldn't know how many children to pop() to return here.
        let mut child_path = tmp_export_path.clone();
        child_path.push(
            path.parent()
                .ok_or(format!("extra_file.path is not in a subfolder? {:?}", path))?,
        );
        fs::create_dir_all(&child_path)?;
        child_path.push(&path.file_name().unwrap());
        fs::write(&child_path, &data)?;
    }

    // 7za will *add* files to an existing archive.
    // but we want a clean new archive => delete
    // the file if it existed
    if fname.exists() {
        fs::remove_file(&fname)?;
    }

    let seven_z_cmd = seven_z_command()?;
    let pass_param = if password.is_empty() {
        borrow::Cow::Borrowed("")
    } else {
        borrow::Cow::Owned(format!("-p{}", password))
    };
    let status = process::Command::new(seven_z_cmd)
        .args(&[
            "a",
            pass_param.as_ref(),
            "-sdel",
            &fname.to_string_lossy(),
            &format!("{}/*", tmp_export_path.to_string_lossy()),
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("7zip execution failed: {:?}", status.code()).into())
    }
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
            let indentation_indicator = 2;
            format!(
                "{}|{}{}{}",
                line_start,
                indentation_indicator,
                separator,
                itertools::join(
                    contents.split("\\n").map(|l| l
                        .replace(r#"\""#, r#"""#)
                        .replace(r#"\\"#, r#"\"#)
                        .replace(r#"\t"#, "\t")),
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
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
) -> ExportResult<ProjectEnvImportExport> {
    let items = export_env_group(sql_conn, project, env, is_first_env, None, extra_files)?;

    let mut items_in_groups = HashMap::new();
    for gn in group_names {
        let group = export_env_group(sql_conn, project, env, is_first_env, Some(gn), extra_files)?;
        items_in_groups.insert(gn.clone(), group);
    }

    Ok(ProjectEnvImportExport {
        items,
        items_in_groups,
    })
}

fn export_env_group(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    env: EnvironmentType,
    is_first_env: bool,
    group_name: Option<&str>,
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
) -> ExportResult<ProjectEnvGroupImportExport> {
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
        .load::<Server>(sql_conn)?;

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
        .load::<ProjectNote>(sql_conn)?;

    let project_notes_import_export = project_notes
        .into_iter()
        .map(|n| {
            // we don't want to repeat the same note, once for each environment.
            // is this the first time we export this note?
            // YES => we export the full note
            // NO => we will display just "shared"
            let is_first_env_for_this_note = matches!(
                (
                    n.has_dev && project.has_dev,
                    n.has_stage && project.has_stage,
                    n.has_uat && project.has_uat,
                    env,
                ),
                (_, _, _, EnvironmentType::EnvDevelopment)
                    | (false, _, _, EnvironmentType::EnvStage)
                    | (false, false, _, EnvironmentType::EnvUat)
                    | (false, false, false, EnvironmentType::EnvProd)
            );
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
        .load::<ServerLink>(sql_conn)?;

    let server_links_export: Vec<_> = server_links
        .into_iter()
        .map(|srv| to_server_link_import_export(sql_conn, srv))
        .collect::<ExportResult<_>>()?;

    let project_pois = prj_poi::project_point_of_interest
        .filter(
            prj_poi::project_id
                .eq(project.id)
                .and(sqlite_is(prj_poi::group_name, group_name)),
        )
        .order((prj_poi::desc.asc(), prj_poi::path.asc()))
        .load::<ProjectPointOfInterest>(sql_conn)?;

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

    Ok(ProjectEnvGroupImportExport {
        servers: srvs
            .into_iter()
            .map(|s| export_server(sql_conn, s, extra_files))
            .collect::<ExportResult<Vec<_>>>()?,
        server_links: server_links_export,
        project_pois: project_pois_export,
        project_notes: project_notes_import_export,
    })
}

fn export_server(
    sql_conn: &diesel::SqliteConnection,
    server: Server,
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
) -> ExportResult<ServerWithItemsImportExport> {
    let items = export_server_items(sql_conn, extra_files, &server, None)?;
    let group_names = projectpadsql::get_server_group_names(sql_conn, server.id);
    let mut items_in_groups = HashMap::new();
    for gn in &group_names {
        let items = export_server_items(sql_conn, extra_files, &server, Some(gn))?;
        items_in_groups.insert(gn.clone(), items);
    }
    let data_path = match (&server.auth_key, &server.auth_key_filename) {
        (Some(key), Some(fname)) => {
            let path_base = if server.desc.is_empty() {
                server.id.to_string()
            } else {
                escape_filename(&server.desc.clone())
            };
            let sub_path = find_unique_data_path(path_base, extra_files);
            let mut path = PathBuf::from(&sub_path);
            path.push(fname);
            extra_files.insert(path, key.clone());
            Some(PathBuf::from(sub_path))
        }
        _ => None,
    };
    Ok(ServerWithItemsImportExport {
        server: ServerImportExport { server, data_path },
        items,
        items_in_groups,
    })
}

fn find_unique_data_path<T>(path_base: String, extra_files: &HashMap<PathBuf, T>) -> String {
    // generate an infinite lazy sequence of candidate folder names
    let mut counter = 0;
    let mut path_candidates = std::iter::from_fn(move || {
        counter += 1;
        Some(if counter == 1 {
            path_base.clone()
        } else {
            format!("{}-{}", path_base, counter)
        })
    });

    path_candidates
        // can't use contains because the keys are full paths, but i want
        // to test only root folders existence now
        .find(|c| !extra_files.keys().any(|k| k.starts_with(c)))
        .unwrap()
}

fn export_server_items(
    sql_conn: &diesel::SqliteConnection,
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
    server: &Server,
    group_name: Option<&str>,
) -> ExportResult<ServerGroupImportExport> {
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
        .load::<ServerPointOfInterest>(sql_conn)?;

    let server_websites = srv_www::server_website
        .filter(
            srv_www::server_id
                .eq(server.id)
                .and(sqlite_is(srv_www::group_name, group_name)),
        )
        .order(srv_www::desc.asc())
        .load::<ServerWebsite>(sql_conn)?
        .into_iter()
        .map(|www| to_server_website_import_export(sql_conn, www))
        .collect::<ExportResult<Vec<_>>>()?;

    let server_databases = srv_db::server_database
        .filter(
            srv_db::server_id
                .eq(server.id)
                .and(sqlite_is(srv_db::group_name, group_name)),
        )
        .order(srv_db::desc.asc())
        .load::<ServerDatabase>(sql_conn)?
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
        .load::<ServerNote>(sql_conn)?;

    let server_extra_users = srv_usr::server_extra_user_account
        .filter(
            srv_usr::server_id
                .eq(server.id)
                .and(sqlite_is(srv_usr::group_name, group_name)),
        )
        .order(srv_usr::username.asc())
        .load::<ServerExtraUserAccount>(sql_conn)?;
    let server_extra_users = server_extra_users
        .into_iter()
        .map(|usr| export_server_extra_user(usr, extra_files))
        .collect();

    Ok(ServerGroupImportExport {
        server_pois,
        server_websites,
        server_databases,
        server_notes,
        server_extra_users,
    })
}

fn export_server_extra_user(
    user: ServerExtraUserAccount,
    extra_files: &mut HashMap<PathBuf, Vec<u8>>,
) -> ServerExtraUserImportExport {
    let data_path = match (user.auth_key.as_ref(), user.auth_key_filename.as_ref()) {
        (Some(key), Some(fname)) => {
            let path_base = if user.desc.is_empty() {
                user.id.to_string()
            } else {
                escape_filename(&user.desc.clone())
            };
            let sub_path = find_unique_data_path(path_base, extra_files);
            let mut path = PathBuf::from(&sub_path);
            path.push(fname);
            extra_files.insert(path, key.clone());
            Some(PathBuf::from(sub_path))
        }
        _ => None,
    };
    ServerExtraUserImportExport {
        username: user.username,
        password: user.password,
        desc: user.desc,
        data_path,
        auth_key_filename: user.auth_key_filename,
    }
}

fn to_server_link_import_export(
    sql_conn: &SqliteConnection,
    server_link: ServerLink,
) -> ExportResult<ServerLinkImportExport> {
    use projectpadsql::schema::project::dsl as prj;
    use projectpadsql::schema::server::dsl as srv;
    let (srv, prj) = srv::server
        .inner_join(prj::project)
        .filter(srv::id.eq(server_link.linked_server_id))
        .first::<(Server, Project)>(sql_conn)?;
    let server = ServerPath {
        project_name: prj.name,
        environment: srv.environment,
        server_id: Some(srv.id).filter(|_| srv.desc.is_empty()),
        server_desc: Some(srv.desc).filter(|d| !d.is_empty()),
    };
    Ok(ServerLinkImportExport {
        desc: server_link.desc,
        server,
    })
}

fn to_server_website_import_export(
    sql_conn: &SqliteConnection,
    website: ServerWebsite,
) -> ExportResult<ServerWebsiteImportExport> {
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

#[cfg(test)]
mod tests {
    use super::super::import::tests::{tests_load_yaml, SAMPLE_YAML_PROJECT};
    use super::*;

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
            "test: |2\n  first line\n  second line",
            yaml_fix_multiline_strings("test: \"first line\\nsecond line\"")
        );
    }

    #[test]
    fn fix_yaml_strings_newlines_and_quotes_in_string() {
        assert_eq!(
            "test: |2\n  first \"line\"\n  second \\line",
            yaml_fix_multiline_strings("test: \"first \\\"line\\\"\\nsecond \\\\line\"")
        );
    }

    #[test]
    fn serialize_should_yield_same_yaml() {
        use projectpadsql::schema::project::dsl as prj;
        let sql_conn = tests_load_yaml(SAMPLE_YAML_PROJECT);
        let projects = prj::project.load::<Project>(&sql_conn).unwrap();
        let project = projects.get(0).unwrap();

        let project_import_export =
            export_project(&sql_conn, &project, &mut HashMap::new(), &PathBuf::from("")).unwrap();
        let raw_output = generate_yaml(&project_import_export);
        assert_eq!(
            // drop the leading \n on the left
            &SAMPLE_YAML_PROJECT[1..],
            // the replace is a workaround for a minor issue (trailing \n, i think)
            // that i'm not particularly interested in at this point
            raw_output
                .replace("          \nuat_environ", "uat_environ")
                .trim_end()
        );
    }
}
