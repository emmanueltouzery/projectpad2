use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::InterestType;
use std::str::FromStr;

use crate::widgets::project_item::WidgetMode;

use super::common::{self, SuffixAction};

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::ProjectPoiViewEdit)]
    pub struct ProjectPoiViewEdit {
        #[property(get, set)]
        interest_type: Rc<RefCell<String>>,

        #[property(get, set)]
        path: Rc<RefCell<String>>,

        #[property(get, set)]
        text: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectPoiViewEdit {
        const NAME: &'static str = "ProjectPoiViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ProjectPoiViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectPoiViewEdit {
        fn constructed(&self) {}
    }

    impl WidgetImpl for ProjectPoiViewEdit {}

    impl adw::subclass::prelude::BinImpl for ProjectPoiViewEdit {}
}

glib::wrapper! {
    pub struct ProjectPoiViewEdit(ObjectSubclass<imp::ProjectPoiViewEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ProjectPoiViewEdit {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();
        this
    }

    pub fn poi_get_text_label(interest_type: InterestType) -> &'static str {
        match interest_type {
            InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
            _ => "Text",
        }
    }

    // call this after setting all the properties
    pub fn prepare(&self, widget_mode: WidgetMode) {
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .build();
        let server_item0 = adw::PreferencesGroup::builder().build();

        let path = self.property::<String>("path");
        let text = self.property::<String>("text");
        let interest_type =
            InterestType::from_str(&self.property::<String>("interest_type")).unwrap();

        let path = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "path",
            widget_mode,
            "Path",
            SuffixAction::copy(&path),
            &[],
        );
        server_item0.add(&path);

        let text = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "text",
            widget_mode,
            Self::poi_get_text_label(interest_type),
            SuffixAction::copy(&text),
            &[],
        );
        server_item0.add(&text);
        self.bind_property("interest_type", &text, "title")
            .transform_to(|_, it| {
                Some(Self::poi_get_text_label(InterestType::from_str(it).unwrap()).to_value())
            })
            .sync_create()
            .build();

        let interest_type_row = common::combo_row(
            self.upcast_ref::<glib::Object>(),
            "interest_type",
            widget_mode,
            "Interest Type",
            &[
                "Application",
                "Backup/archive",
                "Command to run",
                "Command to run (terminal)",
                "Config file",
                "Log file",
            ],
            |v| InterestType::from_str(&v.get::<String>().unwrap()).unwrap() as u8 as u32,
            |i| {
                InterestType::from_repr(TryInto::<u8>::try_into(i).unwrap())
                    .unwrap()
                    .to_string()
                    .to_value()
            },
        );
        server_item0.add(&interest_type_row);

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
