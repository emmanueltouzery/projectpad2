use super::standard_dialogs;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

#[derive(Msg)]
pub enum Msg {
    RemoveAuthFile,
    SaveAuthFile,
    PickFile,
    FileChanged((Option<String>, Option<Vec<u8>>)),
}

pub struct Model {
    relm: relm::Relm<FileContentsButton>,
    filename: Option<String>,
    // store the contents & not the Path, because it's what I have
    // when reading from SQL. So by storing it also when adding a new
    // server, I have the same data for add & edit.
    file_contents: Option<Vec<u8>>,
    file_extension: Option<String>,
}

#[widget]
impl Widget for FileContentsButton {
    fn init_view(&mut self) {
        self.btn_box.get_style_context().add_class("linked");
        self.update_auth_file();
    }

    fn update_auth_file(&self) {
        self.auth_key_stack
            .set_visible_child_name(if self.model.filename.is_some() {
                "file"
            } else {
                "no_file"
            });
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (Option<String>, Option<Vec<u8>>, Option<String>),
    ) -> Model {
        let (filename, file_contents, file_extension) = params;
        Model {
            relm: relm.clone(),
            filename,
            file_contents,
            file_extension,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::RemoveAuthFile => {
                self.model.filename = None;
                self.update_auth_file();
                self.model
                    .relm
                    .stream()
                    .emit(Msg::FileChanged((None, None)));
            }
            Msg::PickFile => {
                let dialog = gtk::FileChooserNativeBuilder::new()
                    .action(gtk::FileChooserAction::Open)
                    .title("Select file")
                    .modal(true)
                    .build();
                let filter = gtk::FileFilter::new();
                if let Some(ext) = self.model.file_extension.as_ref() {
                    filter.add_pattern(&ext);
                } else {
                    filter.add_pattern("*.*");
                }
                dialog.set_filter(&filter);
                if dialog.run() == gtk::ResponseType::Accept {
                    match dialog.get_filename().and_then(|f| {
                        let path = Path::new(&f);
                        let fname = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.to_string());
                        let contents = std::fs::read(path).ok();
                        match (fname, contents) {
                            (Some(f), Some(c)) => Some((f, c)),
                            _ => None,
                        }
                    }) {
                        Some((f, c)) => {
                            self.model.filename = Some(f);
                            self.model.file_contents = Some(c);
                            self.update_auth_file();
                            self.model.relm.stream().emit(Msg::FileChanged((
                                self.model.filename.clone(),
                                self.model.file_contents.clone(),
                            )));
                        }
                        None => {
                            standard_dialogs::display_error("Error reading the file", None);
                        }
                    }
                }
            }
            // meant for my parent
            Msg::FileChanged(_) => {}
            Msg::SaveAuthFile => {
                // native file picker to save files, so it works also within flatpak
                let dialog = gtk::FileChooserNativeBuilder::new()
                    .title("Select destination folder")
                    .action(gtk::FileChooserAction::SelectFolder)
                    .accept_label("Save")
                    .modal(true)
                    .build();
                let file_contents = self.model.file_contents.clone();
                let filename = self.model.filename.clone();

                if dialog.run() == gtk::ResponseType::Accept {
                    match dialog.get_filename() {
                        Some(f) => {
                            if let Err(e) = Self::write_auth_key(
                                &file_contents,
                                &filename,
                                f,
                                &self.model.file_extension,
                            ) {
                                standard_dialogs::display_error(
                                    "Error writing the file",
                                    Some(Box::new(e)),
                                );
                            }
                        }
                        None => {
                            standard_dialogs::display_error("Invalid filename selected", None);
                        }
                    }
                }
            }
        }
    }

    fn write_auth_key(
        auth_key: &Option<Vec<u8>>,
        auth_key_filename: &Option<String>,
        folder: PathBuf,
        file_extension: &Option<String>,
    ) -> std::io::Result<()> {
        if let (Some(data), Some(fname)) = (auth_key, auth_key_filename) {
            // in the case of the project icon picker, we display something like "<project icon>"
            // => let's save to disk "project icon.png"
            let mut corrected_fname = fname.replace('<', "").replace('>', "");
            if let Some(ext) = file_extension {
                corrected_fname.push_str(&ext[1..]);
            }
            let mut file = File::create(folder.join(corrected_fname))?;
            file.write_all(&data)
        } else {
            Ok(())
        }
    }

    view! {
        #[name="auth_key_stack"]
        gtk::Stack {
            // if there is no file, a file picker...
            // I used to use FileChooserButton here, but I couldn't
            // make it use the native file picker, the file extension
            // filters weren't working when being used in a flatpak.
            #[name="picker_btn"]
            gtk::Button {
                child: {
                    name: Some("no_file")
                },
                hexpand: true,
                gtk::Box {
                    gtk::Label {
                        text: "(None)",
                        xalign: 0.0,
                        hexpand: true,
                    },
                    gtk::Image {
                        property_icon_name: Some("document-open-symbolic"),
                    },
                },
                button_press_event(_, _) => (Msg::PickFile, Inhibit(false)),
            },
            // if there is a file, a label with the filename,
            // and a button to remove the file
            #[name="btn_box"]
            gtk::Box {
                orientation: gtk::Orientation::Horizontal,
                child: {
                    name: Some("file")
                },
                gtk::Label {
                    hexpand: true,
                    ellipsize: pango::EllipsizeMode::End,
                    text: self.model.filename.as_deref().unwrap_or_else(|| "")
                },
                gtk::Button {
                    always_show_image: true,
                    image: Some(&gtk::Image::from_icon_name(
                        Some("document-save-symbolic"), gtk::IconSize::Menu)),
                    button_press_event(_, _) => (Msg::SaveAuthFile, Inhibit(false)),
                },
                gtk::Button {
                    always_show_image: true,
                    image: Some(&gtk::Image::from_icon_name(
                        Some("edit-delete-symbolic"), gtk::IconSize::Menu)),
                    button_press_event(_, _) => (Msg::RemoveAuthFile, Inhibit(false)),
                },
            },
        },
    }
}
