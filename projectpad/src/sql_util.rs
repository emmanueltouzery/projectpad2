use diesel::prelude::*;
use diesel::query_builder::IntoUpdateTarget;
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::query_dsl::methods::FindDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{associations::HasTable, helper_types::Find, query_builder::DeleteStatement};

no_arg_sql_function!(
    last_insert_rowid,
    diesel::sql_types::Integer,
    "Represents the SQL last_insert_row() function"
);

/// insert a row and get back the id of the newly inserted row
/// unfortunately sqlite doesn't support sql RETURNING
pub fn insert_row(
    sql_conn: &SqliteConnection,
    insert_statement: impl ExecuteDsl<SqliteConnection>,
) -> Result<i32, (String, Option<String>)> {
    let insert_result = ExecuteDsl::execute(insert_statement, sql_conn)
        .map_err(|e| ("Error inserting entity".to_string(), Some(e.to_string())));
    match insert_result {
        Ok(1) => {
            // https://github.com/diesel-rs/diesel/issues/771
            // http://www.sqlite.org/c3ref/last_insert_rowid.html
            // caveats of last_insert_rowid seem to be in case of multiple
            // threads sharing a connection (which we don't do), and triggers
            // and other things (which we don't have).
            diesel::select(last_insert_rowid)
                .get_result::<i32>(sql_conn)
                .map_err(|e| {
                    (
                        "Error getting inserted entity id".to_string(),
                        Some(e.to_string()),
                    )
                })
        }
        Ok(x) => Err((
            format!("Expected 1 row modified after insert, got {}!?", x),
            None,
        )),
        Err(e) => Err(e),
    }
}

// https://stackoverflow.com/a/55213728/516188
pub type DeleteFindStatement<F> =
    DeleteStatement<<F as HasTable>::Table, <F as IntoUpdateTarget>::WhereClause>;

pub fn delete_row<Tbl, Pk>(
    sql_conn: &SqliteConnection,
    table: Tbl,
    pk: Pk,
) -> Result<(), (&'static str, Option<String>)>
where
    Tbl: FindDsl<Pk>,
    Find<Tbl, Pk>: IntoUpdateTarget,
    DeleteFindStatement<Find<Tbl, Pk>>: ExecuteDsl<SqliteConnection>,
{
    let find = table.find(pk);
    let delete = diesel::delete(find);
    match delete.execute(sql_conn) {
        Ok(1) => Ok(()),
        Ok(x) => Err((
            "Entity deletion failed",
            Some(format!(
                "Expected 1 row to be modified, but {} rows were modified",
                x
            )),
        )),
        Err(e) => Err(("Entity deletion failed", Some(e.to_string()))),
    }
}

// I tried to implement this with generics with diesel... gave up.
// way simpler with macros.
// i'm not the only one: https://users.rust-lang.org/t/creating-a-generic-insert-method-for-diesel/24124/2
macro_rules! perform_insert_or_update {
    ($sql_conn:expr, $row_id:expr, $table:expr, $key:expr, $changeset: expr, $type: tt,) => {{
        use crate::sql_util::insert_row;
        let row_id_result = match $row_id {
            Some(id) => {
                // update
                diesel::update($table.filter($key.eq(id)))
                    .set($changeset)
                    .execute($sql_conn)
                    .map_err(|e| ("Error updating entity".to_string(), Some(e.to_string())))
                    .map(|_| id)
            }
            None => {
                // insert
                insert_row($sql_conn, diesel::insert_into($table).values($changeset))
            }
        };
        // re-read back the server
        row_id_result.and_then(|row_id| {
            $table
                .filter($key.eq(row_id))
                .first::<$type>($sql_conn)
                .map_err(|e| ("Error reading back entity".to_string(), Some(e.to_string())))
        })
    }};
}
