use app::ProjectpadApplication;
use glib::prelude::*;
use glib::Properties;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Builder, Button, MessageDialog, ResponseType};
use widgets::project_list::ProjectList;
mod widgets;
use adw::subclass::prelude::*;
use glib::{clone, subclass, Sender};
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod app;

#[derive(Default)]
pub struct Project {
    name: String,
}

#[derive(Default)]
pub struct ProjectItem {
    name: String,
}

mod imp {
    use super::*;
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/gtk_builder.ui")]
    pub struct ProjectpadApplicationWindow {
        #[template_child]
        pub project_list: TemplateChild<ProjectList>,
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

            self.project_list.get().set_project_items(Vec::new());
            // let app = ProjectpadApplication::default();
            // let sender = app.imp().sender.clone();
            // let player = app.imp().player.clone();

            // self.obj().setup_widgets(sender.clone(), player);
            // self.obj().setup_gactions(sender);
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
        glib::Object::new::<Self>()
    }
}

// impl Default for ProjectpadApplicationWindow {
//     fn default() -> Self {
//         SwApplication::default()
//             .active_window()
//             .unwrap()
//             .downcast()
//             .unwrap()
//     }
// }

fn main() -> glib::ExitCode {
    let res_bytes = include_bytes!("resources.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    ProjectpadApplication::run()
    // let application = gtk::Application::new(
    //     Some("com.github.gtk-rs.examples.builder_basics"),
    //     Default::default(),
    // );
    // application.connect_activate(build_ui);
    // application.run()
}

fn build_ui(application: &Application) {
    // https://github.com/gtk-rs/gtk4-rs/issues/116
    // must call before using in UI files
    widgets::project_item_row::ProjectItemRow::static_type();
    ProjectList::static_type();

    // let ui_src = include_str!("gtk_builder.ui");
    // let builder = Builder::from_string(ui_src);

    // // let window: ApplicationWindow = builder.object("window").expect("Couldn't get window");
    // window.set_application(Some(application));
    // // let bigbutton: Button = builder.object("button").expect("Couldn't get button");
    // let dialog: MessageDialog = builder
    //     .object("messagedialog")
    //     .expect("Couldn't get messagedialog");

    // // probably look at SwApplicationWindow in shortwave
    // dialog.connect_response(move |d: &MessageDialog, _: ResponseType| {
    //     d.hide();
    // });

    // // bigbutton.connect_clicked(move |_| {
    // //     dialog.show();
    // // });

    // window.show();
}
