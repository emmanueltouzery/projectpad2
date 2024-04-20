use gtk::prelude::*;

mod imp {
    use super::*;
    use glib::subclass::{prelude::ObjectImpl, types::ObjectSubclass};
    use gtk::{gdk, subclass::widget::WidgetImpl};

    #[derive(Default)]
    pub struct EditModeSwitch {}

    #[glib::object_subclass]
    impl ObjectSubclass for EditModeSwitch {
        const NAME: &'static str = "EditModeSwitch";
        type ParentType = gtk::Widget;
        type Type = super::EditModeSwitch;
    }

    impl ObjectImpl for EditModeSwitch {}

    impl WidgetImpl for EditModeSwitch {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Vertical => (24, 24, -1, -1),
                _ => (46, 46, -1, -1),
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            // let widget = self.obj();

            let bg_color = gdk::RGBA::parse("#555555").unwrap();
            let fg_color = gdk::RGBA::parse("#eeeeee").unwrap();

            snapshot.append_fill(
                &gsk4::Path::parse("M 12 0 A 1 1 0 0 0 12 24 L 32 24 A 12 12 0 0 0 32 0").unwrap(),
                gsk4::FillRule::Winding,
                &bg_color,
            );

            snapshot.append_fill(
                &gsk4::Path::parse("M 0 12 A 1 1 0 0 0 24 12 M 0 12 A 1 1 0 0 1 24 12").unwrap(),
                gsk4::FillRule::Winding,
                &fg_color,
            );

            let icon_paintable = gtk::IconTheme::default().lookup_icon(
                "view-reveal-symbolic",
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
            snapshot.translate(&graphene::Point::new(4.0, 4.0));
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
