use projectpadsql::models::{
    EnvironmentType, InterestType, Server, ServerAccessType, ServerDatabase,
    ServerExtraUserAccount, ServerNote, ServerPointOfInterest, ServerType,
};
use serde::de;
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

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

pub struct ServerImportExport {
    pub server: Server,
    pub data_path: Option<PathBuf>,
}

impl Serialize for ServerImportExport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = &self.server;
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

        serialize_if_some(&mut state, "data_folder", &self.data_path)?; // TODO rename?
        serialize_if_some(&mut state, "auth_key_filename", &s.auth_key_filename)?;
        state.serialize_entry("server_type", &s.server_type)?;
        state.serialize_entry("access_type", &s.access_type)?;
        serialize_if_some(&mut state, "ssh_tunnel_port", &s.ssh_tunnel_port)?;
        // TODO through_server_id

        state.end()
    }
}

struct ServerImportExportMapVisitor {}

impl<'de> Visitor<'de> for ServerImportExportMapVisitor {
    type Value = ServerImportExport;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("ServerImportExport")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = HashMap::<String, String>::new(); // TODO do I need String for kv?
        while let Some((key, value)) = access.next_entry()? {
            map.insert(key, value);
        }
        Ok(ServerImportExport {
            server: Server {
                id: 0,
                desc: map.get("desc").cloned().unwrap_or_else(|| "".to_string()),
                ip: map.get("ip").cloned().unwrap_or_else(|| "".to_string()),
                text: map.get("text").cloned().unwrap_or_else(|| "".to_string()),
                is_retired: map.get("is_retired").map(|r| r == "true").unwrap_or(false),
                username: map
                    .get("username")
                    .cloned()
                    .unwrap_or_else(|| "".to_string()),
                password: map
                    .get("password")
                    .cloned()
                    .unwrap_or_else(|| "".to_string()),
                auth_key: None,
                auth_key_filename: map
                    .get("auth_key_filename")
                    .map(|f| Some(f.clone()))
                    .unwrap_or(None),
                server_type: map
                    .get("server_type")
                    .and_then(|s| ServerType::from_str(s.as_str()).ok())
                    .ok_or_else(|| de::Error::custom("missing or invalid server_type"))?,
                access_type: map
                    .get("access_type")
                    .and_then(|s| ServerAccessType::from_str(s.as_str()).ok())
                    .ok_or_else(|| de::Error::custom("missing or invalid access_type"))?,
                ssh_tunnel_port: None,
                ssh_tunnel_through_server_id: None,
                environment: EnvironmentType::EnvDevelopment,
                group_name: None,
                project_id: 0,
            },
            data_path: map
                .get("data_folder") // TODO rename? (path_folder vs data_folder)
                .map(|f| Some(PathBuf::from(f)))
                .unwrap_or(None),
        })
    }
}

impl<'de> Deserialize<'de> for ServerImportExport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ServerImportExportMapVisitor {})
    }
}

#[derive(Deserialize)]
pub struct ServerDatabaseImportExport(pub ServerDatabase);

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
pub struct ServerDatabasePath {
    pub project_name: String,
    pub environment: EnvironmentType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_desc: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerWebsiteImportExport {
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_database: Option<ServerDatabasePath>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerLinkImportExport {
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub desc: String,
    pub server: ServerPath,
}

#[derive(Serialize, Deserialize)]
pub struct ServerPath {
    pub project_name: String,
    pub environment: EnvironmentType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub server_desc: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectImportExport {
    pub project_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub development_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub staging_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uat_environment: Option<ProjectEnvImportExport>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prod_environment: Option<ProjectEnvImportExport>,
}

impl ProjectImportExport {
    pub fn dependencies_project_names(&self) -> HashSet<String> {
        let mut deps = HashSet::new();

        if let Some(env) = &self.development_environment {
            deps.extend(env.dependencies_project_names());
        }
        if let Some(env) = &self.staging_environment {
            deps.extend(env.dependencies_project_names());
        }
        if let Some(env) = &self.uat_environment {
            deps.extend(env.dependencies_project_names());
        }
        if let Some(env) = &self.prod_environment {
            deps.extend(env.dependencies_project_names());
        }
        deps
    }
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
pub struct ProjectPoiImportExport {
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub interest_type: InterestType,
    #[serde(default)]
    pub shared_with_other_environments: Option<String>,
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
pub struct ProjectNoteImportExport {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub contents: String,
    #[serde(default)]
    pub shared_with_other_environments: Option<String>,
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
pub struct ProjectEnvImportExport {
    pub items: ProjectEnvGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub items_in_groups: HashMap<String, ProjectEnvGroupImportExport>,
}

impl ProjectEnvImportExport {
    fn dependencies_project_names(&self) -> HashSet<String> {
        let mut result = self.items.dependencies_project_names();
        for group in self.items_in_groups.values() {
            result.extend(group.dependencies_project_names())
        }
        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProjectEnvGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub servers: Vec<ServerWithItemsImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_links: Vec<ServerLinkImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub project_pois: Vec<ProjectPoiImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub project_notes: Vec<ProjectNoteImportExport>,
}

impl ProjectEnvGroupImportExport {
    fn dependencies_project_names(&self) -> HashSet<String> {
        // https://github.com/rust-lang/rfcs/issues/2023
        // https://users.rust-lang.org/t/intersecting-multiple-hashset-string/34176
        // https://users.rust-lang.org/t/hashset-union-expecting-hashset-string-got-hashset-string/24584/11 <---------
        let mut linked_projects = HashSet::new();
        for server in &self.servers {
            linked_projects.extend(server.dependencies_project_names());
        }
        for server_link in &self.server_links {
            linked_projects.insert(server_link.server.project_name.clone());
        }
        linked_projects
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerWithItemsImportExport {
    pub server: ServerImportExport,
    pub items: ServerGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub items_in_groups: HashMap<String, ServerGroupImportExport>,
}

impl ServerWithItemsImportExport {
    fn dependencies_project_names(&self) -> HashSet<String> {
        let mut result = self.items.dependencies_project_names();
        for group in self.items_in_groups.values() {
            result.extend(group.dependencies_project_names());
        }
        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerGroupImportExport {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_pois: Vec<ServerPointOfInterest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_websites: Vec<ServerWebsiteImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_databases: Vec<ServerDatabaseImportExport>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_notes: Vec<ServerNote>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub server_extra_users: Vec<ServerExtraUserAccount>,
}

impl ServerGroupImportExport {
    fn dependencies_project_names(&self) -> HashSet<String> {
        let mut deps = HashSet::new();
        for www in &self.server_websites {
            if let Some(db) = www.server_database.as_ref() {
                deps.insert(db.project_name.clone());
            }
        }
        deps
    }
}
