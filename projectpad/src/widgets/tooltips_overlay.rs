use super::project_badge::OUTER_SIZE;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Msg)]
pub enum Msg {
    UpdateProjectTooltip(Option<(String, i32)>),
}

pub struct Model {
    tooltip: Rc<RefCell<Option<(String, i32)>>>,
}

#[widget]
impl Widget for TooltipsOverlay {
    fn init_view(&mut self) {
        let tooltip = self.model.tooltip.clone();
        self.tooltips_area.connect_draw(move |area, context| {
            if let Some(tooltip) = &tooltip.borrow().clone() {
                let label = tooltip.0.clone();
                let style_context = area.get_style_context();
                style_context.add_class("project_name_tooltip");
                let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
                let pango_context = area.create_pango_context();
                let layout = pango::Layout::new(&pango_context);
                layout.set_text(&label);
                layout.set_ellipsize(pango::EllipsizeMode::End);
                layout.set_width(350 * pango::SCALE);

                let rect = layout.get_extents().1;
                let text_w = (rect.width / pango::SCALE) as f64;
                let text_h = (rect.height / pango::SCALE) as f64;

                let total_width = text_w + padding.left as f64 + padding.right as f64;
                let total_height = text_h + padding.top as f64 + padding.bottom as f64;

                let y_offset = tooltip.1 + OUTER_SIZE / 2 - total_height as i32 / 2;

                gtk::render_background(
                    &style_context,
                    context,
                    0.0,
                    y_offset as f64,
                    total_width,
                    total_height,
                );

                gtk::render_frame(
                    &style_context,
                    context,
                    0.0,
                    y_offset as f64,
                    total_width,
                    total_height,
                );

                gtk::render_layout(
                    &style_context,
                    context,
                    padding.left as f64,
                    y_offset as f64 + padding.top as f64,
                    &layout,
                );
                style_context.remove_class("project_name_tooltip");
            }
            Inhibit(false)
        });
    }

    fn model(_relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            tooltip: Rc::new(RefCell::new(None)),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::UpdateProjectTooltip(tooltip) => {
                self.model.tooltip.replace(tooltip);
                self.tooltips_area.queue_draw();
            }
        }
    }

    view! {
        #[name="tooltips_area"]
        gtk::DrawingArea {
            child: {
                expand: true
            },
        }
    }
}
