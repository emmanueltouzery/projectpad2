use projectpadsql::models::InterestType;

pub mod server_database_view_edit;
pub mod server_extra_user_account_view_edit;
pub mod server_poi_view_edit;
pub mod server_website_view_edit;

pub fn interest_type_get_icon(interest_type: InterestType) -> &'static str {
    match interest_type {
        InterestType::PoiLogFile => "log-file",
        InterestType::PoiConfigFile => "config-file",
        InterestType::PoiApplication => "folder-plus",
        InterestType::PoiCommandToRun => "cog",
        InterestType::PoiBackupArchive => "archive",
        InterestType::PoiCommandTerminal => "terminal",
    }
}
