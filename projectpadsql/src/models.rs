use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::types::*;
use std::str::FromStr;
use strum_macros::EnumString;

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

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsExpression, FromSqlRow)]
pub enum ServerType {
    SrvDatabase,
    SrvApplication,
    SrvHttpOrProxy,
    SrvMonitoring,
    SrvReporting,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsExpression, FromSqlRow)]
pub enum ServerAccessType {
    SrvAccessSsh,
    SrvAccessRdp,
    SrvAccessWww,
    SrvAccessSshTunnel,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, AsExpression, FromSqlRow)]
pub enum EnvironmentType {
    EnvDevelopment,
    EnvUat,
    EnvStage,
    EnvProd,
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
    };
}

simple_enum!(EnvironmentType);
simple_enum!(ServerType);
simple_enum!(ServerAccessType);

#[derive(Queryable, Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub id: i32,
    pub desc: String,
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
