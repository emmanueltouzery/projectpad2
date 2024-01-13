use diesel::prelude::*;
use std::sync::mpsc;

use adw::prelude::*;
use projectpadsql::models::ProjectNote;

use crate::{notes, sql_thread::SqlFunc, widgets::project_item::WidgetMode};

use super::common;

pub fn get_note_toolbar() -> gtk::Box {
    let toolbar = gtk::Box::builder().css_classes(["toolbar"]).build();

    toolbar.append(&gtk::Button::builder().icon_name("heading").build());
    toolbar.append(&gtk::Button::builder().icon_name("list-ul").build());
    toolbar.append(&gtk::Button::builder().icon_name("list-ol").build());
    toolbar.append(&gtk::Button::builder().icon_name("bold").build());
    toolbar.append(&gtk::Button::builder().icon_name("italic").build());
    toolbar.append(&gtk::Button::builder().icon_name("strikethrough").build());
    toolbar.append(&gtk::Button::builder().icon_name("link").build());
    toolbar.append(&gtk::Button::builder().icon_name("lock").build());
    toolbar.append(&gtk::Button::builder().icon_name("code").build());
    toolbar.append(&gtk::Button::builder().icon_name("quote").build());
    return toolbar;
}

pub fn load_and_display_note(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    note_id: Option<i32>,
    widget_mode: WidgetMode,
) {
    let (sender, receiver) = async_channel::bounded(1);
    let nid = note_id.unwrap();
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::project_note::dsl as prj_note;
            let note = prj_note::project_note
                .filter(prj_note::id.eq(nid))
                .first::<ProjectNote>(sql_conn)
                .unwrap();
            sender.send_blocking(note).unwrap();
        }))
        .unwrap();
    let p = parent.clone();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_note(&p, channel_data, widget_mode);
    });
}

fn display_note(parent: &adw::Bin, note: ProjectNote, widget_mode: WidgetMode) {
    let vbox = common::get_contents_box_with_header(&note.title, widget_mode);
    parent.set_child(Some(&vbox));

    let text_view = get_note_contents_widget(&note.contents, widget_mode);
    let scrolled_text_view = gtk::ScrolledWindow::builder()
        .child(&text_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    vbox.append(&scrolled_text_view);
}

pub fn get_note_contents_widget(contents: &str, widget_mode: WidgetMode) -> gtk::Widget {
    if widget_mode == WidgetMode::Show {
        gtk::TextView::builder()
            .buffer(
                &notes::note_markdown_to_text_buffer(contents, &crate::notes::build_tag_table())
                    .buffer,
            )
            .editable(false)
            .build()
            .upcast::<gtk::Widget>()
    } else {
        let buf = sourceview5::Buffer::with_language(
            &sourceview5::LanguageManager::default()
                .language("markdown")
                .unwrap(),
        );
        // https://stackoverflow.com/a/63351603/516188
        // TODO don't hardcode sourceview to dark mode
        // dbg!(&sourceview5::StyleSchemeManager::default().scheme_ids());
        buf.set_property(
            "style-scheme",
            sourceview5::StyleSchemeManager::default().scheme("Adwaita-dark"),
        );
        buf.set_text(contents);
        let view = sourceview5::View::with_buffer(&buf);
        view.set_vexpand(true);
        let text_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        text_box.append(&get_note_toolbar());
        text_box.append(&view);
        text_box.upcast::<gtk::Widget>()
    }
}
