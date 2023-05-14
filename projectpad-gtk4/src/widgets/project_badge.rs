use std::time::Duration;

use gtk::prelude::*;
use projectpadsql::models::Project;
use relm4::{
    drawing::DrawHandler,
    factory::FactoryView,
    gtk::{self, cairo::Operator},
    prelude::{DynamicIndex, FactoryComponent},
    FactorySender,
};

// factory.rs example

use super::project_list::ProjectListMsg;

#[derive(Debug)]
pub struct ProjectBadge {
    project: Project,
    // font_size_for_width: Rc<RefCell<Option<(i32, f64)>>>, // cache the computed font size
    // backing_buffer: Rc<RefCell<Option<gtk::cairo::ImageSurface>>>,
    is_active: bool,
    handler: DrawHandler,
}

#[relm4::factory(pub)]
impl FactoryComponent for ProjectBadge {
    type Init = (Project, bool);
    type Input = bool;
    type Output = Project;
    type CommandOutput = ();
    type ParentInput = ProjectListMsg;
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            #[local_ref]
            area -> gtk::DrawingArea {
                set_hexpand: true,
                set_vexpand: true,

                connect_resize[sender] => move |_, x, y| {
                    sender.input(true);
                }
            }
        }
    }

    fn init_model(value: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        // sender.input(value.1);

        Self {
            is_active: value.1,
            project: value.0,
            handler: DrawHandler::new(),
        }
    }

    fn init_widgets(
        &mut self,
        index: &DynamicIndex,
        root: &Self::Root,
        returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let area = self.handler.drawing_area();
        let widgets = view_output!();

        // sender.command(|out, shutdown| {
        //     shutdown
        //         .register(async move {
        //             loop {
        //                 tokio::time::sleep(Duration::from_millis(20)).await;
        //                 out.send(true).unwrap();
        //             }
        //         })
        //         .drop_on_shutdown()
        // });

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        let cx = self.handler.get_context();

        dbg!(&self.project.name);

        cx.set_operator(Operator::Clear);
        cx.set_source_rgba(1.0, 0.0, 0.0, 0.0);
        cx.paint().expect("Couldn't fill context");

        cx.set_source_rgb(0.0, 0.0, 0.0);
        cx.arc(5.0, 5.0, 3.0, 0.0, std::f64::consts::PI * 2.0);
        cx.fill().expect("error filling text");
        cx.text_path(&self.project.name);
        cx.fill().expect("error filling text");

        self.is_active = msg;
        // CounterMsg::Increment => {
        //     self.value = self.value.wrapping_add(1);
        // }
        // CounterMsg::Decrement => {
        //     self.value = self.value.wrapping_sub(1);
        // }
    }
}
