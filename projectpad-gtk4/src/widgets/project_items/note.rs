use crate::{
    notes::{text_tag_search_match, TAG_SEARCH_HIGHLIGHT},
    perform_insert_or_update,
    widgets::{project_item_list::ProjectItemList, project_item_model::ProjectItemType},
};
use diesel::prelude::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::mpsc};

use adw::prelude::*;
use glib::property::PropertySet;
use glib::*;
use gtk::{gdk, subclass::prelude::*};
use projectpadsql::{
    get_project_group_names,
    models::{EnvironmentType, ProjectNote, ServerNote},
};

use crate::{
    app::ProjectpadApplication,
    notes::{self, ItemDataInfo},
    sql_thread::SqlFunc,
    widgets::{
        project_item::WidgetMode,
        project_items::common::{copy_to_clipboard, display_item_edit_dialog, DialogClamp},
        search_bar::SearchBar,
    },
};

use super::{
    common::EnvOrEnvs, item_header_edit::ItemHeaderEdit, item_header_view::ItemHeaderView,
};

#[derive(Clone, Default)]
pub struct NoteInfo<'a> {
    pub id: i32,
    pub title: &'a str,
    pub env: EnvOrEnvs,
    pub contents: &'a str,
    pub display_header: bool,
    pub group_name: Option<&'a str>,
    pub all_group_names: &'a [String],
}

#[derive(Debug)]
pub struct NoteMetaData {
    pub note_links: Vec<ItemDataInfo>,
    pub note_passwords: Vec<ItemDataInfo>,
    pub header_iters: Vec<gtk::TextIter>,
}

mod imp {
    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::Note)]
    pub struct Note {
        #[property(get, set)]
        edit_mode: Cell<bool>,

        #[property(get, set)]
        pub project_note_id: Cell<i32>,

        #[property(get, set)]
        pub server_note_id: Cell<i32>,

        pub text_view: Rc<RefCell<Option<(gtk::TextView, NoteMetaData)>>>,
        pub text_edit: Rc<RefCell<Option<sourceview5::View>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Note {
        const NAME: &'static str = "Note";
        type ParentType = adw::Bin;
        type Type = super::Note;

        fn class_init(_klass: &mut Self::Class) {
            // Self::bind_template(klass);
        }

        fn instance_init(_obj: &subclass::InitializingObject<Self>) {
            // obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Note {
        fn constructed(&self) {
            //     self.obj().init_list();
            let _ = self.obj().connect_edit_mode_notify(|note: &super::Note| {
                println!("edit mode changed");
                let server_note_id = note.imp().server_note_id.get();
                if server_note_id != 0 {
                    note.load_and_display_server_note(note.imp().server_note_id.get());
                } else if dbg!(note.imp().project_note_id.get()) != 0 {
                    note.load_and_display_project_note(note.imp().project_note_id.get());
                } else {
                    // note that both IDs will be 0 when creating a new note
                    note.display_note_contents(NoteInfo {
                        id: 0,
                        title: "",
                        env: EnvOrEnvs::None,
                        contents: "",
                        display_header: false,
                        group_name: None,
                        all_group_names: &[],
                    });
                }
            });
            let _ = self
                .obj()
                .connect_server_note_id_notify(|note: &super::Note| {
                    if note.imp().server_note_id.get() != 0 {
                        note.load_and_display_server_note(note.imp().server_note_id.get());
                    }
                });
            let _ = self
                .obj()
                .connect_project_note_id_notify(|note: &super::Note| {
                    if note.imp().project_note_id.get() != 0 {
                        note.load_and_display_project_note(note.imp().project_note_id.get());
                    }
                });
        }
    }

    impl WidgetImpl for Note {}

    impl adw::subclass::prelude::BinImpl for Note {}
}

glib::wrapper! {
    pub struct Note(ObjectSubclass<imp::Note>)
        @extends gtk::Widget, adw::Bin;
}

impl Note {
    pub fn new() -> Self {
        let note = glib::Object::new::<Self>();
        // note.imp().project_item_list.connect_activate(
        //     glib::clone!(@weak win as w => move |project_item_id, project_item_type| {
        //         w.imp().project_item.set_project_item_type(project_item_type as u8);
        //         w.imp().project_item.set_item_id(project_item_id)
        //     }),
        // );
        note
    }

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

    fn load_and_display_project_note(&self, note_id: i32) {
        let db_sender = Self::get_db_sender();
        let (sender, receiver) = async_channel::bounded(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_note::dsl as prj_note;
                let note = prj_note::project_note
                    .filter(prj_note::id.eq(note_id))
                    .first::<ProjectNote>(sql_conn)
                    .unwrap();

                let project_group_names = get_project_group_names(sql_conn, note.project_id);

                sender.send_blocking((note, project_group_names)).unwrap();
            }))
            .unwrap();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let (channel_data, project_group_names) = receiver.recv().await.unwrap();
            p.display_note_contents(NoteInfo {
                id: channel_data.id,
                title: &channel_data.title,
                env: EnvOrEnvs::Envs(Self::get_envs(&channel_data)),
                contents: &channel_data.contents,
                display_header: true,
                group_name: channel_data.group_name.as_deref(),
                all_group_names: &project_group_names,
            });
        });
    }

    pub fn get_envs(project_note: &ProjectNote) -> HashSet<EnvironmentType> {
        let mut env_set = HashSet::new();
        if project_note.has_uat {
            env_set.insert(EnvironmentType::EnvUat);
        }
        if project_note.has_dev {
            env_set.insert(EnvironmentType::EnvDevelopment);
        }
        if project_note.has_stage {
            env_set.insert(EnvironmentType::EnvStage);
        }
        if project_note.has_prod {
            env_set.insert(EnvironmentType::EnvProd);
        }
        env_set
    }

    fn load_and_display_server_note(&self, note_id: i32) {
        let db_sender = Self::get_db_sender();
        let (sender, receiver) = async_channel::bounded(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_note::dsl as srv_note;
                let note = srv_note::server_note
                    .filter(srv_note::id.eq(note_id))
                    .first::<ServerNote>(sql_conn)
                    .unwrap();
                sender.send_blocking(note).unwrap();
            }))
            .unwrap();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let channel_data = receiver.recv().await.unwrap();
            p.display_note_contents(NoteInfo {
                id: channel_data.id,
                title: &channel_data.title,
                env: EnvOrEnvs::None,
                contents: &channel_data.contents,
                display_header: false,
                group_name: None,
                all_group_names: &[],
            });
        });
    }

    fn get_db_sender() -> mpsc::Sender<SqlFunc> {
        let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
        app.unwrap().get_sql_channel()
    }

    fn note_toc_menu(note: &NoteInfo) -> gtk::PopoverMenu {
        let note_poc_menu = gio::Menu::new();
        let mut options = pulldown_cmark::Options::empty();
        options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
        let parser = pulldown_cmark::Parser::new_ext(note.contents, options);
        let mut header_idx = 0;

        let mut is_in_header = None::<usize>;
        parser.for_each(|evt| {
            match (&is_in_header, evt) {
                (Some(level), pulldown_cmark::Event::Text(v)) => {
                    note_poc_menu.append(
                        Some(&("    ".repeat(*level - 1) + " " + &v)),
                        Some(&format!("menu_actions.jump_to_header({header_idx})")),
                    );
                    is_in_header = None;
                    header_idx += 1;
                }
                (_, pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading(level))) => {
                    is_in_header = level.try_into().ok();
                }
                _ => {}
            };
        });
        gtk::PopoverMenu::builder()
            .menu_model(&note_poc_menu)
            .build()
    }

    fn display_note_contents(&self, note: NoteInfo) {
        let widget_mode = if self.edit_mode() {
            WidgetMode::Edit
        } else {
            WidgetMode::Show
        };
        if note.display_header {
            // project note, we handle the editing
            let (header_box, vbox, _) = self.note_contents(note.clone(), WidgetMode::Show);

            let toc_menu = Self::note_toc_menu(&note);
            let toc_btn = gtk::MenuButton::builder()
                .icon_name("list-ol")
                .valign(gtk::Align::Center)
                .halign(gtk::Align::End)
                .popover(&toc_menu)
                .build();
            if widget_mode != WidgetMode::Edit {
                toc_btn.set_hexpand(true);
            }
            if toc_menu.menu_model().unwrap().n_items() == 0 {
                toc_btn.set_sensitive(false);
            }
            header_box.append(&toc_btn);

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

            let t = note.title.to_owned();
            let c = note.contents.to_owned();
            let g = note.group_name.map(|g| g.to_owned());
            let a = note.all_group_names.to_vec();
            let tv_var = self.imp().text_edit.clone();
            let s = self.clone();

            edit_btn.connect_closure(
                    "clicked",
                    false,
                    glib::closure_local!(@strong s as _s,
                                         @strong t as _t,
                                         @strong c as _c,
                                         @strong g as _g,
                                         @strong a as _a,
                                         @strong tv_var as tv,
                                         @strong vbox as v => move |_b: gtk::Button| {
                        let n = NoteInfo {
                            id: note.id,
                            title: &_t,
                            env: note.env.clone(),
                            contents: &_c,
                            display_header: note.display_header,
                            group_name: _g.as_deref(),
                            all_group_names: &_a
                        };
                        let (_, vbox, project_item_header_edit) = s.note_contents(n.clone(),  WidgetMode::Edit);
                        vbox.set_margin_start(30);
                        vbox.set_margin_end(30);

                        let (dialog, save_btn) = display_item_edit_dialog(&v, "Edit Note", vbox, 6000, 6000, DialogClamp::No);
                        let ttv = tv.clone();
                        let project_note_id = note.id;
                        let s = _s.clone();
                        let h_e = project_item_header_edit.clone();
                        save_btn.connect_clicked(move |_| {
                            match (&*ttv.borrow(), &h_e) {
                                ( Some(v), Some(header_edit) ) => {
                                    let receiver = Self::save_project_note(v, header_edit, Some(project_note_id));
                                    let d = dialog.clone();
                                    let s1 = s.clone();
                                    glib::spawn_future_local(async move {
                                        let project_note_after_result = receiver.recv().await.unwrap();
                                        d.close();
                                        // s1.clone().load_and_display_project_note(project_note_id);
                                        ProjectItemList::display_project_item(project_note_id, ProjectItemType::ProjectNote);
                                    });
                                },
                                _ => panic!()
                            }
                        });
                    }),
                    );
            self.set_child(Some(&vbox));
        } else {
            // server note, the parent handles the editing
            let vbox = self.note_contents(note, widget_mode).1;
            self.set_child(Some(&vbox));
        }
    }

    pub fn save_project_note(
        v: &sourceview5::View,
        header_edit: &ItemHeaderEdit,
        project_note_id: Option<i32>,
    ) -> async_channel::Receiver<Result<ProjectNote, (String, Option<String>)>> {
        let buf = v.buffer();
        let start_iter = buf.start_iter();
        let end_iter = buf.end_iter();
        let new_contents = v.buffer().text(&start_iter, &end_iter, false);
        let app = gio::Application::default()
            .and_downcast::<ProjectpadApplication>()
            .unwrap();
        let (sender, receiver) = async_channel::bounded(1);
        let db_sender = app.get_sql_channel();
        let title = header_edit.title();
        let group_name = header_edit.group_name();
        let has_dev = header_edit.property::<bool>("env_dev");
        let has_stg = header_edit.property::<bool>("env_stg");
        let has_uat = header_edit.property::<bool>("env_uat");
        let has_prd = header_edit.property::<bool>("env_prd");
        let win = app.imp().window.get().unwrap().upgrade().unwrap();
        let project_id = glib::VariantDict::new(win.action_state("select-project-item").as_ref())
            .lookup::<i32>("project_id")
            .unwrap()
            .unwrap();
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_note::dsl as prj_note;
                let changeset = (
                    prj_note::title.eq(title.as_str()),
                    // // never store Some("") for group, we want None then.
                    prj_note::group_name.eq(Some(&group_name).filter(|s| !s.is_empty())),
                    prj_note::contents.eq(new_contents.as_str()),
                    prj_note::has_dev.eq(has_dev),
                    prj_note::has_stage.eq(has_stg),
                    prj_note::has_uat.eq(has_uat),
                    prj_note::has_prod.eq(has_prd),
                    prj_note::project_id.eq(project_id),
                );
                let project_note_after_result = perform_insert_or_update!(
                    sql_conn,
                    project_note_id,
                    prj_note::project_note,
                    prj_note::id,
                    changeset,
                    ProjectNote,
                );
                sender.send_blocking(project_note_after_result).unwrap();
            }))
            .unwrap();
        receiver
    }

    pub fn note_contents(
        &self,
        note: NoteInfo,
        widget_mode: WidgetMode,
    ) -> (gtk::Box, gtk::Box, Option<ItemHeaderEdit>) {
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let (note_view, scrolled_window) =
            self.get_note_contents_widget(&note.contents, widget_mode);

        match (widget_mode, &*self.imp().text_view.borrow()) {
            (WidgetMode::Show, Some((_, note_metadata))) => {
                let action_group = gio::SimpleActionGroup::new();
                let h_i = note_metadata.header_iters.clone();
                action_group.add_action_entries([gio::ActionEntry::builder("jump_to_header")
                    .parameter_type(Some(&i32::static_variant_type()))
                    .activate(move |_, _action, parameter| {
                        let idx = parameter.unwrap().get::<i32>().unwrap();
                        if let Some(tv) = scrolled_window.child().and_downcast::<gtk::TextView>() {
                            let mut target_iter = h_i[usize::try_from(idx).unwrap()].clone();
                            tv.scroll_to_iter(&mut target_iter, 0.0, true, 0.0, 0.0);
                        }
                    })
                    .build()]);
                self.insert_action_group("menu_actions", Some(&action_group));
            }
            _ => {}
        }

        let (maybe_project_item_header_edit, header_box) = if widget_mode == WidgetMode::Edit {
            let project_item_header = ItemHeaderEdit::new(
                ProjectItemType::ProjectNote.get_icon(),
                note.group_name,
                note.all_group_names,
                note.env,
            );
            project_item_header.set_title(note.title);
            if note.display_header {
                vbox.append(&project_item_header);
            }
            let hbox = project_item_header.header_box();
            (Some(project_item_header), hbox)
        } else {
            let project_item_header = ItemHeaderView::new(ProjectItemType::ProjectNote);
            project_item_header.set_title(note.title);
            if note.display_header {
                vbox.append(&project_item_header);
            }
            (None, project_item_header.header_box())
        };

        vbox.append(&note_view);

        (header_box, vbox, maybe_project_item_header_edit)
    }

    pub fn get_contents_text(&self) -> GString {
        let text_edit_b = self.imp().text_edit.borrow();
        let text_edit = text_edit_b.as_ref().unwrap();

        let buf = text_edit.buffer();
        let start_iter = buf.start_iter();
        let end_iter = buf.end_iter();
        text_edit.buffer().text(&start_iter, &end_iter, false)
    }

    pub fn get_note_contents_widget(
        &self,
        contents: &str,
        widget_mode: WidgetMode,
    ) -> (gtk::Widget, gtk::ScrolledWindow) {
        let toast_parent = adw::ToastOverlay::new();
        let text_view = if widget_mode == WidgetMode::Show {
            let note_buffer_info =
                notes::note_markdown_to_text_buffer(contents, &crate::notes::build_tag_table());
            let text_view = gtk::TextView::builder()
                .buffer(&note_buffer_info.buffer)
                .left_margin(10)
                .right_margin(10)
                .top_margin(10)
                .bottom_margin(10)
                .editable(false)
                .build();
            self.imp().text_view.set(Some((
                text_view.clone(),
                NoteMetaData {
                    note_links: note_buffer_info.links,
                    note_passwords: note_buffer_info.passwords,
                    header_iters: note_buffer_info.header_iters,
                },
            )));
            self.register_events(&text_view, &toast_parent);

            for anchor in &note_buffer_info.separator_anchors {
                let sep = gtk::Separator::builder()
                    .margin_top(15)
                    .margin_bottom(15)
                    .width_request(350)
                    .build();
                text_view.add_child_at_anchor(&sep, anchor);
            }

            text_view.upcast::<gtk::Widget>()
        } else {
            let buf = sourceview5::Buffer::with_language(
                &sourceview5::LanguageManager::default()
                    .language("markdown")
                    .unwrap(),
            );
            buf.upcast_ref::<gtk::TextBuffer>()
                .tag_table()
                .add(&text_tag_search_match());
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
            view.set_wrap_mode(gtk::WrapMode::Word);
            self.imp().text_edit.set(Some(view.clone()));
            view.upcast::<gtk::Widget>() // TODO buffer_iters?
        };

        let scrolled_text_view = gtk::ScrolledWindow::builder()
            .child(&text_view)
            .vexpand(true)
            .hexpand(true)
            .build();

        let overlay = gtk::Overlay::builder().child(&scrolled_text_view).build();
        let search_bar = SearchBar::new();
        let revealer = gtk::Revealer::builder()
            .child(&search_bar)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .vexpand(false)
            .build();
        overlay.add_overlay(&revealer);
        toast_parent.set_child(Some(&overlay));

        let r = revealer.clone();
        let tv = self.imp().text_view.clone();
        let te = self.imp().text_edit.clone();
        search_bar.connect_closure(
            "esc-pressed",
            false,
            glib::closure_local!(move |_: SearchBar| {
                r.set_reveal_child(false);
                Self::clear_search(widget_mode, tv.clone(), te.clone());
            }),
        );
        let tv = self.imp().text_view.clone();
        let te = self.imp().text_edit.clone();
        search_bar.connect_closure(
            "search-changed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = &*tv.borrow();
                let cur_te = &*te.borrow();
                match (widget_mode, cur_tv, cur_te) {
                    (WidgetMode::Show, Some((v, _)), _) => Self::apply_search(
                        v,
                        v.buffer().start_iter().forward_search(
                            &search,
                            gtk::TextSearchFlags::all(),
                            None,
                        ),
                    ),
                    (WidgetMode::Edit, _, Some(tv)) => Self::apply_search(
                        tv,
                        tv.buffer().start_iter().forward_search(
                            &search,
                            gtk::TextSearchFlags::all(),
                            None,
                        ),
                    ),
                    _ => {}
                }
            }),
        );
        let tv2 = self.imp().text_view.clone();
        let te2 = self.imp().text_edit.clone();
        search_bar.connect_closure(
            "prev-pressed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = tv2.borrow();
                let cur_te = te2.borrow();
                match (widget_mode, &*cur_tv, &*cur_te) {
                    (WidgetMode::Show, Some((v, _)), _) => {
                        Self::note_search_previous(v, Some(&search))
                    }
                    (WidgetMode::Edit, _, Some(tv)) => {
                        Self::note_search_previous(tv, Some(&search))
                    }
                    _ => {}
                }
            }),
        );
        let tv3 = self.imp().text_view.clone();
        let te3 = self.imp().text_edit.clone();
        search_bar.connect_closure(
            "next-pressed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = tv3.borrow();
                let cur_te = te3.borrow();
                match (widget_mode, &*cur_tv, &*cur_te) {
                    (WidgetMode::Show, Some((v, _)), _) => Self::note_search_next(v, Some(&search)),
                    (WidgetMode::Edit, _, Some(tv)) => Self::note_search_next(tv, Some(&search)),
                    _ => {}
                }
            }),
        );

        let widget = if widget_mode == WidgetMode::Show {
            toast_parent.clone().upcast::<gtk::Widget>()
        } else {
            let vbox = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            vbox.append(&Self::get_note_toolbar());
            vbox.append(&toast_parent);
            vbox.upcast::<gtk::Widget>()
        };

        let key_controller = gtk::EventControllerKey::new();
        let tv = self.imp().text_view.clone();
        let te = self.imp().text_edit.clone();
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
            if keyval.to_unicode() == Some('f') && state.contains(gdk::ModifierType::CONTROL_MASK) {
                revealer.set_reveal_child(true);
                search_bar.grab_focus();
                return glib::Propagation::Stop;
            }
            if keyval == gdk::Key::Escape {
                revealer.set_reveal_child(false);
                Self::clear_search(widget_mode, tv.clone(), te.clone());
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed // Allow other handlers to process the event
        });
        widget.add_controller(key_controller);

        (widget, scrolled_text_view)
    }

    fn clear_search(
        widget_mode: WidgetMode,
        tv: Rc<RefCell<Option<(gtk::TextView, NoteMetaData)>>>,
        te: Rc<RefCell<Option<sourceview5::View>>>,
    ) {
        let cur_tv = &*tv.borrow();
        let cur_te = &*te.borrow();
        let buffer = match (widget_mode, cur_tv, cur_te) {
            (WidgetMode::Show, Some((v, _)), _) => v.buffer(),
            (WidgetMode::Edit, _, Some(tv)) => tv.buffer(),
            _ => {
                panic!()
            }
        };
        buffer.remove_tag_by_name(
            TAG_SEARCH_HIGHLIGHT,
            &buffer.start_iter(),
            &buffer.end_iter(),
        );
    }

    fn note_search_next<T>(textview: &T, note_search_text: Option<&str>)
    where
        T: TextViewExt,
    {
        let buffer = textview.buffer();
        if let Some(search) = note_search_text.clone() {
            Self::apply_search(
                textview,
                buffer
                    .iter_at_offset(buffer.cursor_position() + 1)
                    .forward_search(&search, gtk::TextSearchFlags::all(), None),
            );
        }
    }

    fn note_search_previous<T>(textview: &T, note_search_text: Option<&str>)
    where
        T: TextViewExt,
    {
        let buffer = textview.buffer();
        if let Some(search) = note_search_text.clone() {
            Self::apply_search(
                textview,
                buffer
                    .iter_at_offset(buffer.cursor_position())
                    .backward_search(&search, gtk::TextSearchFlags::all(), None),
            );
        }
    }

    fn apply_search<T>(textview: &T, range: Option<(gtk::TextIter, gtk::TextIter)>)
    where
        T: TextViewExt,
    {
        textview.buffer().remove_tag_by_name(
            TAG_SEARCH_HIGHLIGHT,
            &textview.buffer().start_iter(),
            &textview.buffer().end_iter(),
        );
        if let Some((mut start, end)) = range {
            textview
                .buffer()
                .apply_tag_by_name(TAG_SEARCH_HIGHLIGHT, &start, &end);
            textview.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
            textview.buffer().place_cursor(&start); // so that previous and next work, i need to
                                                    // know where i "am" now
        }
    }

    fn register_events(&self, text_view: &gtk::TextView, toast_parent: &adw::ToastOverlay) {
        let gesture_ctrl = gtk::GestureClick::new();
        let tv = text_view.clone();

        let action_group = gio::SimpleActionGroup::new();
        text_view.insert_action_group("note", Some(&action_group));

        let copy_password_action =
            gio::SimpleAction::new("copy-password", Some(&i32::static_variant_type()));
        let tv1 = self.imp().text_view.clone();
        let tp = toast_parent.clone();
        copy_password_action.connect_activate(move |_action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let password_index = parameter.unwrap().get::<i32>().unwrap() as usize;
            if let Some((_, note_metadata)) = &*tv1.borrow() {
                if let Some(p) = note_metadata.note_passwords.get(password_index) {
                    copy_to_clipboard(&p.data);
                    tp.add_toast(adw::Toast::new("Password copied to the clipboard"));
                }
            }
        });
        action_group.add_action(&copy_password_action);

        let reveal_password_action =
            gio::SimpleAction::new("reveal-password", Some(&i32::static_variant_type()));
        let tv2 = self.imp().text_view.clone();
        let tp = toast_parent.clone();
        reveal_password_action.connect_activate(move |action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let password_index = parameter.unwrap().get::<i32>().unwrap() as usize;
            if let Some((_, note_metadata)) = &*tv2.borrow() {
                if let Some(p) = note_metadata.note_passwords.get(password_index) {
                    tp.add_toast(adw::Toast::new(&format!("The password is: {}", p.data)));
                }
            }
        });
        action_group.add_action(&reveal_password_action);

        let tv3 = self.imp().text_view.clone();
        gesture_ctrl.connect_released(move |_gesture, _n, x, y| {
            if let Some((tv_widget, note_metadata)) = &*tv3.borrow() {
                let (bx, by) = tv_widget.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                if let Some(iter) = tv_widget.iter_at_location(bx, by) {
                    let offset = iter.offset();
                    if Self::iter_matches_tags(&iter, &[notes::TAG_LINK, notes::TAG_PASSWORD]) {
                        if let Some(link) = note_metadata
                            .note_links
                            .iter()
                            .find(|l| l.start_offset <= offset && l.end_offset > offset)
                        {
                            gtk::UriLauncher::new(&link.data).launch(
                                None::<&gtk::Window>,
                                None::<&gio::Cancellable>,
                                |_| {},
                            );
                        } else if let Some(pass_idx) = note_metadata
                            .note_passwords
                            .iter()
                            .position(|l| l.start_offset <= offset && l.end_offset > offset)
                        {
                            Self::password_popover(
                                &tv_widget,
                                pass_idx,
                                &tv_widget.iter_location(&iter),
                            );
                        }
                    }
                }
            }
        });
        text_view.add_controller(gesture_ctrl);

        let motion_controller = gtk::EventControllerMotion::new();
        let tv3 = tv.clone();
        motion_controller.connect_motion(move |_, x, y| {
            let (bx, by) =
                tv3.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
            if let Some(iter) = tv3.iter_at_location(bx, by) {
                if Self::iter_is_link_or_password(&iter) {
                    tv3.set_cursor_from_name(Some("pointer"));
                // } else if let Some(iter) = self.widgets.note_textview.iter_at_location(bx, by) {
                //     let is_code = Self::iter_matches_tags(&iter, &[crate::notes::TAG_CODE]);
                //     if is_code {
                //         self.textview_move_cursor_over_code(iter);
                //     }
                } else {
                    tv3.set_cursor(None);
                }
            } else {
                tv3.set_cursor(None);
            }
        });
        tv.add_controller(motion_controller);
    }

    fn iter_is_link_or_password(iter: &gtk::TextIter) -> bool {
        Self::iter_matches_tags(iter, &[crate::notes::TAG_LINK, crate::notes::TAG_PASSWORD])
    }

    fn password_popover(text_view: &gtk::TextView, pass_idx: usize, position: &gdk::Rectangle) {
        // i'd initialize the popover in the init & reuse it,
        // but i can't get the toplevel there, probably things
        // are not fully initialized yet.
        let popover = gtk::PopoverMenu::builder().pointing_to(position).build();

        popover.set_parent(text_view);
        popover.set_position(gtk::PositionType::Bottom);

        let menu_model = gio::Menu::new();
        menu_model.append(
            Some("Copy password"),
            Some(&format!("note.copy-password({})", pass_idx)),
        );
        menu_model.append(
            Some("Reveal password"),
            Some(&format!("note.reveal-password({})", pass_idx)),
        );
        popover.set_menu_model(Some(&menu_model));
        popover.popup();
    }

    fn iter_matches_tags(iter: &gtk::TextIter, tags: &[&str]) -> bool {
        iter.tags().iter().any(|t| {
            if let Some(prop_name) = t.name() {
                let prop_name_str = prop_name.as_str();
                tags.contains(&prop_name_str)
            } else {
                false
            }
        })
    }
}
