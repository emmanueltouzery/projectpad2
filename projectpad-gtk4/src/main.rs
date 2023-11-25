use app::ProjectpadApplication;
use gtk::glib;
use widgets::project_item_list::ProjectItemList;
mod widgets;
use adw::subclass::prelude::*;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod app;

mod imp {
    use crate::widgets::project_item::ProjectItem;

    use super::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/gtk_builder.ui")]
    pub struct ProjectpadApplicationWindow {
        #[template_child]
        pub project_avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub project_avatar_popover: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub project_item_list: TemplateChild<ProjectItemList>,
        #[template_child]
        pub project_item: TemplateChild<ProjectItem>,
        #[template_child]
        pub edit_btn: TemplateChild<gtk::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectpadApplicationWindow {
        const NAME: &'static str = "ProjectpadApplicationWindow";
        type ParentType = adw::ApplicationWindow;
        type Type = super::ProjectpadApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectpadApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.project_item_list.get().set_project_items();
            // let app = ProjectpadApplication::default();
            // let sender = app.imp().sender.clone();
            // let player = app.imp().player.clone();

            // self.obj().setup_widgets(sender.clone(), player);
            // self.obj().setup_gactions(sender);
            self.edit_btn
                .bind_property("active", &self.project_item.get(), "edit_mode")
                .build();

            let gesture = gtk::GestureClick::new();
            let project_avatar_popover = self.project_avatar_popover.get();
            gesture.connect_released(move |gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                println!("Box pressed!");
                // https://discourse.gnome.org/t/using-gtkpopovermenu-as-a-gtkmenu-replacement/3786/14
                let popover_menu = gio::Menu::new();
                popover_menu.append(Some("Add project..."), None);
                // let popover = gtk::PopoverMenu::builder()
                //     .menu_model(&popover_menu)
                //     .build();
                project_avatar_popover.set_menu_model(Some(&popover_menu));
                project_avatar_popover.popup();
            });
            self.project_avatar.get().add_controller(gesture);
            // https://www.reddit.com/r/GTK/comments/15y17a0/how_to_add_a_right_click_menu_to_a_drawingarea/
        }
    }

    impl WidgetImpl for ProjectpadApplicationWindow {}

    impl WindowImpl for ProjectpadApplicationWindow {
        // fn close_request(&self) -> glib::Propagation {
        // debug!("Saving window geometry.");
        // let width = self.obj().default_size().0;
        // let height = self.obj().default_size().1;

        // settings_manager::set_integer(Key::WindowWidth, width);
        // settings_manager::set_integer(Key::WindowHeight, height);
        // glib::Propagation::Proceed
        // }
    }

    impl ApplicationWindowImpl for ProjectpadApplicationWindow {}

    impl AdwApplicationWindowImpl for ProjectpadApplicationWindow {}

    impl ProjectpadApplicationWindow {}
}

glib::wrapper! {
    pub struct ProjectpadApplicationWindow(
        ObjectSubclass<imp::ProjectpadApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow;
        // @implements gio::ActionMap, gio::ActionGroup;
}

impl ProjectpadApplicationWindow {
    pub fn new() -> Self {
        let win = glib::Object::new::<Self>();
        win.imp().project_item_list.connect_activate(
            glib::clone!(@weak win as w => move |project_item_id| {
                w.imp().project_item.set_item_id(project_item_id)
            }),
        );
        win
    }
}

fn main() -> glib::ExitCode {
    let res_bytes = include_bytes!("resources.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    ProjectpadApplication::run()
}
