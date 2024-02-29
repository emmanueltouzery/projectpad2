use adw::prelude::*;
use diesel::prelude::*;
use std::sync::mpsc;

use projectpadsql::models::{InterestType, ProjectPointOfInterest};

use crate::{sql_thread::SqlFunc, widgets::project_item::WidgetMode};

use super::common::{self, DetailsRow, SuffixAction};

pub fn load_and_display_project_poi(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    project_poi_id: Option<i32>,
    widget_mode: WidgetMode,
) {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
            let pid = project_poi_id.unwrap();
            let poi = prj_poi::project_point_of_interest
                .filter(prj_poi::id.eq(pid))
                .first::<ProjectPointOfInterest>(sql_conn)
                .unwrap();

            sender.send_blocking(poi).unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    glib::spawn_future_local(async move {
        let poi = receiver.recv().await.unwrap();
        display_project_oi(&p, poi, widget_mode);
    });
}

fn poi_get_text_label(interest_type: InterestType) -> &'static str {
    match interest_type {
        InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
        _ => "Text",
    }
}

fn display_project_oi(parent: &adw::Bin, poi: ProjectPointOfInterest, widget_mode: WidgetMode) {
    let vbox =
        common::get_contents_box_with_header(&poi.desc, common::EnvOrEnvs::None, widget_mode);

    let desc = match poi.interest_type {
        InterestType::PoiLogFile => "Log file",
        InterestType::PoiConfigFile => "Config file",
        InterestType::PoiApplication => "Application",
        InterestType::PoiCommandToRun => "Command to run",
        InterestType::PoiBackupArchive => "Backup/Archive",
        InterestType::PoiCommandTerminal => "Command to run",
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

    vbox.append(&prefs_group);
    parent.set_child(Some(&vbox));
}
