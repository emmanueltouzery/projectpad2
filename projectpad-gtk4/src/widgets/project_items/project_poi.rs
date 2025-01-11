use adw::prelude::*;
use diesel::prelude::*;
use std::sync::mpsc;

use projectpadsql::models::{InterestType, ProjectPointOfInterest};

use crate::{
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
        display_project_oi(&p, poi, &project_group_names);
    });
}

fn display_project_oi(
    parent: &adw::Bin,
    poi: ProjectPointOfInterest,
    project_group_names: &[String],
) {
    let (header_box, vbox) = project_poi_contents(&poi, project_group_names, WidgetMode::Show);
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
                let (_, vbox) = project_poi_contents(&p, &pgn_, WidgetMode::Edit);

                display_item_edit_dialog(&v, "Edit project POI", vbox, 600, 600, DialogClamp::Yes);
            }),
        );

    parent.set_child(Some(&vbox));
}

pub fn project_poi_contents(
    poi: &ProjectPointOfInterest,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (gtk::Box, gtk::Box) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let header_box = if widget_mode == WidgetMode::Edit {
        let project_item_header = ProjectItemHeaderEdit::new(
            ProjectItemType::ProjectPointOfInterest,
            poi.group_name.as_deref(),
            project_group_names,
            common::EnvOrEnvs::None,
        );
        project_item_header.set_title(poi.desc.clone());
        vbox.append(&project_item_header);
        project_item_header.header_box()
    } else {
        let project_item_header =
            ProjectItemHeaderView::new(ProjectItemType::ProjectPointOfInterest);
        project_item_header.set_title(poi.desc.clone());
        vbox.append(&project_item_header);
        project_item_header.header_box()
    };

    let project_poi_view_edit = ProjectPoiViewEdit::new();
    project_poi_view_edit.set_interest_type(poi.interest_type.to_string());
    project_poi_view_edit.set_path(poi.path.clone());
    project_poi_view_edit.set_text(poi.text.clone());
    project_poi_view_edit.prepare(widget_mode);
    vbox.append(&project_poi_view_edit);

    (header_box, vbox)
}
