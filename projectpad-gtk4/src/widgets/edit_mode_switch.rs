use gtk::prelude::*;

mod imp {
    use gtk::subclass::prelude::*;
    use std::cell::Cell;

    use super::*;
    use glib::{
        subclass::{prelude::ObjectImpl, types::ObjectSubclass},
        Properties,
    };
    use gtk::{gdk, subclass::widget::WidgetImpl};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EditModeSwitch)]
    pub struct EditModeSwitch {
        #[property(get, set)]
        edit_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditModeSwitch {
        const NAME: &'static str = "EditModeSwitch";
        type ParentType = gtk::Widget;
        type Type = super::EditModeSwitch;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EditModeSwitch {
        fn constructed(&self) {
            let s = self.obj().clone();
            let _ = self
                .obj()
                .connect_edit_mode_notify(move |_switch: &super::EditModeSwitch| {
                    s.queue_draw();
                });
        }
    }

    impl WidgetImpl for EditModeSwitch {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Vertical => (24, 24, -1, -1),
                _ => (46, 46, -1, -1),
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            // let widget = self.obj();

            // TODO deprecated
            let style_context = self.obj().style_context();
            let bg_color = style_context.lookup_color("accent_bg_color").unwrap();
            let fg_color = style_context.lookup_color("accent_fg_color").unwrap();

            snapshot.append_fill(
                &gsk4::Path::parse("M 12 0 A 1 1 0 0 0 12 24 L 32 24 A 12 12 0 0 0 32 0").unwrap(),
                gsk4::FillRule::Winding,
                &bg_color,
            );

            let x_offset = if self.obj().edit_mode() { 20.0 } else { 0.0 };

            snapshot.save();
            snapshot.translate(&graphene::Point::new(x_offset, 0.0));
            snapshot.append_fill(
                &gsk4::Path::parse("M 0 12 A 1 1 0 0 0 24 12 M 0 12 A 1 1 0 0 1 24 12").unwrap(),
                gsk4::FillRule::Winding,
                &fg_color,
            );
            snapshot.restore();

            let icon_paintable = gtk::IconTheme::for_display(
                &gdk::Display::default().expect("Could not connect to a display."),
            )
            .lookup_icon(
                if self.obj().edit_mode() {
                    "document-edit-symbolic"
                } else {
                    "view-reveal-symbolic"
                },
                &[],
                16,
                16,
                gtk::TextDirection::Ltr,
                gtk::IconLookupFlags::FORCE_SYMBOLIC,
            );

            // dbg!(gtk::Image::from_icon_name("view-reveal-symbolic").icon_name());
            // let icon_paintable = gtk::Image::from_icon_name("view-reveal-symbolic")
            //     .paintable()
            //     .unwrap();

            snapshot.save();
            snapshot.translate(&graphene::Point::new(x_offset + 4.0, 4.0));
            icon_paintable.snapshot_symbolic(snapshot, 16.0, 16.0, &[gdk::RGBA::BLACK]);
            snapshot.restore();
        }
    }
}

glib::wrapper! {
    pub struct EditModeSwitch(ObjectSubclass<imp::EditModeSwitch>)
        @extends gtk::Widget;
}

impl EditModeSwitch {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}
