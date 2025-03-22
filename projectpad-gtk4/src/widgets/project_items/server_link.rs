use adw::prelude::*;
use projectpadsql::{get_server_group_names, models::ServerLink};

use crate::{
    search_engine::SearchItemsType,
    widgets::{
        project_item::WidgetMode, project_item_model::ProjectItemType,
        search::search_picker::SearchPicker,
    },
};

use super::{
    common,
    item_header_edit::ItemHeaderEdit,
    project_poi::{project_item_header, DisplayHeaderMode},
};

pub const NO_GROUP: &'static str = "No group";

pub fn server_link_contents(
    server_link: &ServerLink,
    project_group_names: &[String],
    widget_mode: WidgetMode,
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
        widget_mode,
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
