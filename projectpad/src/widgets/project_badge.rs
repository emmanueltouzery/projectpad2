use gtk::prelude::*;
use gtk::DrawingArea;
use projectpadsql::models::Project;
use relm::{DrawHandler, Widget};
use relm_derive::{widget, Msg};
use std::f64::consts::PI;

#[derive(Msg)]
pub enum Msg {
    UpdateDrawBuffer,
    Click,
    Activate(Project),
    ActiveProjectChanged(Project),
}

pub struct Model {
    relm: relm::Relm<ProjectBadge>,
    project: Project,
    draw_handler: DrawHandler<DrawingArea>,
    font_size_for_width: Option<(i32, f64)>, // cache the computed font size
    is_active: bool,
}

#[widget]
impl Widget for ProjectBadge {
    fn init_view(&mut self) {
        println!("badge init_view called");
        self.drawing_area.set_size_request(60, 60);
        self.model.draw_handler.init(&self.drawing_area);
        self.drawing_area
            .add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    }

    fn model(relm: &relm::Relm<Self>, project: Project) -> Model {
        Model {
            relm: relm.clone(),
            project,
            draw_handler: DrawHandler::new().expect("draw handler"),
            font_size_for_width: None,
            is_active: false,
        }
    }

    fn compute_font_size(context: &cairo::Context, width: f64) -> f64 {
        let mut size = 5.0;
        context.set_font_size(size);
        while context.text_extents("HU").width < width {
            context.set_font_size(size);
            size += 1.0;
        }
        size
    }

    fn draw_icon(context: &cairo::Context, allocation_width: i32, icon: &[u8]) {
        match cairo::ImageSurface::create_from_png(&mut icon.clone()).ok() {
            Some(surface) => {
                let w = surface.get_width() as f64;
                let h = surface.get_height() as f64;
                let scale_ratio = f64::min(60.0 / w, 60.0 / h);
                context.scale(scale_ratio, scale_ratio);
                let (offsetx, offsety) = if w > h {
                    (0.0, (60.0 - 60.0 * h / w) / 2.0)
                } else {
                    ((60.0 - 60.0 * w / h) / 2.0, 0.0)
                };
                if w > h {
                    println!(
                        "w {} h {} effective height: {} offsety {}",
                        w,
                        h,
                        60.0 * h / w,
                        offsety
                    );
                }
                context.set_source_surface(&surface, offsetx / scale_ratio, offsety / scale_ratio);
                context.paint();
            }
            _ => {
                eprintln!("failed reading png {}", icon.len());
            }
        }
    }

    fn draw_label(context: &cairo::Context, allocation_width: i32, contents: &str) {
        // context.set_source_rgb(1.0, 1.0, 1.0);
        let text_extents = context.text_extents(contents);
        context.move_to(
            (allocation_width / 2) as f64 - text_extents.width / 2.0 - text_extents.x_bearing,
            (allocation_width / 2) as f64 - text_extents.y_bearing - text_extents.height / 2.0,
        );
        context.text_path(contents);
        context.fill();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::UpdateDrawBuffer => {
                let context = self.model.draw_handler.get_context();
                let allocation = self.drawing_area.get_allocation();
                // println!("drawing badge, allocation: {:?}", allocation);
                match self.model.font_size_for_width {
                    Some((w, font_size)) if w == allocation.width => {
                        context.set_font_size(font_size)
                    }
                    _ => {
                        self.model.font_size_for_width = Some((
                            allocation.width,
                            Self::compute_font_size(&context, allocation.width as f64 * 0.75),
                        ));
                    }
                }
                context.set_antialias(cairo::Antialias::Best);

                context.set_source_rgb(1.0, 1.0, 1.0);
                context.rectangle(0.0, 0.0, allocation.width.into(), allocation.height.into());
                context.fill();

                if self.model.is_active {
                    context.set_source_rgb(0.5, 0.5, 0.5);
                } else {
                    context.set_source_rgb(0.0, 0.0, 0.0);
                }
                context.arc(
                    (allocation.width / 2).into(),
                    (allocation.width / 2).into(),
                    (allocation.width / 2).into(),
                    0.0,
                    2.0 * PI,
                );
                context.stroke();

                // https://developer.gnome.org/gtkmm-tutorial/stable/sec-draw-images.html.en
                match &self.model.project.icon {
                    // the 'if' works around an issue reading from SQL. should be None if it's empty!!
                    Some(icon) if icon.len() > 0 => {
                        Self::draw_icon(&context, allocation.width, &icon)
                    }
                    _ => {
                        Self::draw_label(&context, allocation.width, &self.model.project.name[..2])
                    }
                }
            }
            Msg::Click => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::Activate(self.model.project.clone()));
            }
            Msg::Activate(_) => {
                // meant for my parent, not me
            }
            Msg::ActiveProjectChanged(p) => {
                self.model.is_active = p == self.model.project;
            }
        }
    }

    view! {
        #[name="drawing_area"]
        gtk::DrawingArea {
            draw(_, _) => (Msg::UpdateDrawBuffer, Inhibit(false)),
            button_press_event(_, _) => (Msg::Click, Inhibit(false)),
        }
    }
}
