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
    fn get_widget(&self) -> &gtk::Grid {
        match self {
            AddEditDialogComponent::Poi(ref x) => x.widget(),
            AddEditDialogComponent::Db(ref x) => x.widget(),
            AddEditDialogComponent::User(ref x) => x.widget(),
        }
    }
}
