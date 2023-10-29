use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gio::Action,
    glib::{self, Sender},
};

use crate::Project;

const OUTER_SIZE: i32 = 60;
const PADDING: i32 = 5;

mod imp {
    use std::{cell::Cell, f64::consts::PI};

    use super::*;

    #[derive(Default)]
    pub struct ProjectBadge {
        pub project: Cell<Project>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectBadge {
        const NAME: &'static str = "ProjectBadge";
        type Type = super::ProjectBadge;
        type ParentType = gtk::DrawingArea;
    }

    impl ObjectImpl for ProjectBadge {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_draw_func(&Self::draw_func);
            obj.set_size_request(OUTER_SIZE, OUTER_SIZE);
        }
    }

    impl WidgetImpl for ProjectBadge {}

    impl DrawingAreaImpl for ProjectBadge {}

    impl ProjectBadge {
        fn draw_func(
            drawing_area: &gtk::DrawingArea,
            context: &cairo::Context,
            allocation_width_: i32,
            allocation_height_: i32,
        ) {
            context.set_antialias(cairo::Antialias::Best);
            let output_scale = drawing_area.scale_factor();
            let allocation_width = allocation_width_ * output_scale;
            let allocation_height = allocation_height_ * output_scale;

            let style_context = drawing_area.style_context();

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
            context.set_source_rgb(
                fg_color.red().into(),
                fg_color.green().into(),
                fg_color.blue().into(),
            );

            context.arc(
                (allocation_width / 2).into(),
                (allocation_width / 2).into(),
                (allocation_width / 2 - PADDING).into(),
                0.0,
                2.0 * PI,
            );
            context.stroke_preserve().expect("stroke_preserve");
        }
    }
}

glib::wrapper! {
    pub struct ProjectBadge(ObjectSubclass<imp::ProjectBadge>)
        @extends gtk::DrawingArea, gtk::Widget;
}

impl ProjectBadge {
    pub fn new(sender: Sender<Action>, project: Project) -> Self {
        let row = glib::Object::new::<Self>();
        row.imp().project.set(project);
        row
    }
}
