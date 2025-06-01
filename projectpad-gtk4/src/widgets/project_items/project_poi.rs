use adw::prelude::*;
use diesel::prelude::*;
use std::str::FromStr;
use std::sync::mpsc;

use projectpadsql::{
    get_project_group_names,
    models::{EnvironmentType, InterestType, Project, ProjectPointOfInterest},
};

use crate::{
    perform_insert_or_update,
    sql_thread::SqlFunc,
    sql_util,
    widgets::{
        project_item::WidgetMode,
        project_item_list::ProjectItemList,
        project_item_model::ProjectItemType,
        project_items::common::{
            confirm_delete, display_item_edit_dialog, run_sqlfunc_and_then, DialogClamp,
        },
    },
};
use gtk::subclass::prelude::*;

use super::{
    common::{self},
    item_header_edit::ItemHeaderEdit,
    item_header_view::ItemHeaderView,
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
            use projectpadsql::schema::project::dsl as prj;
            use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
            let poi = prj_poi::project_point_of_interest
                .filter(prj_poi::id.eq(project_poi_id))
                .first::<ProjectPointOfInterest>(sql_conn)
                .unwrap();

            let project_group_names = get_project_group_names(sql_conn, poi.project_id);

            let project = prj::project
                .filter(prj::id.eq(poi.project_id))
                .first::<Project>(sql_conn)
                .unwrap();

            sender
                .send_blocking((project, poi, project_group_names))
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    glib::spawn_future_local(async move {
        let (project, poi, project_group_names) = receiver.recv().await.unwrap();
        display_project_poi(&p, poi, &project_group_names, &project.allowed_envs());
    });
}

fn display_project_poi(
    parent: &adw::Bin,
    poi: ProjectPointOfInterest,
    project_group_names: &[String],
    allowed_envs: &[EnvironmentType],
) {
    let (maybe_header_edit, project_poi_view_edit, header_box, vbox) =
        project_poi_contents(&poi, project_group_names, WidgetMode::Show, allowed_envs);
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
    let poi_name = &poi.desc;
    let poi_id = poi.id;
    let project_id = poi.project_id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(
            #[strong(rename_to = poi_n)]
            poi_name,
            #[strong(rename_to = p_id)]
            poi_id,
            #[strong(rename_to = pid)]
            project_id,
            move |_b: gtk::Button| {
            confirm_delete("Delete POI", &format!("Do you want to delete '{}'? This action cannot be undone.", poi_n),
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
    let ae = allowed_envs.to_owned();
    edit_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(
            #[strong(rename_to = p)]
            poi,
            #[strong(rename_to = pgn_)]
            pgn,
            #[strong(rename_to = v)]
            vbox,
            #[strong(rename_to = ae_)]
            ae,
            move |_b: gtk::Button| {
                let (maybe_header_edit, project_poi_view_edit, _, vbox) =
                    project_poi_contents(&p, &pgn_, WidgetMode::Edit, &ae_);

                let (dlg, save_btn) = display_item_edit_dialog(
                    &v,
                    "Edit project POI",
                    vbox,
                    600,
                    600,
                    DialogClamp::Yes,
                );
                let he = maybe_header_edit.unwrap().clone();
                save_btn.connect_clicked(move |_| {
                    let receiver = save_project_poi(
                        Some(poi.id),
                        he.property("group_name"),
                        he.property("title"),
                        project_poi_view_edit.property("path"),
                        project_poi_view_edit.property("text"),
                        InterestType::from_str(
                            &project_poi_view_edit.property::<String>("interest_type"),
                        )
                        .unwrap(),
                    );

                    let db_sender = common::app().get_sql_channel();
                    let dlg = dlg.clone();
                    glib::spawn_future_local(async move {
                        let project_poi_after_result = receiver.recv().await.unwrap();
                        let app = common::app();
                        let window = app.imp().window.get().unwrap();
                        let win_binding = window.upgrade();
                        let win_binding_ref = win_binding.as_ref().unwrap();
                        let pi_bin = &win_binding_ref.imp().project_item.imp().project_item;
                        // load_and_display_project_poi(pi_bin, db_sender, poi.id);
                        ProjectItemList::display_project_item(
                            None,
                            poi.id,
                            ProjectItemType::ProjectPointOfInterest,
                        );
                        dlg.close();
                    });
                });
            }
        ),
    );

    parent.set_child(Some(&vbox));
}

pub fn project_poi_contents(
    poi: &ProjectPointOfInterest,
    project_group_names: &[String],
    widget_mode: WidgetMode,
    allowed_envs: &[EnvironmentType],
) -> (
    Option<ItemHeaderEdit>,
    ProjectPoiViewEdit,
    gtk::Box,
    gtk::Box,
) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let (maybe_header_edit, header_box) = project_item_header(
        &vbox,
        &poi.desc,
        poi.group_name.as_deref(),
        ProjectItemType::ProjectPointOfInterest,
        common::EnvOrEnvs::None,
        project_group_names,
        widget_mode,
        DisplayHeaderMode::Yes,
        None,
        allowed_envs,
    );

    let project_poi_view_edit = ProjectPoiViewEdit::new();
    project_poi_view_edit.set_interest_type(poi.interest_type.to_string());
    project_poi_view_edit.set_path(poi.path.clone());
    project_poi_view_edit.set_text(poi.text.clone());
    project_poi_view_edit.prepare(widget_mode);
    vbox.append(&project_poi_view_edit);

    (maybe_header_edit, project_poi_view_edit, header_box, vbox)
}

#[derive(PartialEq, Eq)]
pub enum DisplayHeaderMode {
    Yes,
    No,
}

pub fn project_item_header(
    vbox: &gtk::Box,
    desc: &str,
    group_name: Option<&str>,
    project_item_type: ProjectItemType,
    env_info: common::EnvOrEnvs,
    project_group_names: &[String],
    widget_mode: WidgetMode,
    display_header_mode: DisplayHeaderMode,
    item_header_view_css_class: Option<&str>,
    allowed_envs: &[EnvironmentType],
) -> (Option<ItemHeaderEdit>, gtk::Box) {
    if widget_mode == WidgetMode::Edit {
        let project_item_header = ItemHeaderEdit::new(
            project_item_type.get_icon(),
            group_name,
            project_group_names,
            env_info,
            allowed_envs,
        );
        project_item_header.set_title(desc);
        if display_header_mode == DisplayHeaderMode::Yes {
            vbox.append(&project_item_header);
        }
        (
            Some(project_item_header.clone()),
            project_item_header.header_box(),
        )
    } else {
        let project_item_header = ItemHeaderView::new(project_item_type);
        project_item_header.set_title(desc);
        if display_header_mode == DisplayHeaderMode::Yes {
            vbox.append(&project_item_header);
        }
        if let Some(css) = item_header_view_css_class {
            project_item_header.set_css_classes(&[css]);
        }
        (None, project_item_header.header_box())
    }
}

pub fn save_project_poi(
    project_poi_id: Option<i32>,
    new_group_name: String,
    new_desc: String,
    new_path: String,
    new_text: String,
    new_interest_type: InterestType,
) -> async_channel::Receiver<Result<ProjectPointOfInterest, (String, Option<String>)>> {
    let app = common::app();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);
    let project_id = app.project_id().unwrap();

    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
            let changeset = (
                prj_poi::desc.eq(new_desc.as_str()),
                prj_poi::path.eq(new_path.as_str()),
                prj_poi::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                prj_poi::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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
