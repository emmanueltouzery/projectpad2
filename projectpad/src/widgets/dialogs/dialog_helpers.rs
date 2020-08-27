use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use diesel::query_builder::IntoUpdateTarget;
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::query_dsl::methods::FindDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{associations::HasTable, helper_types::Find, query_builder::DeleteStatement};
use gtk::prelude::*;
use std::sync::mpsc;

pub fn get_project_group_names(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
    use projectpadsql::schema::server::dsl as srv;
    let server_group_names = srv::server
        .filter(
            srv::project_id
                .eq(project_id)
                .and(srv::group_name.is_not_null()),
        )
        .order(srv::group_name.asc())
        .select(srv::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_poi_group_names = ppoi::project_point_of_interest
        .filter(
            ppoi::project_id
                .eq(project_id)
                .and(ppoi::group_name.is_not_null()),
        )
        .order(ppoi::group_name.asc())
        .select(ppoi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut project_group_names = server_group_names;
    project_group_names.append(&mut prj_poi_group_names);
    let mut project_group_names_no_options: Vec<_> = project_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    project_group_names_no_options.sort();
    project_group_names_no_options.dedup();
    project_group_names_no_options
}

pub fn get_server_group_names(sql_conn: &diesel::SqliteConnection, server_id: i32) -> Vec<String> {
    use projectpadsql::schema::server_database::dsl as db;
    use projectpadsql::schema::server_extra_user_account::dsl as usr;
    use projectpadsql::schema::server_note::dsl as not;
    use projectpadsql::schema::server_point_of_interest::dsl as poi;
    use projectpadsql::schema::server_website::dsl as www;
    let server_poi_group_names = poi::server_point_of_interest
        .filter(
            poi::server_id
                .eq(server_id)
                .and(poi::group_name.is_not_null()),
        )
        .order(poi::group_name.asc())
        .select(poi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_www_group_names = www::server_website
        .filter(
            www::server_id
                .eq(server_id)
                .and(www::group_name.is_not_null()),
        )
        .order(www::group_name.asc())
        .select(www::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_db_group_names = db::server_database
        .filter(
            db::server_id
                .eq(server_id)
                .and(db::group_name.is_not_null()),
        )
        .order(db::group_name.asc())
        .select(db::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_usr_group_names = usr::server_extra_user_account
        .filter(
            usr::server_id
                .eq(server_id)
                .and(usr::group_name.is_not_null()),
        )
        .order(usr::group_name.asc())
        .select(usr::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_notes_group_names = not::server_note
        .filter(
            not::server_id
                .eq(server_id)
                .and(not::group_name.is_not_null()),
        )
        .order(not::group_name.asc())
        .select(not::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_group_names = server_poi_group_names;
    server_group_names.append(&mut server_www_group_names);
    server_group_names.append(&mut server_db_group_names);
    server_group_names.append(&mut server_usr_group_names);
    server_group_names.append(&mut server_notes_group_names);
    let mut server_group_names_no_options: Vec<_> = server_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    server_group_names_no_options.sort();
    server_group_names_no_options.dedup();
    server_group_names_no_options
}

pub fn init_group_control(groups_store: &gtk::ListStore, group: &gtk::ComboBoxText) {
    let completion = gtk::EntryCompletion::new();
    completion.set_model(Some(groups_store));
    completion.set_text_column(0);
    group
        .get_child()
        .unwrap()
        .dynamic_cast::<gtk::Entry>()
        .unwrap()
        .set_completion(Some(&completion));
}

pub fn fill_groups(
    groups_store: &gtk::ListStore,
    group_widget: &gtk::ComboBoxText,
    groups: &[String],
    cur_group_name: &Option<String>,
) {
    for group in groups {
        let iter = groups_store.append();
        groups_store.set_value(&iter, 0, &glib::Value::from(&group));
        group_widget.append_text(&group);
    }

    if let Some(t) = cur_group_name.as_deref() {
        group_widget
            .get_child()
            .unwrap()
            .dynamic_cast::<gtk::Entry>()
            .unwrap()
            .set_text(t);
    }
}

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
        use crate::widgets::dialogs::dialog_helpers::insert_row;
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

pub fn style_grid(grid: &gtk::Grid) {
    grid.set_margin_start(30);
    grid.set_margin_end(30);
    grid.set_margin_top(10);
    grid.set_margin_bottom(5);
    grid.set_row_spacing(5);
    grid.set_column_spacing(10);
}

pub fn fetch_server_groups(
    groups_sender: &relm::Sender<Vec<String>>,
    server_id: i32,
    db_sender: &mpsc::Sender<SqlFunc>,
) {
    let s = groups_sender.clone();
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            s.send(get_server_group_names(sql_conn, server_id)).unwrap();
        }))
        .unwrap();
}

pub trait ServerItemDialogModelParam<T> {
    fn get_item(&self) -> Option<&T>;
}

impl<T> ServerItemDialogModelParam<T> for (mpsc::Sender<SqlFunc>, i32, Option<T>) {
    fn get_item(&self) -> Option<&T> {
        self.2.as_ref()
    }
}

/// you must keep a reference to the component in your model,
/// otherwise event processing will die when the component gets dropped
pub fn prepare_add_edit_item_dialog<T, Dlg>(
    widget_for_window: gtk::Widget,
    widget_param: Dlg::ModelParam,
    ok_pressed_event: Dlg::Msg,
    item_desc: &'static str,
) -> (gtk::Dialog, relm::Component<Dlg>, gtk::Button)
where
    Dlg: relm::Widget + 'static,
    Dlg::Msg: Clone,
    Dlg::ModelParam: ServerItemDialogModelParam<T>,
{
    let title = if widget_param.get_item().is_some() {
        "Edit "
    } else {
        "Add "
    }
    .to_string()
        + item_desc;
    let dialog_contents =
        relm::init::<Dlg>(widget_param).expect("error initializing the server item add edit modal");
    let d_c = dialog_contents.clone();
    standard_dialogs::prepare_custom_dialog(
        widget_for_window,
        600,
        200,
        title,
        dialog_contents,
        move |_| {
            d_c.emit(ok_pressed_event.clone());
            standard_dialogs::DialogActionResult::CloseDialog
        },
    )
}
