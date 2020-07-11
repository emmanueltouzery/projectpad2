use gtk::prelude::*;
use gtk::DrawingArea;
use projectpadsql::models::Project;
use relm::{DrawHandler, Widget};
use relm_derive::{widget, Msg};
use std::f64::consts::PI;

const OUTER_SIZE: i32 = 60;
const PADDING: i32 = 5;

#[derive(Msg)]
pub enum Msg {
    UpdateDrawBuffer,
    Click,
    Activate(Project),
    ActiveProjectChanged(i32),
}

pub struct Model {
    relm: relm::Relm<ProjectBadge>,
    project: Project,
    draw_handler: DrawHandler<DrawingArea>,
    font_size_for_width: Option<(i32, f64)>, // cache the computed font size
    backing_buffer: Option<cairo::ImageSurface>,
    is_active: bool,
}

#[widget]
impl Widget for ProjectBadge {
    fn init_view(&mut self) {
        println!("badge init_view called");
        self.drawing_area.set_size_request(OUTER_SIZE, OUTER_SIZE);
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
            backing_buffer: None,
            is_active: false,
        }
    }

    fn prepare_backing_buffer(
        &mut self,
        allocation_width: i32,
        allocation_height: i32,
    ) -> cairo::ImageSurface {
        let buf =
            cairo::ImageSurface::create(cairo::Format::ARgb32, allocation_width, allocation_height)
                .expect("cairo backing buffer");
        let context = cairo::Context::new(&buf);

        // println!("drawing badge, allocation: {:?}", allocation);
        match self.model.font_size_for_width {
            Some((w, font_size)) if w == allocation_width => context.set_font_size(font_size),
            _ => {
                self.model.font_size_for_width = Some((
                    allocation_width,
                    Self::compute_font_size(
                        &context,
                        (allocation_width - PADDING * 2) as f64 * 0.75,
                    ),
                ));
            }
        }
        context.set_antialias(cairo::Antialias::Best);

        context.set_source_rgb(1.0, 1.0, 1.0);
        context.rectangle(0.0, 0.0, allocation_width.into(), allocation_height.into());
        context.fill();

        context.set_source_rgb(0.0, 0.0, 0.0);

        if self.model.is_active {
            // TODO maybe a rounded rect?  https://www.cairographics.org/samples/rounded_rectangle/
            context.rectangle(
                PADDING as f64 / 2.0,
                PADDING as f64 / 2.0,
                allocation_width as f64 - PADDING as f64 / 2.0,
                allocation_height as f64 - PADDING as f64 / 2.0,
            );
            context.fill();
        }

        context.arc(
            (allocation_width / 2).into(),
            (allocation_width / 2).into(),
            (allocation_width / 2 - PADDING).into(),
            0.0,
            2.0 * PI,
        );
        context.stroke_preserve();
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.fill();
        context.set_source_rgb(0.0, 0.0, 0.0);

        match &self.model.project.icon {
            // the 'if' works around an issue reading from SQL. should be None if it's empty!!
            Some(icon) if icon.len() > 0 => Self::draw_icon(&context, allocation_width, &icon),
            _ => Self::draw_label(&context, allocation_width, &self.model.project.name[..2]),
        }
        buf
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

    pub fn draw_icon(context: &cairo::Context, allocation_width: i32, icon: &[u8]) {
        context.save();
        match cairo::ImageSurface::create_from_png(&mut icon.clone()).ok() {
            Some(surface) => {
                let p = PADDING as f64;
                let aw = (allocation_width - PADDING * 2) as f64;
                let w = surface.get_width() as f64;
                let h = surface.get_height() as f64;
                let scale_ratio = f64::min(aw / w, aw / h);
                context.scale(scale_ratio, scale_ratio);
                let (offsetx, offsety) = if w > h {
                    (p, p + (aw - aw * h / w) / 2.0)
                } else {
                    (p + (aw - aw * w / h) / 2.0, p)
                };
                if w > h {
                    println!(
                        "w {} h {} effective height: {} offsety {}",
                        w,
                        h,
                        aw * h / w,
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
        context.restore();
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
                if Some((allocation.width, allocation.height))
                    == self
                        .model
                        .backing_buffer
                        .as_ref()
                        .map(|b| (b.get_width(), b.get_height()))
                {
                    // the backing buffer is good, just paint it
                    context.set_source_surface(
                        self.model.backing_buffer.as_ref().unwrap(),
                        0.0,
                        0.0,
                    );
                    context.paint();
                } else {
                    // need to set up the backing buffer
                    println!("computing the backing buffer");
                    self.model.backing_buffer =
                        Some(self.prepare_backing_buffer(allocation.width, allocation.height));
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
            Msg::ActiveProjectChanged(pid) => {
                let new_active = pid == self.model.project.id;
                if new_active != self.model.is_active {
                    self.model.is_active = new_active;
                    // force a recompute of the display
                    self.model.backing_buffer = None;
                }
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
