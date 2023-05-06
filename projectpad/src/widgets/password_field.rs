use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    RevealPassword(gtk::ModelButton),
    CopyPassword,
    RequestPassword,
    PublishPassword(String),
    PasswordChanged(String),
}

#[derive(PartialEq, Eq)]
pub enum ActivatesDefault {
    Yes,
    #[allow(dead_code)]
    No,
}

pub struct Model {
    relm: relm::Relm<PasswordField>,
    text: String,
    activates_default: ActivatesDefault,
    popover: Option<gtk::Popover>,
}

#[widget]
impl Widget for PasswordField {
    fn init_view(&mut self) {
        self.widgets
            .password_entry
            .set_icon_from_icon_name(gtk::EntryIconPosition::Secondary, Some(Icon::LOCK.name()));
        self.init_popover();
        let popover = self.model.popover.as_ref().unwrap().clone();
        let password_entry = self.widgets.password_entry.clone();
        self.widgets
            .password_entry
            .connect_icon_release(move |_, _, _| {
                popover
                    .set_pointing_to(&password_entry.icon_area(gtk::EntryIconPosition::Secondary));
                popover.popup();
            });
        let r = self.model.relm.clone();
        self.widgets
            .password_entry
            .connect_changed(move |p| r.stream().emit(Msg::PasswordChanged(p.text().to_string())));
    }

    fn init_popover(&mut self) {
        self.model.popover = Some(gtk::Popover::new(Some(&self.widgets.password_entry)));
        let popover_vbox = gtk::Box::builder()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_reveal_btn = gtk::ModelButton::builder()
            .label("Reveal")
            .role(gtk::ButtonRole::Check)
            .build();
        relm::connect!(
            self.model.relm,
            popover_reveal_btn,
            connect_clicked(btn),
            Msg::RevealPassword(btn.clone())
        );
        popover_vbox.add(&popover_reveal_btn);
        let popover_copy_btn = gtk::ModelButton::builder().label("Copy").build();
        relm::connect!(
            self.model.relm,
            popover_copy_btn,
            connect_clicked(_),
            Msg::CopyPassword
        );
        popover_vbox.add(&popover_copy_btn);
        popover_vbox.show_all();
        self.model.popover.as_ref().unwrap().add(&popover_vbox);
        self.model
            .popover
            .as_ref()
            .unwrap()
            .set_position(gtk::PositionType::Left);
    }

    fn model(relm: &relm::Relm<Self>, params: (String, ActivatesDefault)) -> Model {
        let (text, activates_default) = params;
        Model {
            relm: relm.clone(),
            text,
            activates_default,
            popover: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::RevealPassword(popover_reveal_btn) => {
                let new_reveal = !EntryExt::is_visible(&self.widgets.password_entry);
                self.widgets.password_entry.set_visibility(new_reveal);
                popover_reveal_btn.set_active(new_reveal);
            }
            Msg::CopyPassword => {
                if let Some(clip) = gtk::Clipboard::default(&self.widgets.password_entry.display())
                {
                    clip.set_text(self.widgets.password_entry.text().as_str());
                }
            }
            Msg::RequestPassword => {
                self.model.relm.stream().emit(Msg::PublishPassword(
                    self.widgets.password_entry.text().to_string(),
                ));
            }
            Msg::PublishPassword(_) => {}
            Msg::PasswordChanged(_) => {}
        }
    }

    view! {
        #[name="password_entry"]
        gtk::Entry {
            input_purpose: gtk::InputPurpose::Password,
            visibility: false,
            text: &self.model.text,
            activates_default: self.model.activates_default == ActivatesDefault::Yes
        }
    }
}
