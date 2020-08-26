#[macro_use]
pub mod dialog_helpers;
mod auth_key_button;
pub mod server_add_edit_dlg;
pub mod server_add_item_dlg;
pub mod server_database_add_edit_dlg;
pub mod server_extra_user_add_edit_dlg;
pub mod server_poi_add_edit_dlg;
pub mod standard_dialogs;

pub enum AddEditDialogComponent {
    Poi(relm::Component<server_poi_add_edit_dlg::ServerPoiAddEditDialog>),
    Db(relm::Component<server_database_add_edit_dlg::ServerDatabaseAddEditDialog>),
    User(relm::Component<server_extra_user_add_edit_dlg::ServerExtraUserAddEditDialog>),
}

impl AddEditDialogComponent {
    fn un_poi(&self) -> Option<&relm::Component<server_poi_add_edit_dlg::ServerPoiAddEditDialog>> {
        match self {
            AddEditDialogComponent::Poi(ref x) => Some(x),
            _ => None,
        }
    }

    fn un_db(
        &self,
    ) -> Option<&relm::Component<server_database_add_edit_dlg::ServerDatabaseAddEditDialog>> {
        match self {
            AddEditDialogComponent::Db(ref x) => Some(x),
            _ => None,
        }
    }
}
