use std::{cell::RefCell, collections::HashMap, rc::Rc};

use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

mod imp {
    use super::*;
    use std::cell::Cell;

    use glib::Properties;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use projectpadsql::models::EnvironmentType;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::EnvironmentPicker)]
    pub struct EnvironmentPicker {
        #[property(get, set)]
        pub environment: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvironmentPicker {
        const NAME: &'static str = "EnvironmentPicker";
        type ParentType = adw::Bin;
        type Type = super::EnvironmentPicker;

        fn class_init(klass: &mut Self::Class) {
            // Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            // obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EnvironmentPicker {
        fn constructed(&self) {}
    }

    impl WidgetImpl for EnvironmentPicker {}

    impl adw::subclass::prelude::BinImpl for EnvironmentPicker {}
}

glib::wrapper! {
    pub struct EnvironmentPicker(ObjectSubclass<imp::EnvironmentPicker>)
        @extends gtk::Widget, adw::Bin;
}

struct DropdownListItemInfo {
    first_label: gtk::Label,
    second_label: gtk::Label,
    check_mark: gtk::Image,
}

impl EnvironmentPicker {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();
        let dropdown = gtk::DropDown::from_strings(&["DEV", "STG", "UAT", "PRD"]);

        let item_to_info = Rc::new(RefCell::new(
            HashMap::<gtk::ListItem, DropdownListItemInfo>::new(),
        ));

        let list_item_factory = gtk::SignalListItemFactory::new();
        let item_to_info_clone0 = item_to_info.clone();
        // pattern partly taken from the gtk4-demo "Selections"
        list_item_factory.connect_setup(move |_factory, list_item_obj| {
            let list_item = list_item_obj.downcast_ref::<gtk::ListItem>().unwrap();
            let hbox = gtk::Box::builder().spacing(10).build();
            let vbox = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let first_label = gtk::Label::builder()
                .css_classes(["heading"])
                .halign(gtk::Align::Start)
                .build();
            let second_label = gtk::Label::builder()
                .css_classes(["dim-label"])
                .halign(gtk::Align::Start)
                .build();
            vbox.append(&first_label);
            vbox.append(&second_label);

            let check_mark = gtk::Image::from_icon_name("object-select-symbolic");

            hbox.append(&vbox);
            hbox.append(&check_mark);
            list_item.set_child(Some(&hbox));
            item_to_info_clone0.borrow_mut().insert(
                list_item.clone(),
                DropdownListItemInfo {
                    first_label,
                    second_label,
                    check_mark,
                },
            );
        });

        let dropdown_clone = dropdown.clone();
        let item_to_info_clone = item_to_info.clone();
        list_item_factory.connect_bind(move |_factory, list_item_obj| {
            let list_item = list_item_obj.downcast_ref::<gtk::ListItem>().unwrap();
            let str_obj = list_item.item();

            dbg!(&dropdown_clone.selected_item());
            dbg!(list_item_obj);
            let item_info_b = item_to_info_clone.borrow();
            let item_info = item_info_b.get(list_item).unwrap();
            let check_mark = item_info.check_mark.clone();

            let str_val = str_obj.clone()
                .unwrap()
                .downcast_ref::<gtk::StringObject>()
                .unwrap()
                .string();
            let desc = match str_val.as_str() {
                "PRD" => "Production environment",
                "UAT" => "User Acceptance Testing environment",
                "STG" => "Staging environment",
                "DEV" => "Development environment",
                _ => unreachable!(),
            };
            item_info.first_label.set_label(str_val.as_str());
            item_info.second_label.set_label(desc);

            check_mark.set_opacity(if dropdown_clone.selected_item() == str_obj {
                1.0
            } else {
                0.0
            });

            dropdown_clone.connect_closure(
                "notify::selected-item",
                false,
                glib::closure_local!(@strong check_mark as cm, @strong str_obj as o => move |dd: gtk::DropDown, _item: glib::ParamSpec| {
                    cm.set_opacity(if dd.selected_item() == o {
                        1.0
                    } else {
                        0.0
                    });
                }),
            );
        });

        dropdown.set_list_factory(Some(&list_item_factory));
        this.set_child(Some(&dropdown));
        this
    }
}
