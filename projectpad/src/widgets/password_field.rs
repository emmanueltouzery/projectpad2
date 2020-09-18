use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    RequestPassword,
    PublishPassword(String),
}

pub struct Model {
    relm: relm::Relm<PasswordField>,
    text: String,
}

#[widget]
impl Widget for PasswordField {
    fn init_view(&mut self) {
        self.password_entry
            .set_icon_from_icon_name(gtk::EntryIconPosition::Secondary, Some(Icon::LOCK.name()));
    }

    fn model(relm: &relm::Relm<Self>, text: String) -> Model {
        Model {
            relm: relm.clone(),
            text,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::RequestPassword => {
                self.model.relm.stream().emit(Msg::PublishPassword(
                    self.password_entry.get_text().to_string(),
                ));
            }
            Msg::PublishPassword(_) => {}
        }
    }

    view! {
        #[name="password_entry"]
        gtk::Entry {
            input_purpose: gtk::InputPurpose::Password,
            visibility: false,
            text: &self.model.text,
        }
    }
}
