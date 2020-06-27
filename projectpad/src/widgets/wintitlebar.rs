use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    SearchClicked,
    SearchChanged,
}

pub struct Model {}

#[widget]
impl Widget for WinTitleBar {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {}
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::SearchClicked => {
                let new_visible = self.search_toggle.get_active();
                self.search_entry.set_visible(new_visible);
                if new_visible {
                    self.search_entry.grab_focus();
                }
            }
            Msg::SearchChanged => {
                println!(
                    "{}",
                    self.search_entry
                        .get_text()
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "".to_string())
                );
            }
        }
    }

    view! {
        #[name="header_bar"]
        gtk::HeaderBar {
            show_close_button: true,
            title: Some("Projectpad"),
            #[name="search_toggle"]
            gtk::ToggleButton {
                image: Some(&gtk::Image::new_from_icon_name(Some("edit-find-symbolic"), gtk::IconSize::Menu)),
                child: {
                    pack_type: gtk::PackType::End
                },
                toggled() => Msg::SearchClicked
            },
            #[name="search_entry"]
            gtk::Entry {
                visible: false,
                child: {
                    pack_type: gtk::PackType::End
                },
                changed() => Msg::SearchChanged
            },
        }
    }
}
