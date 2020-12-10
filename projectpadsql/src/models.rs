use chrono::naive::NaiveDateTime;
use diesel::backend::Backend;
use diesel::deserialize::*;
use diesel::serialize::Output;
use diesel::serialize::*;
use diesel::sql_types::*;
use serde_derive::{Deserialize, Serialize};
use std::io::Write;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{Display, EnumIter, EnumString};

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
    Display,
    EnumIter,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[sql_type = "Varchar"]
pub enum ServerType {
    SrvDatabase,
    SrvApplication,
    SrvHttpOrProxy,
    SrvMonitoring,
    SrvReporting,
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
#[sql_type = "Varchar"]
pub enum ServerAccessType {
    SrvAccessSsh,
    SrvAccessRdp,
    SrvAccessWww,
    SrvAccessSshTunnel,
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
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[sql_type = "Varchar"]
pub enum EnvironmentType {
    EnvDevelopment,
    EnvUat,
    EnvStage,
    EnvProd,
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
#[sql_type = "Varchar"]
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
#[sql_type = "Varchar"]
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
            fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
                Ok(<$x>::from_str(&String::from_sql(bytes)?)?)
            }
        }

        impl<DB> ToSql<Varchar, DB> for $x
        where
            DB: Backend,
        {
            fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> diesel::serialize::Result {
                out.write_all(self.to_string().as_bytes())?;
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Server {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub is_retired: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub auth_key: Option<Vec<u8>>,
    #[serde(default)]
    pub auth_key_filename: Option<String>,
    pub server_type: ServerType,
    pub access_type: ServerAccessType,
    #[serde(default)]
    pub ssh_tunnel_port: Option<i32>,
    #[serde(default)]
    pub ssh_tunnel_through_server_id: Option<i32>,
    #[serde(default)]
    pub environment: EnvironmentType,
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ProjectPointOfInterest {
    pub id: i32,
    pub desc: String,
    pub path: String,
    pub text: String,
    pub interest_type: InterestType,
    pub group_name: Option<String>,
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLink {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub desc: String,
    pub linked_server_id: i32,
    #[serde(skip)]
    pub environment: EnvironmentType,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerExtraUserAccount {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub desc: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auth_key: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auth_key_filename: Option<String>,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
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
