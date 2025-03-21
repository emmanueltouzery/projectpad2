use adw::prelude::*;
use projectpadsql::models::ServerLink;

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

pub fn server_link_contents(
    server_link: &ServerLink,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (Option<ItemHeaderEdit>, SearchPicker, gtk::Box, gtk::Box) {
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

    (maybe_header_edit, search_picker, header_box, vbox)
}
