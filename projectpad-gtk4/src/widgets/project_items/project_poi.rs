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

use super::common::{self, DetailsRow, SuffixAction};

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

fn poi_get_text_label(interest_type: InterestType) -> &'static str {
    match interest_type {
        InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
        _ => "Text",
    }
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

fn project_poi_contents(
    poi: &ProjectPointOfInterest,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (gtk::Box, gtk::Box) {
    let (header_box, vbox) = common::get_contents_box_with_header(
        &poi.desc,
        ProjectItemType::ProjectPointOfInterest,
        poi.group_name.as_deref(),
        project_group_names,
        common::EnvOrEnvs::None,
        widget_mode,
    );

    let (desc, idx) = match poi.interest_type {
        InterestType::PoiApplication => ("Application", 0),
        InterestType::PoiBackupArchive => ("Backup/Archive", 1),
        InterestType::PoiCommandToRun => ("Command to run", 2),
        InterestType::PoiCommandTerminal => ("Command to run", 3),
        InterestType::PoiConfigFile => ("Config file", 4),
        InterestType::PoiLogFile => ("Log file", 5),
    };

    let prefs_group = adw::PreferencesGroup::builder().title(desc).build();

    DetailsRow::new("Path", &poi.path, SuffixAction::copy(&poi.path), &[])
        .add(widget_mode, &prefs_group);

    DetailsRow::new(
        poi_get_text_label(poi.interest_type),
        &poi.text,
        SuffixAction::copy(&poi.text),
        &[],
    )
    .add(widget_mode, &prefs_group);

    if widget_mode == WidgetMode::Edit {
        let interest_type_combo = adw::ComboRow::new();
        interest_type_combo.set_title("Interest Type");
        let interest_type_model = gtk::StringList::new(&[
            "Application",
            "Backup/archive",
            "Command to run",
            "Command to run (terminal)",
            "Config file",
            "Log file",
        ]);
        interest_type_combo.set_model(Some(&interest_type_model));
        interest_type_combo.set_selected(idx);

        prefs_group.add(&interest_type_combo);
    }

    vbox.append(&prefs_group);

    (header_box, vbox)
}
