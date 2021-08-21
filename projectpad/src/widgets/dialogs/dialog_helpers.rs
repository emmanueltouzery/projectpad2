use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use std::sync::mpsc;

pub fn init_group_control(groups_store: &gtk::ListStore, group: &gtk::ComboBoxText) {
    let completion = gtk::EntryCompletion::new();
    completion.set_model(Some(groups_store));
    completion.set_text_column(0);
    group
        .get_child()
        .unwrap()
        .dynamic_cast::<gtk::Entry>()
        .unwrap()
        .set_completion(Some(&completion));
}

pub fn fill_groups(
    groups_store: &gtk::ListStore,
    group_widget: &gtk::ComboBoxText,
    groups: &[String],
    cur_group_name: &Option<String>,
) {
    for group in groups {
        let iter = groups_store.append();
        groups_store.set_value(&iter, 0, &glib::Value::from(&group));
        group_widget.append_text(group);
    }

    if let Some(t) = cur_group_name.as_deref() {
        group_widget
            .get_child()
            .unwrap()
            .dynamic_cast::<gtk::Entry>()
            .unwrap()
            .set_text(t);
    }
}

pub fn style_grid(grid: &gtk::Grid) {
    grid.set_margin_start(30);
    grid.set_margin_end(30);
    grid.set_margin_top(10);
    grid.set_margin_bottom(10);
    grid.set_row_spacing(5);
    grid.set_column_spacing(10);
}

pub fn fetch_server_groups(
    groups_sender: &relm::Sender<Vec<String>>,
    server_id: i32,
    db_sender: &mpsc::Sender<SqlFunc>,
) {
    let s = groups_sender.clone();
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            s.send(projectpadsql::get_server_group_names(sql_conn, server_id))
                .unwrap();
        }))
        .unwrap();
}

pub trait ServerItemDialogModelParam<T> {
    fn get_item(&self) -> Option<&T>;
    fn get_accel_group(&self) -> &gtk::AccelGroup;
}

impl<T> ServerItemDialogModelParam<T> for (mpsc::Sender<SqlFunc>, i32, Option<T>, gtk::AccelGroup) {
    fn get_item(&self) -> Option<&T> {
        self.2.as_ref()
    }

    fn get_accel_group(&self) -> &gtk::AccelGroup {
        &self.3
    }
}

impl<T> ServerItemDialogModelParam<T> for (mpsc::Sender<SqlFunc>, Option<T>, gtk::AccelGroup) {
    fn get_item(&self) -> Option<&T> {
        self.1.as_ref()
    }

    fn get_accel_group(&self) -> &gtk::AccelGroup {
        &self.2
    }
}

pub fn prepare_dialog_param<T>(
    db_sender: mpsc::Sender<SqlFunc>,
    parent_id: i32,
    val: Option<T>,
) -> (mpsc::Sender<SqlFunc>, i32, Option<T>, gtk::AccelGroup) {
    (db_sender, parent_id, val, gtk::AccelGroup::new())
}

/// you must keep a reference to the component in your model,
/// otherwise event processing will die when the component gets dropped
pub fn prepare_add_edit_item_dialog<T, Dlg>(
    widget_for_window: gtk::Widget,
    widget_param: Dlg::ModelParam,
    ok_pressed_event: Dlg::Msg,
    item_desc: &'static str,
) -> (gtk::Dialog, relm::Component<Dlg>, gtk::Button)
where
    Dlg: relm::Widget + 'static,
    Dlg::Msg: Clone,
    Dlg::ModelParam: ServerItemDialogModelParam<T>,
{
    let title = if widget_param.get_item().is_some() {
        "Edit "
    } else {
        "Add "
    }
    .to_string()
        + item_desc;
    let accel_group = widget_param.get_accel_group();
    let dialog = standard_dialogs::modal_dialog(widget_for_window, 600, 150, title);
    dialog.add_accel_group(accel_group);
    let dialog_contents =
        relm::init::<Dlg>(widget_param).expect("error initializing the server item add edit modal");
    let d_c = dialog_contents.stream();
    standard_dialogs::prepare_custom_dialog(dialog, dialog_contents, move |_| {
        d_c.emit(ok_pressed_event.clone());
    })
}
