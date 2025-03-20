use adw::prelude::*;
use projectpadsql::models::ServerLink;

use crate::{
    app::ProjectpadApplication,
    search_engine,
    sql_thread::SqlFunc,
    widgets::{
        project_item::WidgetMode, project_item_model::ProjectItemType,
        search::search_item_list::SearchItemList,
    },
};

use super::{
    common,
    item_header_edit::ItemHeaderEdit,
    project_poi::{project_item_header, DisplayHeaderMode},
};

pub fn server_link_contents(
    server_link: &ServerLink,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (Option<ItemHeaderEdit>, SearchItemList, gtk::Box, gtk::Box) {
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
        widget_mode,
        DisplayHeaderMode::Yes,
    );

    let search_item_list = glib::Object::builder::<SearchItemList>().build();
    vbox.append(
        &gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&adw::Clamp::builder().child(&search_item_list).build())
            .build(),
    );

    populate_search_list(&search_item_list);

    (maybe_header_edit, search_item_list, header_box, vbox)
}

fn populate_search_list(search_item_list: &SearchItemList) {
    let search_spec = search_engine::search_parse("test"); // TODO
    let f = search_spec.search_pattern;

    let search_results_receiver = common::run_sqlfunc(Box::new(move |sql_conn| {
        search_engine::run_search_filter(
            sql_conn,
            search_engine::SearchItemsType::All,
            &f,
            &None,
            false,
        )
    }));
    let mut sil = search_item_list.clone();
    glib::spawn_future_local(async move {
        let search_res = search_results_receiver.recv().await.unwrap();
        sil.set_search_items(search_res);
    });
}
