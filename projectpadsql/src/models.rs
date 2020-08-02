use diesel::backend::Backend;
use diesel::serialize::Output;
use diesel::types::*;
use std::io::Write;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{Display, EnumString};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsExpression, FromSqlRow, Display)]
pub enum ServerType {
    SrvDatabase,
    SrvApplication,
    SrvHttpOrProxy,
    SrvMonitoring,
    SrvReporting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsExpression, FromSqlRow, Display)]
pub enum ServerAccessType {
    SrvAccessSsh,
    SrvAccessRdp,
    SrvAccessWww,
    SrvAccessSshTunnel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsExpression, FromSqlRow, Display)]
#[sql_type = "Varchar"]
pub enum EnvironmentType {
    EnvDevelopment,
    EnvUat,
    EnvStage,
    EnvProd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsExpression, FromSqlRow, Display)]
pub enum InterestType {
    PoiApplication,
    PoiLogFile,
    PoiConfigFile,
    PoiCommandToRun,
    PoiCommandTerminal,
    PoiBackupArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, AsExpression, FromSqlRow, Display)]
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub id: i32,
    pub desc: String,
    pub ip: String,
    pub text: String,
    pub is_retired: bool,
    pub username: String,
    pub password: String,
    pub auth_key: Option<Vec<u8>>,
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerLink {
    pub id: i32,
    pub desc: String,
    pub linked_server_id: i32,
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerPointOfInterest {
    pub id: i32,
    pub desc: String,
    pub path: String,
    pub text: String,
    pub interest_type: InterestType,
    pub run_on: RunOn,
    pub group_name: Option<String>,
    pub server_id: i32,
}

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerNote {
    pub id: i32,
    pub title: String,
    pub contents: String,
    pub group_name: Option<String>,
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

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct ServerDatabase {
    pub id: i32,
    pub desc: String,
    pub name: String,
    pub text: String,
    pub username: String,
    pub password: String,
    pub group_name: Option<String>,
    pub server_id: i32,
}
