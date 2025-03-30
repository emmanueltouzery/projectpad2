use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use std::str::FromStr;

use crate::{
    search_engine::SearchItemsType,
    widgets::{
        project_item::WidgetMode,
        project_items::common,
        search::{
            search_item_model::{SearchItemModel, SearchItemType},
            search_picker::SearchPicker,
        },
    },
    win::ProjectpadApplicationWindow,
};

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
        search_items_type: Rc<RefCell<String>>,

        #[property(get, set)]
        search_item_type: Rc<RefCell<u8>>,

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
                vec![Signal::builder("item-picked")
                    .param_types([i32::static_type()])
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

        let widget = gtk::Button::builder().css_classes(["flat"]).build();
        widget.connect_closure(
            "clicked",
            false,
            glib::closure_local!(@strong this as s => move |_: gtk::Button| {
                // let app = gio::Application::default()
                //     .expect("Failed to retrieve application singleton")
                //     .downcast::<ProjectpadApplication>()
                //     .unwrap();
                // let window = app.imp().window.get().unwrap();
                // let win_binding = window.upgrade();
                // let win_binding_ref = win_binding.as_ref().unwrap();
                // let file_dialog = gtk::FileDialog::builder().build();
                if widget_mode == WidgetMode::Edit {
                    s.open_item_picker_dlg();
                } else {
                    let search_item_type = SearchItemType::from_repr(s.search_item_type());
                    let item_id = s.item_id();
                    let project_id_server_id_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                        Self::query_get_item_project_id_server_id(sql_conn, search_item_type, Some(item_id).filter(|i| *i > 0))
                    }));
                    glib::spawn_future_local(async move {
                        let (project_id, server_id) = project_id_server_id_recv.recv().await.unwrap();
                        ProjectpadApplicationWindow::display_item_from_search(
                            common::main_win(), project_id, item_id, search_item_type.unwrap() as u8, server_id);
                    });
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

        this.bind_property("search-item-type", &widget, "icon-name")
            .transform_to(move |_, sit: u8| {
                if widget_mode == WidgetMode::Show {
                    SearchItemType::from_repr(sit).map(SearchItemModel::get_search_item_type_icon)
                } else {
                    Some("document-edit-symbolic")
                }
            })
            .sync_create()
            .build();

        this
    }

    fn query_get_item_project_id_server_id(
        sql_conn: &mut SqliteConnection,
        search_item_type: Option<SearchItemType>,
        item_id: Option<i32>,
    ) -> (i32, i32) {
        let server_id = ProjectpadApplicationWindow::query_search_item_get_server_id(
            sql_conn,
            search_item_type,
            item_id,
        );

        if let Some(sid) = server_id {
            use projectpadsql::schema::server::dsl as srv;
            (
                srv::server
                    .filter(srv::id.eq(sid))
                    .select(srv::project_id)
                    .first::<i32>(sql_conn)
                    .unwrap(),
                sid,
            )
        } else {
            panic!("only coded for server items. for project items will have to switch on which project item type to find the project id.");
        }
    }

    fn open_item_picker_dlg(&self) {
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
        self.bind_property("search-items-type", &search_picker, "search-items-type")
            .sync_create()
            .build();
        search_picker.set_margin_start(10);
        search_picker.set_margin_end(10);
        search_picker.set_margin_top(10);
        search_picker.set_margin_bottom(10);

        // pre-select the current item if any in the search picker
        let item_id = Some(self.item_id()).filter(|i| *i > 0);
        if let Some(id) = item_id {
            if let Some(search_item_type) = SearchItemType::from_repr(self.search_item_type()) {
                search_picker.refresh_search(Some((search_item_type, id)));
            }
        }

        vbox.append(&search_picker);

        search_picker
            .bind_property("selected-item-search-item-type", &save_btn, "sensitive")
            .transform_to(move |binding, sit: u8| {
                let sp = binding
                    .source()
                    .unwrap()
                    .downcast::<SearchPicker>()
                    .unwrap();
                let search_item_type = SearchItemType::from_repr(sit);
                let prop_search_item_type = match SearchItemsType::from_str(&sp.search_items_type())
                {
                    Ok(sit) => sit,
                    Err(_) => SearchItemsType::All,
                };
                Some(match prop_search_item_type {
                    SearchItemsType::All => true,
                    SearchItemsType::ServersOnly => {
                        search_item_type == Some(SearchItemType::Server)
                    }
                    SearchItemsType::ServerDbsOnly => {
                        search_item_type == Some(SearchItemType::ServerDatabase)
                    }
                })
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
            s.emit_by_name::<()>("item-picked", &[&db_id]);
        });

        dialog.present(&common::main_win());
    }
}
