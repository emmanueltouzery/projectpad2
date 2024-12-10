use crate::{perform_insert_or_update, widgets::project_item_model::ProjectItemType};
use diesel::prelude::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::mpsc};

use adw::prelude::*;
use glib::property::PropertySet;
use glib::*;
use gtk::{gdk, subclass::prelude::*};
use projectpadsql::models::{EnvironmentType, ProjectNote, ServerNote};

use crate::{
    app::ProjectpadApplication,
    notes::{self, ItemDataInfo},
    sql_thread::SqlFunc,
    widgets::{
        project_item::WidgetMode,
        project_items::common::{
            copy_to_clipboard, display_item_edit_dialog, get_project_group_names, DialogClamp,
        },
        search_bar::SearchBar,
    },
};

use super::common::{self, EnvOrEnvs};

#[derive(Clone)]
struct NoteInfo<'a> {
    id: i32,
    title: &'a str,
    env: EnvOrEnvs,
    contents: &'a str,
    display_header: bool,
    group_name: Option<&'a str>,
    all_group_names: &'a [String],
}

#[derive(Clone, Debug, Default)]
pub enum ViewOrTextView {
    View(sourceview5::View),
    TextView(gtk::TextView),
    #[default]
    None,
}

mod imp {
    use crate::notes::ItemDataInfo;

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

        pub text_view: Rc<RefCell<ViewOrTextView>>,

        pub note_links: Rc<RefCell<Vec<ItemDataInfo>>>,
        pub note_passwords: Rc<RefCell<Vec<ItemDataInfo>>>,
        pub header_iters: Rc<RefCell<Vec<gtk::TextIter>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Note {
        const NAME: &'static str = "Note";
        type ParentType = adw::Bin;
        type Type = super::Note;

        fn class_init(klass: &mut Self::Class) {
            // Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
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
                } else {
                    note.load_and_display_project_note(note.imp().project_note_id.get());
                }
            });
            let _ = self
                .obj()
                .connect_server_note_id_notify(|note: &super::Note| {
                    note.load_and_display_server_note(note.imp().server_note_id.get());
                });
            let _ = self
                .obj()
                .connect_project_note_id_notify(|note: &super::Note| {
                    note.load_and_display_project_note(note.imp().project_note_id.get());
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
            let (header_box, vbox) = self.note_contents(
                note.clone(),
                self.imp().text_view.clone(),
                self.imp().note_links.clone(),
                self.imp().note_passwords.clone(),
                self.imp().header_iters.clone(),
                WidgetMode::Show,
            );

            let toc_btn = gtk::MenuButton::builder()
                .icon_name("list-ol")
                .valign(gtk::Align::Center)
                .halign(gtk::Align::End)
                .popover(&Self::note_toc_menu(&note))
                .build();
            if widget_mode != WidgetMode::Edit {
                toc_btn.set_hexpand(true);
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

            let note_links = self.imp().note_links.clone();
            let note_passwords = self.imp().note_passwords.clone();
            let header_iters = self.imp().header_iters.clone();
            let t = note.title.to_owned();
            let c = note.contents.to_owned();
            let g = note.group_name.map(|g| g.to_owned());
            let a = note.all_group_names.to_vec();
            let tv_var = self.imp().text_view.clone();
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
                                         @strong vbox as v,
                                         @strong note_links as nl,
                                         @strong note_passwords as np,
                                         @strong header_iters as hi => move |_b: gtk::Button| {
                        let n = NoteInfo {
                            id: note.id,
                            title: &_t,
                            env: note.env.clone(),
                            contents: &_c,
                            display_header: note.display_header,
                            group_name: _g.as_deref(),
                            all_group_names: &_a
                        };
                        let (_, vbox) = s.note_contents(n.clone(), tv.clone(), nl.clone(), np.clone(), hi.clone(), WidgetMode::Edit);
                        vbox.set_margin_start(30);
                        vbox.set_margin_end(30);

                        let (dialog, save_btn) = display_item_edit_dialog(&v, "Edit Note", vbox, 6000, 6000, DialogClamp::No);
                        let ttv = tv.clone();
                        let project_note_id = note.id;
                        let s = _s.clone();
                        save_btn.connect_clicked(move |_| {
                            match &*ttv.borrow() {
                                ViewOrTextView::View(v) => {
                                    let buf = v.buffer();
                                    let start_iter = buf.start_iter();
                                    let end_iter = buf.end_iter();
                                    let new_contents = v.buffer().text(&start_iter, &end_iter, false);
                                    let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
                                    let (sender, receiver) = async_channel::bounded(1);
                                    let db_sender = app.unwrap().get_sql_channel();
                                    db_sender
                                        .send(SqlFunc::new(move |sql_conn| {
                                            use projectpadsql::schema::project_note::dsl as prj_note;
                                            let changeset = (
                                                // prj_note::title.eq(new_title.as_str()),
                                                // // never store Some("") for group, we want None then.
                                                // prj_note::group_name.eq(new_group
                                                //     .as_ref()
                                                //     .map(|s| s.as_str())
                                                //     .filter(|s| !s.is_empty())),
                                                    prj_note::contents.eq(new_contents.as_str()),
                                                    // prj_note::has_dev.eq(new_has_dev),
                                                    // prj_note::has_stage.eq(new_has_stg),
                                                    // prj_note::has_uat.eq(new_has_uat),
                                                    // prj_note::has_prod.eq(new_has_prod),
                                                    // prj_note::project_id.eq(project_id),
                                            );
                                            let project_note_after_result = perform_insert_or_update!(
                                                sql_conn,
                                                Some(project_note_id),
                                                prj_note::project_note,
                                                prj_note::id,
                                                changeset,
                                                ProjectNote,
                                            );
                                            sender.send_blocking(project_note_after_result).unwrap();
                                        })).unwrap();

                                    let d = dialog.clone();
                                    let s1 = s.clone();
                                    glib::spawn_future_local(async move {
                                        let project_note_after_result = receiver.recv().await.unwrap();
                                        d.close();
                                        s1.clone().load_and_display_project_note(project_note_id);
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
            let vbox = self
                .note_contents(
                    note,
                    self.imp().text_view.clone(),
                    self.imp().note_links.clone(),
                    self.imp().note_passwords.clone(),
                    self.imp().header_iters.clone(),
                    widget_mode,
                )
                .1;
            self.set_child(Some(&vbox));
        }
    }

    fn note_contents(
        &self,
        note: NoteInfo,
        text_view_field: Rc<RefCell<ViewOrTextView>>,
        note_links: Rc<RefCell<Vec<ItemDataInfo>>>,
        note_passwords: Rc<RefCell<Vec<ItemDataInfo>>>,
        header_iters: Rc<RefCell<Vec<gtk::TextIter>>>,
        widget_mode: WidgetMode,
    ) -> (gtk::Box, gtk::Box) {
        let (header_box, vbox) = if note.display_header {
            common::get_contents_box_with_header(
                &note.title,
                ProjectItemType::ProjectNote,
                note.group_name,
                note.all_group_names,
                note.env,
                widget_mode,
            )
        } else {
            (gtk::Box::builder().build(), gtk::Box::builder().build())
        };

        let (note_view, scrolled_window) = Self::get_note_contents_widget(
            text_view_field,
            note_links,
            note_passwords,
            header_iters,
            &note.contents,
            widget_mode,
        );

        let action_group = gio::SimpleActionGroup::new();
        let h_i = self.imp().header_iters.borrow().clone();
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

        vbox.append(&note_view);

        (header_box, vbox)
    }

    pub fn get_note_contents_widget(
        text_view_field: Rc<RefCell<ViewOrTextView>>,
        note_links: Rc<RefCell<Vec<ItemDataInfo>>>,
        note_passwords: Rc<RefCell<Vec<ItemDataInfo>>>,
        header_iters: Rc<RefCell<Vec<gtk::TextIter>>>,
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
            Self::register_events(
                note_links.clone(),
                note_passwords.clone(),
                &text_view,
                &toast_parent,
            );
            note_links.set(note_buffer_info.links);
            note_passwords.set(note_buffer_info.passwords);
            header_iters.set(note_buffer_info.header_iters);
            text_view_field.set(ViewOrTextView::TextView(text_view.clone()));
            text_view.upcast::<gtk::Widget>()
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
            text_view_field.set(ViewOrTextView::View(view.clone()));
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
        search_bar.connect_closure(
            "esc-pressed",
            false,
            glib::closure_local!(move |_: SearchBar| {
                r.set_reveal_child(false);
            }),
        );
        let tv = text_view_field.clone();
        search_bar.connect_closure(
            "search-changed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = &*tv.borrow();
                match cur_tv {
                    ViewOrTextView::View(v) => Self::apply_search(
                        v,
                        v.buffer().start_iter().forward_search(
                            &search,
                            gtk::TextSearchFlags::all(),
                            None,
                        ),
                    ),
                    ViewOrTextView::TextView(tv) => Self::apply_search(
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
        let tv2 = text_view_field.clone();
        search_bar.connect_closure(
            "prev-pressed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = tv2.borrow();
                match &*cur_tv {
                    ViewOrTextView::View(v) => Self::note_search_previous(v, Some(&search)),
                    ViewOrTextView::TextView(tv) => Self::note_search_previous(tv, Some(&search)),
                    _ => {}
                }
            }),
        );
        let tv3 = text_view_field.clone();
        search_bar.connect_closure(
            "next-pressed",
            false,
            glib::closure_local!(move |_: SearchBar, search: String| {
                let cur_tv = tv3.borrow();
                match &*cur_tv {
                    ViewOrTextView::View(v) => Self::note_search_next(v, Some(&search)),
                    ViewOrTextView::TextView(tv) => Self::note_search_next(tv, Some(&search)),
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
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
            if keyval.to_unicode() == Some('f') && state.contains(gdk::ModifierType::CONTROL_MASK) {
                revealer.set_reveal_child(true);
                search_bar.grab_focus();
                return glib::Propagation::Stop;
            }
            if keyval == gdk::Key::Escape {
                revealer.set_reveal_child(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed // Allow other handlers to process the event
        });
        widget.add_controller(key_controller);

        (widget, scrolled_text_view)
    }

    fn note_search_next<T>(textview: &T, note_search_text: Option<&str>)
    where
        T: TextViewExt,
    {
        let buffer = textview.buffer();
        if let (Some((_start, end)), Some(search)) =
            (buffer.selection_bounds(), note_search_text.clone())
        {
            Self::apply_search(
                textview,
                end.forward_search(&search, gtk::TextSearchFlags::all(), None),
            );
        }
    }

    fn note_search_previous<T>(textview: &T, note_search_text: Option<&str>)
    where
        T: TextViewExt,
    {
        let buffer = textview.buffer();
        if let (Some((start, _end)), Some(search)) =
            (buffer.selection_bounds(), note_search_text.clone())
        {
            Self::apply_search(
                textview,
                start.backward_search(&search, gtk::TextSearchFlags::all(), None),
            );
        }
    }

    fn apply_search<T>(textview: &T, range: Option<(gtk::TextIter, gtk::TextIter)>)
    where
        T: TextViewExt,
    {
        if let Some((mut start, end)) = range {
            textview.buffer().select_range(&start, &end);
            textview.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
        }
    }

    fn register_events(
        note_links: Rc<RefCell<Vec<ItemDataInfo>>>,
        note_passwords: Rc<RefCell<Vec<ItemDataInfo>>>,
        text_view: &gtk::TextView,
        toast_parent: &adw::ToastOverlay,
    ) {
        let gesture_ctrl = gtk::GestureClick::new();
        let tv = text_view.clone();

        let action_group = gio::SimpleActionGroup::new();
        text_view.insert_action_group("note", Some(&action_group));

        let copy_password_action =
            gio::SimpleAction::new("copy-password", Some(&i32::static_variant_type()));
        let sp = note_passwords.clone();
        let tp = toast_parent.clone();
        copy_password_action.connect_activate(move |_action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let password_index = parameter.unwrap().get::<i32>().unwrap() as usize;
            if let Some(p) = sp.borrow().get(password_index) {
                copy_to_clipboard(&p.data);
                tp.add_toast(adw::Toast::new("Password copied to the clipboard"));
            }
        });
        action_group.add_action(&copy_password_action);

        let reveal_password_action =
            gio::SimpleAction::new("reveal-password", Some(&i32::static_variant_type()));
        let sp = note_passwords.clone();
        let tp = toast_parent.clone();
        reveal_password_action.connect_activate(move |action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let password_index = parameter.unwrap().get::<i32>().unwrap() as usize;
            if let Some(p) = sp.borrow().get(password_index) {
                tp.add_toast(adw::Toast::new(&format!("The password is: {}", p.data)));
            }
        });
        action_group.add_action(&reveal_password_action);

        let tv2 = tv.clone();
        gesture_ctrl.connect_released(move |_gesture, _n, x, y| {
            let (bx, by) =
                tv2.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
            if let Some(iter) = tv2.iter_at_location(bx, by) {
                let offset = iter.offset();
                if Self::iter_matches_tags(&iter, &[notes::TAG_LINK, notes::TAG_PASSWORD]) {
                    if let Some(link) = note_links
                        .borrow()
                        .iter()
                        .find(|l| l.start_offset <= offset && l.end_offset > offset)
                    {
                        gtk::UriLauncher::new(&link.data).launch(
                            None::<&gtk::Window>,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    } else if let Some(pass_idx) = note_passwords
                        .borrow()
                        .iter()
                        .position(|l| l.start_offset <= offset && l.end_offset > offset)
                    {
                        Self::password_popover(&tv2, pass_idx, &tv2.iter_location(&iter));
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
