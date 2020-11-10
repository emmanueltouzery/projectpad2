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
    FilePicked,
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
        let filter = gtk::FileFilter::new();
        if let Some(ext) = self.model.file_extension.as_ref() {
            filter.add_pattern(&ext);
        } else {
            filter.add_pattern("*.*");
        }
        self.picker_btn.set_filter(&filter);
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
            Msg::FilePicked => {
                match self.picker_btn.get_filename().and_then(|f| {
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
                        standard_dialogs::display_error(
                            "Error loading the authentication key",
                            None,
                        );
                    }
                }
            }
            // meant for my parent
            Msg::FileChanged(_) => {}
            Msg::SaveAuthFile => {
                // https://stackoverflow.com/questions/54487052/how-do-i-add-a-save-button-to-the-gtk-filechooser-dialog
                let dialog = gtk::FileChooserDialogBuilder::new()
                    .title("Select destination folder")
                    .action(gtk::FileChooserAction::SelectFolder)
                    .use_header_bar(1)
                    .modal(true)
                    .build();
                let file_contents = self.model.file_contents.clone();
                let filename = self.model.filename.clone();
                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Save", gtk::ResponseType::Ok);
                dialog.connect_response(move |d, r| {
                    d.close();
                    let mut fname = None;
                    if r == gtk::ResponseType::Ok {
                        if let Some(filename) = d.get_filename() {
                            fname = Some(filename);
                        }
                    }
                    if let Some(fname) = fname {
                        if let Err(e) = Self::write_auth_key(&file_contents, &filename, fname) {
                            standard_dialogs::display_error(
                                "Error writing the file",
                                Some(Box::new(e)),
                            );
                        }
                    }
                });
                dialog.show();
            }
        }
    }

    fn write_auth_key(
        auth_key: &Option<Vec<u8>>,
        auth_key_filename: &Option<String>,
        folder: PathBuf,
    ) -> std::io::Result<()> {
        if let (Some(data), Some(fname)) = (auth_key, auth_key_filename) {
            let mut file = File::create(folder.join(fname))?;
            file.write_all(&data)
        } else {
            Ok(())
        }
    }

    view! {
        #[name="auth_key_stack"]
        gtk::Stack {
            // if there is no file, a file picker...
            #[name="picker_btn"]
            gtk::FileChooserButton({action: gtk::FileChooserAction::Open}) {
                child: {
                    name: Some("no_file")
                },
                hexpand: true,
                selection_changed(_) => Msg::FilePicked,
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
