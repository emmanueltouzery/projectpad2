use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::io::Cursor;
use std::rc::Rc;

pub const OUTER_SIZE: i32 = 60;
const PADDING: i32 = 5;

#[derive(Msg, Debug)]
pub enum Msg {
    Click,
    Activate(Project),
    ActiveProjectChanged(i32),
    MouseEnter,
    MouseLeave,
    MouseEnterProject(i32),
    MouseLeaveProject(i32),
    DarkThemeToggled,
}

pub struct Model {
    relm: relm::Relm<ProjectBadge>,
    project: Project,
    font_size_for_width: Rc<RefCell<Option<(i32, f64)>>>, // cache the computed font size
    backing_buffer: Rc<RefCell<Option<cairo::ImageSurface>>>,
    is_active: Rc<Cell<bool>>,
}

#[widget]
impl Widget for ProjectBadge {
    fn init_view(&mut self) {
        self.widgets
            .drawing_area
            .set_size_request(OUTER_SIZE, OUTER_SIZE);
        self.widgets.drawing_area.add_events(
            gdk::EventMask::BUTTON_PRESS_MASK
                | gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK,
        );
        let buf = self.model.backing_buffer.clone();
        let fsw = self.model.font_size_for_width.clone();
        let is_a = self.model.is_active.clone();
        let icon = self.model.project.icon.clone();
        let name = self.model.project.name.clone();
        self.widgets.drawing_area.connect_draw(move |da, context| {
            let allocation = da.get_allocation();
            let b0 = buf.borrow();
            let is_buffer_good = Some((allocation.width, allocation.height))
                == b0.as_ref().map(|b| (b.get_width(), b.get_height()));
            let surface_ref = if is_buffer_good {
                b0
            } else {
                drop(b0);
                // need to set up the backing buffer
                buf.replace(Some(Self::prepare_backing_buffer(
                    da,
                    &fsw,
                    is_a.get(),
                    &icon,
                    &name,
                    allocation.width,
                    allocation.height,
                )));
                buf.borrow()
            };
            // paint the backing buffer
            context.set_source_surface(surface_ref.as_ref().unwrap(), 0.0, 0.0);
            context.paint();
            Inhibit(false)
        });
    }

    fn model(relm: &relm::Relm<Self>, project: Project) -> Model {
        Model {
            relm: relm.clone(),
            project,
            font_size_for_width: Rc::new(RefCell::new(None)),
            backing_buffer: Rc::new(RefCell::new(None)),
            is_active: Rc::new(Cell::new(false)),
        }
    }

    fn prepare_backing_buffer(
        drawing_area: &gtk::DrawingArea,
        font_size_for_width: &RefCell<Option<(i32, f64)>>,
        is_active: bool,
        icon: &Option<Vec<u8>>,
        name: &str,
        allocation_width: i32,
        allocation_height: i32,
    ) -> cairo::ImageSurface {
        let buf =
            cairo::ImageSurface::create(cairo::Format::ARgb32, allocation_width, allocation_height)
                .expect("cairo backing buffer");
        let context = cairo::Context::new(&buf);

        // code to make the badge text bold, but i feel it doesn't work out
        // if let Some(family) = context.get_font_face().toy_get_family() {
        //     context.select_font_face(&family, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        // }

        // println!("drawing badge, allocation: {:?}", allocation);
        let new_fsw = match *font_size_for_width.borrow() {
            Some((w, font_size)) if w == allocation_width => {
                context.set_font_size(font_size);
                None
            }
            _ => Some((
                allocation_width,
                Self::compute_font_size(&context, (allocation_width - PADDING * 2) as f64 * 0.75),
            )),
        };
        if let Some(fsw) = new_fsw {
            font_size_for_width.replace(Some(fsw));
        }
        context.set_antialias(cairo::Antialias::Best);

        let style_context = drawing_area.get_style_context();

        gtk::render_background(
            &style_context,
            &context,
            0.0,
            0.0,
            allocation_width.into(),
            allocation_height.into(),
        );
        gtk::render_frame(
            &style_context,
            &context,
            0.0,
            0.0,
            allocation_width.into(),
            allocation_height.into(),
        );

        let fg_color = style_context.lookup_color("theme_fg_color").unwrap();
        context.set_source_rgb(fg_color.red, fg_color.green, fg_color.blue);
        if is_active {
            context.set_line_width(6.0);
            context.set_line_cap(cairo::LineCap::Round);
            context.move_to(10.0, allocation_height as f64 - 5.0);
            context.line_to(
                allocation_width as f64 - 10.0,
                allocation_height as f64 - 5.0,
            );
            context.stroke();
        }

        context.arc(
            (allocation_width / 2).into(),
            (allocation_width / 2).into(),
            (allocation_width / 2 - PADDING).into(),
            0.0,
            2.0 * PI,
        );
        context.stroke_preserve();
        let bg_color = style_context.lookup_color("theme_bg_color").unwrap();
        // so the goal here is to push the contrast. if the background color
        // is darker (<0.5) we go for pure black; if it's brighter, we go
        // for pure white.
        let bg_base = if bg_color.red < 0.5 { 0.0 } else { 1.0 };
        context.set_source_rgb(bg_base, bg_base, bg_base);
        context.fill();
        context.set_source_rgb(fg_color.red, fg_color.green, fg_color.blue);

        match icon {
            // the 'if' works around an issue reading from SQL. should be None if it's empty!!
            Some(icon) if !icon.is_empty() => Self::draw_icon(&context, allocation_width, icon),
            _ => Self::draw_label(&context, allocation_width, &name[..2]),
        }
        buf
    }

    fn compute_font_size(context: &cairo::Context, width: f64) -> f64 {
        let mut size = 5.0;
        context.set_font_size(size);
        while context.text_extents("HU").width < width * 0.8 {
            context.set_font_size(size);
            size += 1.0;
        }
        size
    }

    pub fn draw_icon(context: &cairo::Context, allocation_width: i32, icon: &[u8]) {
        context.save();
        match cairo::ImageSurface::create_from_png(&mut Cursor::new(icon)).ok() {
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
                if new_active != self.model.is_active.get() {
                    self.model.is_active.set(new_active);
                    // force a recompute of the display
                    self.model.backing_buffer.replace(None);
                    self.widgets.drawing_area.queue_draw();
                }
            }
            Msg::MouseEnter => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::MouseEnterProject(self.model.project.id));
            }
            Msg::MouseLeave => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::MouseLeaveProject(self.model.project.id));
            }
            Msg::DarkThemeToggled => {
                // force a recompute of the display
                self.model.backing_buffer.replace(None);
                self.widgets.drawing_area.queue_draw();
            }
            Msg::MouseEnterProject(_) => {}
            Msg::MouseLeaveProject(_) => {}
        }
    }

    view! {
        #[name="drawing_area"]
        gtk::DrawingArea {
            button_press_event(_, _) => (Msg::Click, Inhibit(false)),
            enter_notify_event(_, _) => (Msg::MouseEnter, Inhibit(false)),
            leave_notify_event(_, _) => (Msg::MouseLeave, Inhibit(false)),
        }
    }
}
