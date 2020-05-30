use diesel::prelude::*;

#[derive(Queryable)]
pub struct Table {
    pub id: i32,
    pub name: String,
    pub has_dev: String,
    pub has_uat: String,
    pub has_stage: String,
    pub has_prod: String,
}
