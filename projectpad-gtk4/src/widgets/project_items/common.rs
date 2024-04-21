use std::{collections::HashSet, rc::Rc};

use adw::prelude::*;
use diesel::prelude::*;
use gtk::gdk;
use projectpadsql::{models::EnvironmentType, schema};

use crate::{
    app::ProjectpadApplication,
    widgets::{
        environment_list_picker::EnvironmentListPicker, environment_picker::EnvironmentPicker,
        project_item::WidgetMode,
    },
};

#[derive(Clone)]
pub enum EnvOrEnvs {
    Env(EnvironmentType),
    Envs(HashSet<EnvironmentType>),
    None,
}

#[derive(PartialEq, Eq)]
pub enum DialogClamp {
    Yes,
    No,
}

pub fn display_item_edit_dialog(
    v: &gtk::Box,
    title: &str,
    item_box: gtk::Box,
    width: i32,
    height: i32,
    clamp: DialogClamp,
) {
    let cbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let header_bar = adw::HeaderBar::builder().build();
    cbox.append(&header_bar);
    let contents = if clamp == DialogClamp::Yes {
        adw::Clamp::builder()
            .margin_top(10)
            .child(&item_box)
            .build()
            .upcast::<gtk::Widget>()
    } else {
        item_box.upcast::<gtk::Widget>()
    };
    cbox.append(&contents);
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(width)
        .content_height(height)
        .child(&cbox)
        .build();
    dialog.present(v);
}

pub fn get_project_group_names(
    sql_conn: &mut diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use schema::project_note::dsl as pnote;
    use schema::project_point_of_interest::dsl as ppoi;
    use schema::server::dsl as srv;
    let server_group_names = srv::server
        .filter(
            srv::project_id
                .eq(project_id)
                .and(srv::group_name.is_not_null()),
        )
        .order(srv::group_name.asc())
        .select(srv::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_poi_group_names = ppoi::project_point_of_interest
        .filter(
            ppoi::project_id
                .eq(project_id)
                .and(ppoi::group_name.is_not_null()),
        )
        .order(ppoi::group_name.asc())
        .select(ppoi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_note_group_names = pnote::project_note
        .filter(
            pnote::project_id
                .eq(project_id)
                .and(pnote::group_name.is_not_null()),
        )
        .order(pnote::group_name.asc())
        .select(pnote::group_name)
        .load(sql_conn)
        .unwrap();

    let mut project_group_names = server_group_names;
    project_group_names.append(&mut prj_poi_group_names);
    project_group_names.append(&mut prj_note_group_names);
    let mut project_group_names_no_options: Vec<_> = project_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    project_group_names_no_options.sort();
    project_group_names_no_options.dedup();
    project_group_names_no_options
}

/// for the group names, i could require just the project id,
/// but the problem are notes, for which we share the code
/// between project notes and server notes (and these are
/// different groups...)
pub fn get_contents_box_with_header(
    title: &str,
    group_name: Option<&str>,
    all_group_names: &[String],
    env: EnvOrEnvs,
    widget_mode: WidgetMode,
) -> (gtk::Box, gtk::Box) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    let header_box = gtk::Box::builder().spacing(10).build();

    let server_icon = gtk::Image::builder()
        .icon_name("server")
        .pixel_size(48)
        .build();
    header_box.append(&server_icon);

    let header_second_col = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .hexpand(true)
        .build();

    if widget_mode == WidgetMode::Edit {
        let server = gtk::Entry::builder()
            .text(title)
            .halign(gtk::Align::Fill)
            .hexpand(true)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        header_second_col.append(&server);
    } else {
        let server = gtk::Label::builder()
            .label(title)
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        header_second_col.append(&server);
    }

    header_box.append(&header_second_col);

    if widget_mode == WidgetMode::Edit {
        let environment_picker = match env {
            EnvOrEnvs::Env(e) => Some(EnvironmentPicker::new(e).upcast::<gtk::Widget>()),
            EnvOrEnvs::Envs(es) => Some(EnvironmentListPicker::new(es).upcast::<gtk::Widget>()),
            EnvOrEnvs::None => None,
        };
        if let Some(ep) = environment_picker {
            ep.set_halign(gtk::Align::End);
            ep.set_hexpand(true);
            header_box.append(&ep);
        }
    }

    vbox.append(&header_box);

    if widget_mode == WidgetMode::Edit {
        // ability to change the item's group
        let mut group_name_items = vec!["No group", "New group..."];
        group_name_items.extend(all_group_names.iter().map(String::as_str));
        let hbox = gtk::Box::builder().spacing(10).build();
        hbox.append(&gtk::Label::builder().label("Group").build());
        hbox.append(&gtk::DropDown::from_strings(&group_name_items));
        vbox.append(&hbox);
    }

    (header_box, vbox)
}

pub fn copy_to_clipboard(text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);

        let toast_overlay = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<ProjectpadApplication>()
            .unwrap()
            .get_toast_overlay();
        toast_overlay.add_toast(adw::Toast::new("Copied to the clipboard"));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PasswordMode {
    PlainText,
    Password,
}

pub struct DetailsRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub password_mode: PasswordMode,
    pub main_action: Option<SuffixAction>,
    pub suffix_actions: &'a [SuffixAction],
}

pub struct SuffixAction {
    pub icon: &'static str,
    pub action: Rc<Box<dyn Fn() -> ()>>,
}

impl SuffixAction {
    pub fn copy(txt: &str) -> Option<SuffixAction> {
        let t = txt.to_owned();
        Some(SuffixAction {
            icon: "edit-copy-symbolic",
            action: Rc::new(Box::new(move || copy_to_clipboard(&t))),
        })
    }

    pub fn link(url: &str) -> SuffixAction {
        let u = url.to_owned();
        SuffixAction {
            icon: "external-link-alt-symbolic",
            action: Rc::new(Box::new(move || {
                gtk::UriLauncher::new(&u).launch(
                    None::<&gtk::Window>,
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            })),
        }
    }
}

impl DetailsRow<'_> {
    pub fn new<'a>(
        title: &'a str,
        subtitle: &'a str,
        main_action: Option<SuffixAction>,
        suffix_actions: &'a [SuffixAction],
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::PlainText,
            main_action,
            suffix_actions,
        }
    }

    pub fn new_password<'a>(
        title: &'a str,
        subtitle: &'a str,
        main_action: Option<SuffixAction>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::Password,
            main_action,
            suffix_actions: &[],
        }
    }

    pub fn add(&self, widget_mode: WidgetMode, group: &adw::PreferencesGroup) {
        match widget_mode {
            WidgetMode::Show => self.add_show(group),
            WidgetMode::Edit => self.add_edit(group),
        }
    }

    fn add_show(&self, group: &adw::PreferencesGroup) {
        if !self.subtitle.is_empty() {
            let subtitle = if self.password_mode == PasswordMode::PlainText {
                self.subtitle
            } else {
                "●●●●"
            };
            let e = adw::ActionRow::builder()
                .title(glib::markup_escape_text(self.title))
                .subtitle(glib::markup_escape_text(subtitle))
                // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
                // When used together with the .property style class, AdwActionRow and
                // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
                .css_classes(["property"])
                .build();
            for suffix in self.suffix_actions.iter() {
                let widget = gtk::Button::builder()
                    .css_classes(["flat"])
                    .icon_name(suffix.icon)
                    .build();
                let a = suffix.action.clone();
                widget.connect_closure(
                    "clicked",
                    false,
                    glib::closure_local!(|_b: gtk::Button| {
                        a();
                    }),
                );
                e.add_suffix(&widget);
            }
            if self.password_mode == PasswordMode::Password {
                let widget = gtk::Button::builder()
                    .css_classes(["flat"])
                    .icon_name("view-reveal-symbolic")
                    .build();
                let ar = e.clone();
                let st = self.subtitle.to_owned();
                widget.connect_closure(
                    "clicked",
                    false,
                    glib::closure_local!(|b: gtk::Button| {
                        if b.icon_name() == Some("view-reveal-symbolic".into()) {
                            ar.set_subtitle(&st);
                            b.set_icon_name("view-conceal-symbolic");
                        } else {
                            ar.set_subtitle("●●●●");
                            b.set_icon_name("view-reveal-symbolic");
                        }
                    }),
                );
                e.add_suffix(&widget);
            }
            if let Some(SuffixAction { icon, action }) = self.main_action.as_ref() {
                e.add_suffix(&gtk::Image::builder().icon_name(*icon).build());
                e.set_activatable(true);
                let c_a = action.clone();
                e.connect_activated(move |_ar| {
                    c_a();
                });
            }
            group.add(&e);
        }
    }

    fn add_edit(&self, group: &adw::PreferencesGroup) {
        match self.password_mode {
            PasswordMode::PlainText => {
                let e = adw::EntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                // if let Some(i) = self.suffix_icon {
                //     e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                // }
                group.add(&e);
            }
            PasswordMode::Password => {
                let e = adw::PasswordEntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                // if let Some(i) = self.suffix_icon {
                //     e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                // }
                group.add(&e);
            }
        }
    }
}
