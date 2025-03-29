use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::{
    app::ProjectpadApplication,
    search_engine::SearchItemsType,
    widgets::{
        project_item::WidgetMode,
        search::{search_item_model::SearchItemType, search_picker::SearchPicker},
    },
};

use super::common;

mod imp {
    use std::{cell::RefCell, rc::Rc, sync::OnceLock};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use subclass::Signal;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::ProjectpadItemActionRow)]
    pub struct ProjectpadItemActionRow {
        #[property(get, set)]
        text: Rc<RefCell<String>>,

        #[property(get, set)]
        item_id: Rc<RefCell<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectpadItemActionRow {
        const NAME: &'static str = "ProjectpadItemActionRow";
        type ParentType = adw::ActionRow;
        type Type = super::ProjectpadItemActionRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectpadItemActionRow {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("file-picked")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ProjectpadItemActionRow {}

    impl gtk::subclass::prelude::ListBoxRowImpl for ProjectpadItemActionRow {}
    impl adw::subclass::prelude::PreferencesRowImpl for ProjectpadItemActionRow {}
    impl adw::subclass::prelude::ActionRowImpl for ProjectpadItemActionRow {}
}

glib::wrapper! {
    pub struct ProjectpadItemActionRow(ObjectSubclass<imp::ProjectpadItemActionRow>)
        @extends gtk::Widget, adw::PreferencesRow, adw::ActionRow;
}

impl ProjectpadItemActionRow {
    pub fn new(widget_mode: WidgetMode) -> Self {
        let this = glib::Object::new::<Self>();

        // this.bind_property("text", this.upcast_ref::<adw::PreferencesRow>(), "text")
        //     .sync_create()
        //     .build();

        // .title(glib::markup_escape_text(self.title))
        // .subtitle(glib::markup_escape_text(subtitle))
        // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
        // When used together with the .property style class, AdwActionRow and
        // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
        this.set_css_classes(&["property"]);

        let widget = gtk::Button::builder()
            .css_classes(["flat"])
            .icon_name(if widget_mode == WidgetMode::Show {
                "document-save-symbolic"
            } else {
                "document-open-symbolic"
            })
            .build();
        widget.connect_closure(
            "clicked",
            false,
            glib::closure_local!(@strong this as s => move |b: gtk::Button| {
                // let app = gio::Application::default()
                //     .expect("Failed to retrieve application singleton")
                //     .downcast::<ProjectpadApplication>()
                //     .unwrap();
                // let window = app.imp().window.get().unwrap();
                // let win_binding = window.upgrade();
                // let win_binding_ref = win_binding.as_ref().unwrap();
                // let file_dialog = gtk::FileDialog::builder().build();
                if b.icon_name() == Some("document-open-symbolic".into()) {
                    s.open_database_picker_dlg();
                    // let _s = s.clone();
                    // file_dialog.open(Some(win_binding_ref), None::<&gio::Cancellable>, move |r| {
                    //     if let Ok(f) = r {
                    //         if let Some(p) = f.path() {
                    //             // TODO a little crappy unwrap, could be invalid filename
                    //             _s.set_filename(p.to_str().unwrap());
                    //         }
                    //     }
                    // });
                } else {
                    // let _s = s.clone();
                    // file_dialog.save(Some(win_binding_ref), None::<&gio::Cancellable>, move |r| {
                    //     if let Ok(f) = r {
                    //         if let Some(p) = f.path() {
                    //             dbg!(&p);
                    //             _s.emit_by_name::<()>("file-picked", &[&p.display().to_string()]);
                    //         }
                    //     }
                    // });
                }
            }),
        );
        this.add_suffix(&widget);

        if widget_mode == WidgetMode::Edit {
            let delete_widget = gtk::Button::builder()
                .css_classes(["flat"])
                .icon_name("edit-delete-symbolic")
                .build();
            this.add_suffix(&delete_widget);
            delete_widget.connect_closure(
                "clicked",
                false,
                glib::closure_local!(@strong this as s => move |_b: gtk::Button| {
                    // s.set_filename("");
                }),
            );
        }

        this.set_activatable_widget(Some(&widget));

        this.bind_property("text", this.upcast_ref::<adw::PreferencesRow>(), "subtitle")
            .sync_create()
            .build();

        this
    }

    fn open_database_picker_dlg(&self) {
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header_bar = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(false)
            .build();
        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        header_bar.pack_start(&cancel_btn);

        let save_btn = gtk::Button::builder()
            .label("Save")
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();
        header_bar.pack_end(&save_btn);

        vbox.append(&header_bar);

        let search_picker = SearchPicker::new();
        search_picker.set_search_item_types(SearchItemsType::ServerDbsOnly.to_string());
        search_picker.set_margin_start(10);
        search_picker.set_margin_end(10);
        search_picker.set_margin_top(10);
        search_picker.set_margin_bottom(10);
        vbox.append(&search_picker);

        search_picker
            .bind_property("selected-item-search-item-type", &save_btn, "sensitive")
            .transform_to(|_, sit: u8| {
                let search_item_type = SearchItemType::from_repr(sit);
                Some(search_item_type == Some(SearchItemType::ServerDatabase))
            })
            .sync_create()
            .build();

        let dialog = adw::Dialog::builder()
            .title("Pick item")
            .content_width(600)
            .content_height(600)
            .child(&vbox)
            .build();

        let dlg = dialog.clone();
        cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
            dlg.close();
        });

        let dlg = dialog.clone();
        let s = self.clone();
        let sp = search_picker.clone();
        save_btn.connect_clicked(move |_btn: &gtk::Button| {
            dlg.close();
            let db_id = sp.selected_item_item_id();
            let db_name_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as srv_db;

                srv_db::server_database
                    .filter(srv_db::id.eq(&db_id))
                    .select(srv_db::desc)
                    .first::<String>(sql_conn)
                    .unwrap()
            }));
            let s = s.clone();
            glib::spawn_future_local(async move {
                let db_name = db_name_recv.recv().await.unwrap();
                let _guard = s.freeze_notify();
                s.set_item_id(db_id);
                s.set_text(db_name);
            });
        });

        let app = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<ProjectpadApplication>()
            .unwrap();
        dialog.present(&app.active_window().unwrap());
    }
}
