use super::win::Project;
use gtk::prelude::*;
use gtk::DrawingArea;
use relm::{DrawHandler, Widget};
use relm_derive::{widget, Msg};
use std::f64::consts::PI;

#[derive(Msg)]
pub enum Msg {
    UpdateDrawBuffer,
}

pub struct Model {
    project: Project,
    draw_handler: DrawHandler<DrawingArea>,
}

#[widget]
impl Widget for ProjectBadge {
    fn init_view(&mut self) {
        self.model.draw_handler.init(&self.drawing_area);
    }

    fn model(relm: &relm::Relm<Self>, project: Project) -> Model {
        Model {
            project,
            draw_handler: DrawHandler::new().expect("draw handler"),
        }
    }

    fn compute_font_size(context: &cairo::Context, width: f64) {
        let mut size = 5.0;
        context.set_font_size(size);
        while context.text_extents("HU").width < width {
            println!("Trying font size {}", size);
            context.set_font_size(size);
            size += 1.0;
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::UpdateDrawBuffer => {
                let context = self.model.draw_handler.get_context();
                let allocation = self.drawing_area.get_allocation();
                Self::compute_font_size(&context, allocation.width as f64 * 0.75); // TODO not for every drawing!!
                context.set_antialias(cairo::Antialias::Best);

                context.set_source_rgb(1.0, 1.0, 1.0);
                context.rectangle(0.0, 0.0, allocation.width.into(), allocation.height.into());
                context.fill();

                context.set_source_rgb(0.0, 0.0, 0.0);
                context.arc(
                    (allocation.width / 2).into(),
                    (allocation.width / 2).into(),
                    (allocation.width / 2).into(),
                    0.0,
                    2.0 * PI,
                );
                context.fill();

                context.set_source_rgb(1.0, 1.0, 1.0);
                let text_extents = context.text_extents("HU");
                context.move_to(
                    (allocation.width / 2) as f64
                        - text_extents.width / 2.0
                        - text_extents.x_bearing,
                    (allocation.height / 2) as f64
                        - text_extents.y_bearing
                        - text_extents.height / 2.0,
                );
                context.text_path("HU");
                context.fill();
            }
        }
    }

    view! {
        #[name="drawing_area"]
        gtk::DrawingArea {
            draw(_, _) => (Msg::UpdateDrawBuffer, Inhibit(false)),
        }
    }
}
