use adw::prelude::*;
use diesel::prelude::*;
use std::sync::mpsc;

use projectpadsql::models::{InterestType, ProjectPointOfInterest};

use crate::{
    app::ProjectpadApplication,
    perform_insert_or_update,
    sql_thread::SqlFunc,
    widgets::{
        project_item::WidgetMode,
        project_item_model::ProjectItemType,
        project_items::common::{display_item_edit_dialog, get_project_group_names, DialogClamp},
    },
};

use super::{
    common::{self},
    project_item_header_edit::ProjectItemHeaderEdit,
    project_item_header_view::ProjectItemHeaderView,
    project_poi_view_edit::ProjectPoiViewEdit,
};

pub fn load_and_display_project_poi(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    project_poi_id: i32,
) {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
            let poi = prj_poi::project_point_of_interest
                .filter(prj_poi::id.eq(project_poi_id))
                .first::<ProjectPointOfInterest>(sql_conn)
                .unwrap();

            let project_group_names = get_project_group_names(sql_conn, poi.project_id);

            sender.send_blocking((poi, project_group_names)).unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    glib::spawn_future_local(async move {
        let (poi, project_group_names) = receiver.recv().await.unwrap();
        display_project_poi(&p, poi, &project_group_names);
    });
}

fn display_project_poi(
    parent: &adw::Bin,
    poi: ProjectPointOfInterest,
    project_group_names: &[String],
) {
    let (maybe_header_edit, project_poi_view_edit, header_box, vbox) =
        project_poi_contents(&poi, project_group_names, WidgetMode::Show);
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

    let pgn = project_group_names.to_vec();
    edit_btn.connect_closure(
            "clicked",
            false,
            glib::closure_local!(@strong poi as p, @strong pgn as pgn_, @strong vbox as v => move |_b: gtk::Button| {
                let (maybe_header_edit, project_poi_view_edit, _, vbox) = project_poi_contents(&p, &pgn_, WidgetMode::Edit);

                display_item_edit_dialog(&v, "Edit project POI", vbox, 600, 600, DialogClamp::Yes);
            }),
        );

    parent.set_child(Some(&vbox));
}

pub fn project_poi_contents(
    poi: &ProjectPointOfInterest,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (
    Option<ProjectItemHeaderEdit>,
    ProjectPoiViewEdit,
    gtk::Box,
    gtk::Box,
) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let (maybe_header_edit, header_box) = if widget_mode == WidgetMode::Edit {
        let project_item_header = ProjectItemHeaderEdit::new(
            ProjectItemType::ProjectPointOfInterest,
            poi.group_name.as_deref(),
            project_group_names,
            common::EnvOrEnvs::None,
        );
        project_item_header.set_title(poi.desc.clone());
        vbox.append(&project_item_header);
        (
            Some(project_item_header.clone()),
            project_item_header.header_box(),
        )
    } else {
        let project_item_header =
            ProjectItemHeaderView::new(ProjectItemType::ProjectPointOfInterest);
        project_item_header.set_title(poi.desc.clone());
        vbox.append(&project_item_header);
        (None, project_item_header.header_box())
    };

    let project_poi_view_edit = ProjectPoiViewEdit::new();
    project_poi_view_edit.set_interest_type(poi.interest_type.to_string());
    project_poi_view_edit.set_path(poi.path.clone());
    project_poi_view_edit.set_text(poi.text.clone());
    project_poi_view_edit.prepare(widget_mode);
    vbox.append(&project_poi_view_edit);

    (maybe_header_edit, project_poi_view_edit, header_box, vbox)
}

pub fn save_project_poi(
    project_poi_id: Option<i32>,
    new_desc: String,
    new_path: String,
    new_text: String,
    new_interest_type: InterestType,
) -> async_channel::Receiver<Result<ProjectPointOfInterest, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);
    let project_id = app.project_id().unwrap();

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
            let changeset = (
                prj_poi::desc.eq(new_desc.as_str()),
                prj_poi::path.eq(new_path.as_str()),
                prj_poi::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                // prj_poi::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                prj_poi::interest_type.eq(new_interest_type),
                prj_poi::project_id.eq(project_id),
            );
            let project_poi_after_result = perform_insert_or_update!(
                sql_conn,
                project_poi_id,
                prj_poi::project_point_of_interest,
                prj_poi::id,
                changeset,
                ProjectPointOfInterest,
            );
            sender.send_blocking(project_poi_after_result).unwrap();
        }))
        .unwrap();
    receiver
}
