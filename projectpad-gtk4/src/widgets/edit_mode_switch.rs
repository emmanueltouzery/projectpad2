use gtk::prelude::*;

mod imp {
    use gtk::subclass::prelude::*;
    use std::{cell::Cell, sync::OnceLock};

    use super::*;
    use glib::{
        subclass::{prelude::ObjectImpl, types::ObjectSubclass, Signal},
        Properties,
    };
    use gtk::{gdk, subclass::widget::WidgetImpl};

    const ANIM_DURATION_MICROS: f64 = 150000.0;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EditModeSwitch)]
    pub struct EditModeSwitch {
        #[property(get, set)]
        edit_mode: Cell<bool>,

        anim_offset: Cell<f32>,
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

            let gesture = gtk::GestureClick::new();
            let o = self.obj().clone();
            gesture.connect_released(move |gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                let start_micros = o.frame_clock().unwrap().frame_time();
                o.add_tick_callback(
                    move |switch: &super::EditModeSwitch, clock: &gdk::FrameClock| {
                        let elapsed_micros: f64 =
                            ((clock.frame_time() - start_micros) as i32).into();
                        let edit_mode = switch.edit_mode();
                        let ratio = (elapsed_micros / ANIM_DURATION_MICROS) as f32;
                        if ratio < 1.0 {
                            let offset = if edit_mode {
                                20.0 * (1.0 - ratio)
                            } else {
                                20.0 * ratio
                            };
                            switch.imp().anim_offset.set(offset);
                            switch.queue_draw();
                            glib::ControlFlow::Continue
                        } else {
                            switch.set_edit_mode(!edit_mode);
                            switch
                                .imp()
                                .anim_offset
                                .set(if edit_mode { 0.0 } else { 20.0 });
                            switch.queue_draw();

                            switch.emit_by_name::<()>("toggled", &[&(!edit_mode)]);
                            glib::ControlFlow::Break
                        }
                    },
                );
            });
            self.obj().add_controller(gesture);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("toggled")
                    .param_types([bool::static_type()])
                    .build()]
            })
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

            // TODO deprecated https://discourse.gnome.org/t/replacement-for-gtk-snapshot-render-background/20562
            let style_context = self.obj().style_context();
            let bg_color = style_context.lookup_color("accent_bg_color").unwrap();
            let fg_color = style_context.lookup_color("accent_fg_color").unwrap();

            snapshot.append_fill(
                &gsk4::Path::parse("M 12 0 A 1 1 0 0 0 12 24 L 32 24 A 12 12 0 0 0 32 0").unwrap(),
                gsk4::FillRule::Winding,
                &bg_color,
            );

            let x_offset = self.anim_offset.get();

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

            snapshot.save();
            snapshot.translate(&graphene::Point::new(x_offset + 4.0, 4.0));
            snapshot.push_blur(1.1); // sadly i didn't find a better way => https://discourse.gnome.org/t/draw-symbolic-icon-on-snapshot-antialias/20556
            icon_paintable.snapshot_symbolic(snapshot, 16.0, 16.0, &[gdk::RGBA::BLACK]);
            snapshot.pop();
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
