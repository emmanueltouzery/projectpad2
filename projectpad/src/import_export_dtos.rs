use projectpadsql::models::{
    EnvironmentType, InterestType, Server, ServerDatabase, ServerExtraUserAccount, ServerNote,
    ServerPointOfInterest,
};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct ServerImportExport(pub Server);

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

#[derive(Serialize, Deserialize)]
pub struct ServerWithItemsImportExport {
    pub server: ServerImportExport,
    pub items: ServerGroupImportExport,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub items_in_groups: HashMap<String, ServerGroupImportExport>,
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
