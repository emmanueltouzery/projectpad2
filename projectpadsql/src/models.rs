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

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ip: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "is_false")]
    pub is_retired: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_key: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_key_filename: Option<String>,
    pub server_type: ServerType,
    pub access_type: ServerAccessType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_tunnel_port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_tunnel_through_server_id: Option<i32>,
    #[serde(skip)]
    pub environment: EnvironmentType,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectNote {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub contents: String,
    #[serde(skip)]
    pub has_dev: bool,
    #[serde(skip)]
    pub has_uat: bool,
    #[serde(skip)]
    pub has_stage: bool,
    #[serde(skip)]
    pub has_prod: bool,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPointOfInterest {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    pub interest_type: InterestType,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLink {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip)]
    pub linked_server_id: i32,
    #[serde(skip)]
    pub environment: EnvironmentType,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub project_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerWebsite {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    pub server_database_id: Option<i32>,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPointOfInterest {
    #[serde(skip)]
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_key: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_key_filename: Option<String>,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerDatabase {
    pub id: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(skip)]
    pub group_name: Option<String>,
    #[serde(skip)]
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct DbVersion {
    pub id: i32,
    pub code: i32,
    pub update_date: NaiveDateTime,
}

// https://stackoverflow.com/a/53900684/516188
/// This is only used for serialize
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(num: &bool) -> bool {
    !num
}
