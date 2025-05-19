use std::{cell::RefCell, collections::HashMap, rc::Rc};

use adw::prelude::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::EnvironmentType;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use glib::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::EnvironmentPicker)]
    pub struct EnvironmentPicker {
        #[property(get, set)]
        environment: Rc<RefCell<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvironmentPicker {
        const NAME: &'static str = "EnvironmentPicker";
        type ParentType = adw::Bin;
        type Type = super::EnvironmentPicker;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EnvironmentPicker {}

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

#[derive(PartialEq, Eq)]
enum DropDownFactoryMode {
    Item,
    ListItem,
}

fn dropdown_get_factory(
    dropdown: &gtk::DropDown,
    mode: DropDownFactoryMode,
) -> gtk::SignalListItemFactory {
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
            .vexpand(mode != DropDownFactoryMode::ListItem)
            .valign(if mode == DropDownFactoryMode::ListItem {
                gtk::Align::Start
            } else {
                gtk::Align::Center
            })
            .build();
        let second_label = gtk::Label::builder()
            .css_classes(["dim-label"])
            .halign(gtk::Align::Start)
            .build();
        vbox.append(&first_label);
        if mode == DropDownFactoryMode::ListItem {
            vbox.append(&second_label);
        }

        let check_mark = gtk::Image::from_icon_name("object-select-symbolic");

        hbox.append(&vbox);
        if mode == DropDownFactoryMode::ListItem {
            hbox.append(&check_mark);
        }
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

    let item_to_signal_id = Rc::new(RefCell::new(
        HashMap::<gtk::ListItem, glib::SignalHandlerId>::new(),
    ));

    let item_to_signal_id_clone = item_to_signal_id.clone();
    list_item_factory.connect_bind(move |_factory, list_item_obj| {
            let list_item = list_item_obj.downcast_ref::<gtk::ListItem>().unwrap();
            let str_obj = list_item.item();

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
            item_info.first_label.set_css_classes(&["caption-heading", match str_val.as_str() {
                "PRD" => "project-item-prod",
                "UAT" => "project-item-uat",
                "STG" => "project-item-staging",
                "DEV" => "project-item-dev",
                _ => unreachable!(),
            }]);
            item_info.second_label.set_label(desc);

            check_mark.set_opacity(if dropdown_clone.selected_item() == str_obj {
                1.0
            } else {
                0.0
            });

            let signal_id = dropdown_clone.connect_closure(
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
            item_to_signal_id_clone.borrow_mut().insert(list_item.clone(), signal_id);
        });

    let dropdown_clone2 = dropdown.clone();
    list_item_factory.connect_unbind(move |_factory, list_item_obj| {
        let list_item = list_item_obj.downcast_ref::<gtk::ListItem>().unwrap();
        glib::signal_handler_disconnect(
            &dropdown_clone2,
            item_to_signal_id.borrow_mut().remove(list_item).unwrap(),
        );
    });

    list_item_factory
}

impl EnvironmentPicker {
    pub fn new(allowed_envs: &[EnvironmentType]) -> Self {
        let this = glib::Object::new::<Self>();

        let mut env_strings = vec![];
        let mut sorted_envs = vec![];
        if allowed_envs.contains(&EnvironmentType::EnvDevelopment) {
            env_strings.push("DEV");
            sorted_envs.push(EnvironmentType::EnvDevelopment);
        }
        if allowed_envs.contains(&EnvironmentType::EnvStage) {
            env_strings.push("STG");
            sorted_envs.push(EnvironmentType::EnvStage);
        }
        if allowed_envs.contains(&EnvironmentType::EnvUat) {
            env_strings.push("UAT");
            sorted_envs.push(EnvironmentType::EnvUat);
        }
        if allowed_envs.contains(&EnvironmentType::EnvProd) {
            env_strings.push("PRD");
            sorted_envs.push(EnvironmentType::EnvProd);
        }

        let dropdown = gtk::DropDown::from_strings(&env_strings);
        dropdown.set_css_classes(&["flat"]);

        let list_item_factory = dropdown_get_factory(&dropdown, DropDownFactoryMode::ListItem);
        let item_factory = dropdown_get_factory(&dropdown, DropDownFactoryMode::Item);

        dropdown.set_list_factory(Some(&list_item_factory));
        dropdown.set_factory(Some(&item_factory));

        let ses = sorted_envs.clone();
        let ses2 = sorted_envs.clone();
        this.bind_property("environment", &dropdown, "selected")
            .transform_to(move |_, number: i32| {
                let env = EnvironmentType::from_repr(number.try_into().unwrap())
                    .unwrap_or(EnvironmentType::EnvDevelopment);
                ses.iter().position(|e| *e == env).map(|p| p as u32)
            })
            .transform_from(move |_, number: u32| {
                ses2.get(number as usize).map(|v| (*v as i32).to_value())
            })
            .bidirectional()
            .sync_create()
            .build();

        this.set_child(Some(&dropdown));
        this
    }
}
