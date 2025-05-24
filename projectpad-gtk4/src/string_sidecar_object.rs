// a gobject containing a string and a int sidecar
use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::StringSidecarObject)]
    pub struct StringSidecarObject {
        #[property(get, set)]
        string: Rc<RefCell<String>>,

        #[property(get, set)]
        sidecar: Rc<RefCell<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StringSidecarObject {
        const NAME: &'static str = "StringSidecarObject";
        // type ParentType = glib::Object;
        type Type = super::StringSidecarObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for StringSidecarObject {}
}

glib::wrapper! {
    pub struct StringSidecarObject(ObjectSubclass<imp::StringSidecarObject>);
}

impl StringSidecarObject {
    pub fn new(str: String, sidecar: i32) -> Self {
        let obj = glib::Object::new::<Self>();
        obj.set_string(str);
        obj.set_sidecar(sidecar);
        obj
    }
}
