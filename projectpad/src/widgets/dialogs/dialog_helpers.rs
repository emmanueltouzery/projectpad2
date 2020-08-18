use diesel::prelude::*;
use gtk::prelude::*;

pub fn get_project_group_names(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
    use projectpadsql::schema::server::dsl as srv;
    let mut server_group_names: Vec<Option<String>> = srv::server
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
    server_group_names.append(&mut prj_poi_group_names);
    let mut server_group_names_no_options: Vec<_> =
        server_group_names.into_iter().map(|n| n.unwrap()).collect();
    server_group_names_no_options.sort();
    server_group_names_no_options.dedup();
    server_group_names_no_options
}

pub fn get_server_group_names(sql_conn: &diesel::SqliteConnection, project_id: i32) -> Vec<String> {
    // TODO
    vec![]
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
