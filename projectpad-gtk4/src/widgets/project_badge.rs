use projectpadsql::models::Project;
use relm4::{
    gtk,
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
        root = gtk::DrawingArea {}
    }

    fn init_model(value: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            is_active: value.1,
            project: value.0,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        self.is_active = msg;
        // CounterMsg::Increment => {
        //     self.value = self.value.wrapping_add(1);
        // }
        // CounterMsg::Decrement => {
        //     self.value = self.value.wrapping_sub(1);
        // }
    }
}
