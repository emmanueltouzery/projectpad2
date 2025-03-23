use diesel::prelude::*;
use std::sync::mpsc;

use adw::prelude::*;
use projectpadsql::{
    get_project_group_names, get_server_group_names,
    models::{EnvironmentType, ServerLink},
};

use crate::{
    app::ProjectpadApplication,
    perform_insert_or_update,
    search_engine::SearchItemsType,
    sql_thread::SqlFunc,
    sql_util,
    widgets::{
        project_item::{ProjectItem, WidgetMode},
        project_item_list::ProjectItemList,
        project_item_model::ProjectItemType,
        project_items::common::{confirm_delete, run_sqlfunc_and_then},
        search::search_picker::SearchPicker,
    },
};

use super::{
    common,
    item_header_edit::ItemHeaderEdit,
    project_poi::{project_item_header, DisplayHeaderMode},
    server,
};

pub const NO_GROUP: &'static str = "No group";

pub fn load_and_display_server_link(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    server_link_id: i32,
    project_item: &ProjectItem,
) {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_link::dsl as srv_link;
            let server_link = srv_link::server_link
                .filter(srv_link::id.eq(server_link_id))
                .first::<ServerLink>(sql_conn)
                .unwrap();

            let project_group_names = get_project_group_names(sql_conn, server_link.project_id);

            sender
                .send_blocking((server_link, project_group_names))
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    let pi = project_item.clone();
    glib::spawn_future_local(async move {
        let (server_link, project_group_names) = receiver.recv().await.unwrap();
        // TODO surely i can chain sql requests in a more elegant manner...
        load_and_display_server_link_server(db_sender, &p, server_link, &project_group_names, &pi);
    });
}

fn load_and_display_server_link_server(
    db_sender: mpsc::Sender<SqlFunc>,
    parent: &adw::Bin,
    server_link: ServerLink,
    project_group_names: &[String],
    project_item: &ProjectItem,
) {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            sender
                .send_blocking(server::run_channel_data_query(
                    sql_conn,
                    server_link.linked_server_id,
                ))
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    let pi = project_item.clone();
    let pgn = project_group_names.to_vec();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_server_link(&p, server_link, &channel_data, &pgn, &pi);
    });
}

fn display_server_link(
    parent: &adw::Bin,
    server_link: ServerLink,
    channel_data: &server::ChannelData,
    project_group_names: &[String],
    project_item: &ProjectItem,
) {
    let (header_box, vbox) = server_link_contents_show(
        &server_link,
        channel_data,
        project_group_names,
        &project_item,
    );
    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    header_box.append(&edit_btn);

    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    header_box.append(&delete_btn);
    let poi_name = &server_link.desc;
    let poi_id = server_link.id;
    let project_id = server_link.project_id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong poi_name as server_l, @strong poi_id as p_id, @strong project_id as pid => move |_b: gtk::Button| {
            confirm_delete("Delete Server Link", &format!("Do you want to delete '{}'? This action cannot be undone.", server_l),
            Box::new(move || {
                run_sqlfunc_and_then(
                    Box::new(move |sql_conn| {
                        use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
                        sql_util::delete_row(sql_conn, prj_poi::project_point_of_interest, p_id)
                            .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project(pid);
                        }),
                );
            }))
        })
    );

    let pgn = project_group_names.to_vec();
    // edit_btn.connect_closure(
    //         "clicked",
    //         false,
    //         glib::closure_local!(@strong server_link as p, @strong pgn as pgn_, @strong vbox as v => move |_b: gtk::Button| {
    //             let (maybe_header_edit, project_poi_view_edit, _, vbox) = project_poi_contents(&p, &pgn_, WidgetMode::Edit);

    //             let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit project POI", vbox, 600, 600, DialogClamp::Yes);
    //             let he = maybe_header_edit.unwrap().clone();
    //             save_btn.connect_clicked(move|_| {
    //                 let receiver = save_project_poi(
    //                     Some(server_link.id),
    //                     he.property("group_name"),
    //                     he.property("title"),
    //                     project_poi_view_edit.property("path"),
    //                     project_poi_view_edit.property("text"),
    //                     InterestType::from_str(&project_poi_view_edit.property::<String>("interest_type")).unwrap(),
    //                 );

    //                 let app = gio::Application::default()
    //                     .and_downcast::<ProjectpadApplication>()
    //                     .unwrap();
    //                 let db_sender = app.get_sql_channel();
    //                 let dlg = dlg.clone();
    //                 glib::spawn_future_local(async move {
    //                     let project_poi_after_result = receiver.recv().await.unwrap();
    //                     let window = app.imp().window.get().unwrap();
    //                     let win_binding = window.upgrade();
    //                     let win_binding_ref = win_binding.as_ref().unwrap();
    //                     let pi_bin = &win_binding_ref.imp().project_item.imp().project_item;
    //                     // load_and_display_project_poi(pi_bin, db_sender, poi.id);
    //                     ProjectItemList::display_project_item(server_link.id, ProjectItemType::ProjectPointOfInterest);
    //                     dlg.close();
    //                 });
    //             });
    //         }),
    //     );

    parent.set_child(Some(&vbox));
}

pub fn server_link_contents_edit(
    server_link: &ServerLink,
    project_group_names: &[String],
) -> (
    Option<ItemHeaderEdit>,
    SearchPicker,
    gtk::DropDown,
    gtk::Box,
    gtk::Box,
) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let (maybe_header_edit, header_box) = project_item_header(
        &vbox,
        &server_link.desc,
        server_link.group_name.as_deref(),
        ProjectItemType::ServerLink,
        common::EnvOrEnvs::Env(server_link.environment),
        project_group_names,
        WidgetMode::Edit,
        DisplayHeaderMode::Yes,
    );
    let search_picker = glib::Object::builder::<SearchPicker>()
        .property(
            "search-item-types",
            SearchItemsType::ServersOnly.to_string(),
        )
        .build();
    vbox.append(&search_picker);

    let linked_group_name_hbox = gtk::Box::builder()
        .margin_start(10)
        .margin_end(10)
        .margin_top(10)
        .margin_bottom(10)
        .spacing(10)
        .build();
    linked_group_name_hbox.append(&gtk::Label::builder().label("Server group name").build());

    let server_group_name_dropdown = gtk::DropDown::builder().build();
    linked_group_name_hbox.append(&server_group_name_dropdown);

    let gn_dropdown = server_group_name_dropdown.clone();
    search_picker.connect_selected_item_item_id_notify(move |picker| {
        let server_id = picker.selected_item_item_id();
        let group_names_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
            get_server_group_names(sql_conn, server_id)
        }));
        let dd = gn_dropdown.clone();
        glib::spawn_future_local(async move {
            let all_group_names = group_names_recv.recv().await.unwrap();
            let mut group_names = vec![NO_GROUP];
            group_names.extend(all_group_names.iter().map(String::as_str));
            let dropdown_entries_store = gtk::StringList::new(&group_names);
            dd.set_model(Some(&dropdown_entries_store));
        });
    });

    vbox.append(&linked_group_name_hbox);

    (
        maybe_header_edit,
        search_picker,
        server_group_name_dropdown,
        header_box,
        vbox,
    )
}

pub fn server_link_contents_show(
    server_link: &ServerLink,
    channel_data: &server::ChannelData,
    project_group_names: &[String],
    project_item: &ProjectItem,
) -> (gtk::Box, gtk::Box) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let (maybe_header_edit, header_box) = project_item_header(
        &vbox,
        &server_link.desc,
        server_link.group_name.as_deref(),
        ProjectItemType::ServerLink,
        common::EnvOrEnvs::Env(server_link.environment),
        project_group_names,
        WidgetMode::Show,
        DisplayHeaderMode::Yes,
    );
    // in show mode display the server
    let server_view_edit =
        server::server_view_edit_contents(&channel_data.server, WidgetMode::Show);
    vbox.append(&server_view_edit);

    server::add_server_items(&channel_data, None, &vbox, project_item);

    (header_box, vbox)
}

pub fn save_server_link(
    server_link_id: Option<i32>,
    new_group_name: String,
    new_desc: String,
    new_server_id: i32,
    new_server_group_name: Option<String>,
    new_env_type: EnvironmentType,
) -> async_channel::Receiver<Result<ServerLink, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);
    let project_id = app.project_id().unwrap();

    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_link::dsl as srv_link;
            let changeset = (
                srv_link::desc.eq(new_desc.as_str()),
                srv_link::linked_group_name.eq(new_server_group_name.as_deref()),
                srv_link::linked_server_id.eq(new_server_id),
                // never store Some("") for group, we want None then.
                srv_link::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
                srv_link::project_id.eq(project_id),
                srv_link::environment.eq(new_env_type),
            );
            let project_poi_after_result = perform_insert_or_update!(
                sql_conn,
                server_link_id,
                srv_link::server_link,
                srv_link::id,
                changeset,
                ServerLink,
            );
            sender.send_blocking(project_poi_after_result).unwrap();
        }))
        .unwrap();
    receiver
}
