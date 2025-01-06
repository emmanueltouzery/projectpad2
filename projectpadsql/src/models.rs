use chrono::naive::NaiveDateTime;
use diesel::backend::Backend;
use diesel::deserialize::*;
use diesel::serialize::Output;
use diesel::serialize::*;
use diesel::sql_types::*;
use diesel::sqlite::Sqlite;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{Display, EnumIter, EnumString, FromRepr};

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub icon: Option<Vec<u8>>,
    pub has_dev: bool,
    pub has_uat: bool,
    pub has_stage: bool,
    pub has_prod: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    AsExpression,
    FromSqlRow,
    FromRepr,
    Display,
    EnumIter,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Varchar)]
#[repr(u8)]
pub enum ServerType {
    SrvDatabase = 1,
    SrvApplication = 0,
    SrvHttpOrProxy = 2,
    SrvMonitoring = 3,
    SrvReporting = 4,
}

impl Default for ServerType {
    fn default() -> Self {
        ServerType::SrvApplication
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    AsExpression,
    FromSqlRow,
    FromRepr,
    Display,
    EnumIter,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Varchar)]
#[repr(u8)]
pub enum ServerAccessType {
    SrvAccessSsh = 1,
    SrvAccessRdp = 0,
    SrvAccessWww = 3,
    SrvAccessSshTunnel = 2,
}

impl Default for ServerAccessType {
    fn default() -> Self {
        ServerAccessType::SrvAccessSsh
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    AsExpression,
    FromSqlRow,
    FromRepr,
    Display,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Varchar)]
#[derive(Hash)]
#[repr(u8)]
pub enum EnvironmentType {
    EnvDevelopment = 1,
    EnvStage = 2,
    EnvUat = 4,
    EnvProd = 8,
}

impl Default for EnvironmentType {
    fn default() -> Self {
        EnvironmentType::EnvDevelopment
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    AsExpression,
    FromSqlRow,
    Display,
    EnumIter,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Varchar)]
pub enum InterestType {
    PoiApplication,
    PoiLogFile,
    PoiConfigFile,
    PoiCommandToRun,
    PoiCommandTerminal,
    PoiBackupArchive,
}

impl Default for InterestType {
    fn default() -> Self {
        InterestType::PoiApplication
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    EnumIter,
    AsExpression,
    FromSqlRow,
    Display,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Varchar)]
pub enum RunOn {
    RunOnServer,
    RunOnClient,
}

macro_rules! simple_enum {
    ($x:ty) => {
        impl<DB> FromSql<Varchar, DB> for $x
        where
            DB: Backend,
            String: FromSql<Varchar, DB>,
        {
            fn from_sql(bytes: diesel::backend::RawValue<DB>) -> diesel::deserialize::Result<Self> {
                Ok(<$x>::from_str(&String::from_sql(bytes)?)?)
            }
        }

        // https://diesel.rs/guides/migration_guide.html#2-0-0-to-sql
        impl ToSql<Varchar, Sqlite> for $x
        where
            String: ToSql<Varchar, Sqlite>,
        {
            fn to_sql(&self, out: &mut Output<Sqlite>) -> diesel::serialize::Result {
                out.set_value(self.to_string());
                // self.to_string().to_sql(out);
                Ok(IsNull::No)
            }
        }
    };
}

simple_enum!(EnvironmentType);
simple_enum!(ServerType);
simple_enum!(ServerAccessType);
simple_enum!(InterestType);
simple_enum!(RunOn);

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Default)]
pub struct Server {
    pub id: i32,
    pub desc: String,
    pub ip: String,
    pub text: String,
    pub is_retired: bool,
    pub username: String,
    pub password: String,
    pub auth_key: Option<Vec<u8>>, //
    pub auth_key_filename: Option<String>,
    pub server_type: ServerType,
    pub access_type: ServerAccessType,
    pub ssh_tunnel_port: Option<i32>,
    pub ssh_tunnel_through_server_id: Option<i32>,
    pub environment: EnvironmentType,
    pub group_name: Option<String>,
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ProjectNote {
    pub id: i32,
    pub title: String,
    pub contents: String,
    pub has_dev: bool,
    pub has_uat: bool,
    pub has_stage: bool,
    pub has_prod: bool,
    pub group_name: Option<String>,
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectPointOfInterest {
    pub id: i32,
    pub desc: String,
    pub path: String,
    pub text: String,
    pub interest_type: InterestType,
    pub group_name: Option<String>,
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerLink {
    pub id: i32,
    pub desc: String,
    pub linked_server_id: i32,
    pub linked_group_name: Option<String>,
    pub environment: EnvironmentType,
    pub group_name: Option<String>,
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerWebsite {
    pub id: i32,
    pub desc: String,
    pub url: String,
    pub text: String,
    pub username: String,
    pub password: String,
    pub server_database_id: Option<i32>,
    pub group_name: Option<String>,
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPointOfInterest {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub text: String,
    pub interest_type: InterestType,
    pub run_on: RunOn,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerNote {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub contents: String,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerExtraUserAccount {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub desc: String,
    pub auth_key: Option<Vec<u8>>,
    pub auth_key_filename: Option<String>,
    pub group_name: Option<String>,
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerDatabase {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct DbVersion {
    pub id: i32,
    pub code: i32,
    pub update_date: NaiveDateTime,
}
