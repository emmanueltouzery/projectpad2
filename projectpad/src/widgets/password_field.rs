use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    RevealPassword,
    CopyPassword,
    RequestPassword,
    PublishPassword(String),
}

pub struct Model {
    relm: relm::Relm<PasswordField>,
    text: String,
    popover: Option<gtk::Popover>,
}

#[widget]
impl Widget for PasswordField {
    fn init_view(&mut self) {
        self.password_entry
            .set_icon_from_icon_name(gtk::EntryIconPosition::Secondary, Some(Icon::LOCK.name()));
        self.init_popover();
        let popover = self.model.popover.as_ref().unwrap().clone();
        let password_entry = self.password_entry.clone();
        self.password_entry.connect_icon_release(move |_, _, _| {
            popover
                .set_pointing_to(&password_entry.get_icon_area(gtk::EntryIconPosition::Secondary));
            popover.popup();
        });
    }

    fn init_popover(&mut self) {
        self.model.popover = Some(gtk::Popover::new(Some(&self.password_entry)));
        let popover_vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_reveal_btn = gtk::ModelButtonBuilder::new().label("Reveal").build();
        relm::connect!(
            self.model.relm,
            popover_reveal_btn,
            connect_clicked(_),
            Msg::RevealPassword
        );
        popover_vbox.add(&popover_reveal_btn);
        let popover_copy_btn = gtk::ModelButtonBuilder::new().label("Copy").build();
        relm::connect!(
            self.model.relm,
            popover_copy_btn,
            connect_clicked(_),
            Msg::CopyPassword
        );
        popover_vbox.add(&popover_copy_btn);
        popover_vbox.show_all();
        self.model.popover.as_ref().unwrap().add(&popover_vbox);
    }

    fn model(relm: &relm::Relm<Self>, text: String) -> Model {
        Model {
            relm: relm.clone(),
            text,
            popover: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::RevealPassword => {}
            Msg::CopyPassword => {}
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
