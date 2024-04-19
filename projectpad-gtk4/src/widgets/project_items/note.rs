use diesel::prelude::*;
use std::{collections::HashSet, sync::mpsc};

use adw::prelude::*;
use glib::property::PropertySet;
use glib::*;
use gtk::{gdk, subclass::prelude::*};
use projectpadsql::models::{EnvironmentType, ProjectNote, ServerNote};

use crate::{
    app::ProjectpadApplication,
    notes,
    sql_thread::SqlFunc,
    widgets::{project_item::WidgetMode, project_items::common::copy_to_clipboard},
};

use super::common::{self, EnvOrEnvs};

struct NoteInfo<'a> {
    title: &'a str,
    env: EnvOrEnvs,
    contents: &'a str,
    display_header: bool,
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

        pub note_links: RefCell<Vec<ItemDataInfo>>,
        pub note_passwords: Rc<RefCell<Vec<ItemDataInfo>>>,
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
                sender.send_blocking(note).unwrap();
            }))
            .unwrap();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let channel_data = receiver.recv().await.unwrap();
            p.display_note_contents(NoteInfo {
                title: &channel_data.title,
                env: EnvOrEnvs::Envs(Self::get_envs(&channel_data)),
                contents: &channel_data.contents,
                display_header: true,
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
                title: &channel_data.title,
                env: EnvOrEnvs::None,
                contents: &channel_data.contents,
                display_header: false,
            });
        });
    }

    fn get_db_sender() -> mpsc::Sender<SqlFunc> {
        let app = gio::Application::default().and_downcast::<ProjectpadApplication>();
        app.unwrap().get_sql_channel()
    }

    fn display_note_contents(&self, note: NoteInfo) {
        let widget_mode = if self.edit_mode() {
            WidgetMode::Edit
        } else {
            WidgetMode::Show
        };
        let vbox = if note.display_header {
            common::get_contents_box_with_header(&note.title, None, note.env, widget_mode)
        } else {
            gtk::Box::builder().build()
        };
        self.set_child(Some(&vbox));

        let (note_view, _scrolled_window) =
            self.get_note_contents_widget(&note.contents, widget_mode);

        vbox.append(&note_view);
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
            self.register_events(&text_view, &toast_parent);
            self.imp().note_links.set(note_buffer_info.links);
            self.imp().note_passwords.set(note_buffer_info.passwords);
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
            view.upcast::<gtk::Widget>()
        };

        let scrolled_text_view = gtk::ScrolledWindow::builder()
            .child(&text_view)
            .vexpand(true)
            .hexpand(true)
            .build();

        toast_parent.set_child(Some(&scrolled_text_view));

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
        (widget, scrolled_text_view)
    }

    fn register_events(&self, text_view: &gtk::TextView, toast_parent: &adw::ToastOverlay) {
        let gesture_ctrl = gtk::GestureClick::new();
        let tv = text_view.clone();
        let s = self.clone();

        let action_group = gio::SimpleActionGroup::new();
        text_view.insert_action_group("note", Some(&action_group));

        let copy_password_action =
            gio::SimpleAction::new("copy-password", Some(&i32::static_variant_type()));
        let sp = self.imp().note_passwords.clone();
        let tp = toast_parent.clone();
        copy_password_action.connect_activate(move |action, parameter| {
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
        let sp = self.imp().note_passwords.clone();
        let tp = toast_parent.clone();
        reveal_password_action.connect_activate(move |action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let password_index = parameter.unwrap().get::<i32>().unwrap() as usize;
            if let Some(p) = sp.borrow().get(password_index) {
                tp.add_toast(adw::Toast::new(&format!("The password is: {}", p.data)));
            }
        });
        action_group.add_action(&reveal_password_action);

        gesture_ctrl.connect_released(move |_gesture, _n, x, y| {
            let (bx, by) =
                tv.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
            if let Some(iter) = tv.iter_at_location(bx, by) {
                let offset = iter.offset();
                if Self::iter_matches_tags(&iter, &[notes::TAG_LINK, notes::TAG_PASSWORD]) {
                    if let Some(link) = s
                        .imp()
                        .note_links
                        .borrow()
                        .iter()
                        .find(|l| l.start_offset <= offset && l.end_offset > offset)
                    {
                        gtk::UriLauncher::new(&link.data).launch(
                            None::<&gtk::Window>,
                            None::<&gio::Cancellable>,
                            |_| {},
                        );
                    } else if let Some(pass_idx) = s
                        .imp()
                        .note_passwords
                        .borrow()
                        .iter()
                        .position(|l| l.start_offset <= offset && l.end_offset > offset)
                    {
                        s.password_popover(&tv, pass_idx, &tv.iter_location(&iter));
                    }
                }
            }
        });
        text_view.add_controller(gesture_ctrl);
    }

    fn password_popover(
        &self,
        text_view: &gtk::TextView,
        pass_idx: usize,
        position: &gdk::Rectangle,
    ) {
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
