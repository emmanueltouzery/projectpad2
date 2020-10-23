use crate::icons::Icon;
use gtk::prelude::*;
use itertools::Itertools;
use relm::Widget;
use relm_derive::{widget, Msg};
use sourceview::prelude::*;
#[cfg(test)]
use std::sync::Once;

const HEADER_CYCLE: &[&'static str] = &[" # ", " ## ", " ### ", " - "];

#[derive(Msg)]
pub enum Msg {
    TextBold,
    TextItalic,
    TextStrikethrough,
    TextHeading,
    TextLink,
    TextPassword,
    TextPreformat,
    TextBlockquote,
    // it would be too wasteful to notify the parent of the textview
    // contents everytime the textview changes. So the parent will
    // send us a RequestContents, and we'll return a PublishContents
    RequestContents,
    PublishContents(String),
}

pub struct Model {
    relm: relm::Relm<NoteEdit>,
    contents: String,
    accel_group: gtk::AccelGroup,
}

#[widget]
impl Widget for NoteEdit {
    fn init_view(&mut self) {
        let buf = sourceview::Buffer::new_with_language(
            &sourceview::LanguageManager::get_default()
                .unwrap()
                .get_language("markdown")
                .unwrap(),
        );
        buf.set_text(&self.model.contents);
        self.note_textview.set_buffer(Some(&buf));
        // println!(
        //     "{:?}",
        //     sourceview::LanguageManager::get_default()
        //         .unwrap()
        //         .get_language_ids()
        // );
        // buf.set_language(Some("markdown"));
        self.add_tool_accelerator(&self.bold_btn, 'b');
        self.add_tool_accelerator(&self.italic_btn, 'i');
        self.add_tool_accelerator(&self.strikethrough_btn, 's');
        self.add_tool_accelerator(&self.heading_btn, 'h');
        self.add_tool_accelerator(&self.link_btn, 'l');
        self.add_tool_accelerator(&self.password_btn, 'p');
        self.add_tool_accelerator(&self.preformat_btn, 'f');
        self.add_tool_accelerator(&self.blockquote_btn, 'q');
    }

    fn add_tool_accelerator(&self, btn: &gtk::ToolButton, key: char) {
        btn.add_accelerator(
            "clicked",
            &self.model.accel_group,
            key.into(),
            gdk::ModifierType::CONTROL_MASK,
            gtk::AccelFlags::VISIBLE,
        );
    }

    fn model(relm: &relm::Relm<Self>, params: (String, gtk::AccelGroup)) -> Model {
        let (contents, accel_group) = params;
        Model {
            relm: relm.clone(),
            contents,
            accel_group,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::TextBold => {
                Self::toggle_snippet(&self.note_textview, "**", "**");
            }
            Msg::TextItalic => {
                Self::toggle_snippet(&self.note_textview, "*", "*");
            }
            Msg::TextStrikethrough => {
                Self::toggle_snippet(&self.note_textview, "~~", "~~");
            }
            Msg::TextHeading => {
                Self::toggle_heading(&self.note_textview);
            }
            Msg::TextLink => {
                Self::toggle_snippet(&self.note_textview, "[", "](url)");
            }
            Msg::TextPassword => {
                Self::toggle_password(&self.note_textview);
            }
            Msg::TextPreformat => {
                Self::toggle_preformat(&self.note_textview);
            }
            Msg::TextBlockquote => {
                Self::toggle_blockquote(&self.note_textview);
            }
            Msg::RequestContents => {
                let buf = self.note_textview.get_buffer().unwrap();
                let new_contents = buf
                    .get_text(&buf.get_start_iter(), &buf.get_end_iter(), false)
                    .unwrap()
                    .to_string();
                self.model
                    .relm
                    .stream()
                    .emit(Msg::PublishContents(new_contents));
            }
            // meant for my parent
            Msg::PublishContents(_) => {}
        }
    }

    fn toggle_password(note_textview: &sourceview::View) {
        let buf = note_textview.get_buffer().unwrap();
        let sel_bounds = buf.get_selection_bounds();
        if sel_bounds.is_none() {
            // no selection
            Self::toggle_snippet(note_textview, "[pass`", "`]");
            return;
        }
        let (start_iter, end_iter) = sel_bounds.unwrap();
        let selected_text = buf
            .get_text(&start_iter, &end_iter, false)
            .unwrap()
            .to_string();
        let mut separator = "`".to_string();
        while selected_text.contains(&separator) {
            separator.push('`');
        }
        let extra_space = if selected_text.starts_with('`') || selected_text.ends_with('`') {
            " "
        } else {
            ""
        };
        let before = "[pass".to_string() + &separator + extra_space;
        let after = extra_space.to_string() + &separator + "]";
        Self::toggle_snippet(note_textview, &before, &after);
    }

    fn toggle_preformat(note_textview: &sourceview::View) {
        let buf = note_textview.get_buffer().unwrap();
        let sel_bounds = buf.get_selection_bounds();
        if sel_bounds.is_none() {
            // no selection
            Self::toggle_snippet(note_textview, "`", "`");
            return;
        }
        let (start_iter, end_iter) = sel_bounds.unwrap();
        let selected_text = buf
            .get_text(&start_iter, &end_iter, false)
            .unwrap()
            .to_string();
        if selected_text.contains('\n') {
            // multiline
            Self::toggle_snippet(note_textview, "\n```\n", "\n```\n");
        } else {
            // single line
            Self::toggle_snippet(note_textview, "`", "`");
        }
    }

    fn toggle_blockquote(note_textview: &sourceview::View) {
        let buf = note_textview.get_buffer().unwrap();
        let (start_offset, end_offset) = match buf.get_selection_bounds() {
            None => {
                // no selection
                let cursor_iter = buf.get_iter_at_mark(&buf.get_insert().unwrap());
                let offset = cursor_iter.get_offset();
                (offset, offset)
            }
            Some((sel_start_iter, sel_end_iter)) => {
                // selection
                (sel_start_iter.get_offset(), sel_end_iter.get_offset())
            }
        };
        let mut iter = buf.get_iter_at_offset(end_offset);
        if start_offset != end_offset {
            // there is a selection
            let mut start_iter = buf.get_iter_at_offset(start_offset);
            let selected_text = buf.get_text(&start_iter, &iter, false).unwrap().to_string();
            let lines: Vec<_> = selected_text.lines().collect();
            let next_selection: String = if lines.iter().all(|l| l.starts_with("> ")) {
                // remove the blockquote
                lines.iter().map(|l| &l[2..]).intersperse("\n").collect()
            } else {
                // add the blockquote
                lines
                    .iter()
                    .map(|l| format!("> {}", l))
                    .intersperse("\n".to_string())
                    .collect()
            };
            buf.delete(&mut start_iter, &mut iter);
            start_iter.set_offset(start_offset);
            buf.insert(&mut start_iter, &next_selection);
            // for the apidoc of textbuffer::insert:
            // iter is invalidated when insertion occurs, but the default signal handler
            // revalidates it to point to the end of the inserted text.
            // => start_iter now points to the end of the inserted text
            // iter.set_offset(start_offset); <-- for some reason iter is invalidated & even set_offset can't recover it
            buf.select_range(&buf.get_iter_at_offset(start_offset), &start_iter);
        } else {
            // no selection
            iter.backward_chars(iter.get_line_offset());
            let mut iter2 = buf.get_iter_at_offset(iter.get_offset() + 2);
            if buf
                .get_text(&iter, &iter2, false)
                .unwrap()
                .to_string()
                .as_str()
                == "> "
            {
                buf.delete(&mut iter, &mut iter2);
            } else {
                buf.insert(&mut iter, "> ");
            }
        }
    }

    // Toggle between '#', '##', '###', "-" and no header
    fn toggle_heading(note_textview: &sourceview::View) {
        let buf = note_textview.get_buffer().unwrap();
        let mut to_insert = " # ";
        let mut clear_chars = 0;
        let mut iter = buf.get_iter_at_offset(buf.get_property_cursor_position());
        iter.backward_chars(iter.get_line_offset());
        let mut iter2 = buf.get_start_iter();
        for (i, header) in HEADER_CYCLE.iter().enumerate() {
            iter2.set_offset(iter.get_offset() + header.len() as i32);
            if buf
                .get_text(&iter, &iter2, false)
                .unwrap()
                .to_string()
                .as_str()
                == *header
            {
                // this pattern is in use, next time
                // we want to move to the next pattern
                to_insert = if i + 1 >= HEADER_CYCLE.len() {
                    ""
                } else {
                    HEADER_CYCLE[i + 1]
                };
                clear_chars = header.len() as i32;
                break;
            }
        }
        if clear_chars > 0 {
            iter2.set_offset(iter.get_offset() + clear_chars);
            buf.delete(&mut iter, &mut iter2);
        }
        buf.insert(&mut iter, to_insert);
    }

    fn toggle_snippet(note_textview: &sourceview::View, before: &str, after: &str) {
        let before_len = before.len() as i32;
        let after_len = after.len() as i32;
        let buf = note_textview.get_buffer().unwrap();
        let (start_offset, end_offset) = match buf.get_selection_bounds() {
            None => {
                // no selection
                let cursor_iter = buf.get_iter_at_mark(&buf.get_insert().unwrap());
                let offset = cursor_iter.get_offset();
                (offset, offset)
            }
            Some((sel_start_iter, sel_end_iter)) => {
                // selection
                (sel_start_iter.get_offset(), sel_end_iter.get_offset())
            }
        };
        let mut iter = buf.get_iter_at_offset(end_offset);

        // if the selection is [**test**] and the user clicked bold, should we
        // un-toggle, meaning change the contents to [test]?
        let is_untoggle = start_offset >= before_len && {
            let mut iter2 = buf.get_iter_at_offset(end_offset + after_len);
            if buf.get_text(&iter2, &iter, false).unwrap() != after {
                false
            } else {
                let iter1 = buf.get_iter_at_offset(start_offset);
                iter2.set_offset(start_offset - before_len);
                buf.get_text(&iter1, &iter2, false).unwrap() == before
            }
        };

        if is_untoggle {
            // untoggle => remove the 'before' and 'after' strings
            let mut iter2 = buf.get_iter_at_offset(end_offset + after_len);
            buf.delete(&mut iter, &mut iter2);
            iter.set_offset(start_offset - before_len);
            iter2.set_offset(start_offset);
            buf.delete(&mut iter, &mut iter2);
            // restore the selection
            iter.set_offset(start_offset - before_len);
            iter2.set_offset(end_offset - before_len);
            buf.select_range(&iter, &iter2);
        } else {
            // plain toggle, add the 'before' and 'after' strings
            buf.insert(&mut iter, after);
            iter.set_offset(start_offset);
            buf.insert(&mut iter, before);
            iter.set_offset(start_offset);
            if start_offset < end_offset {
                // restore the selection
                iter.set_offset(start_offset + before_len);
                let iter_end = buf.get_iter_at_offset(end_offset + before_len);
                buf.select_range(&iter, &iter_end);
            } else {
                iter.set_offset(start_offset + before_len);
                buf.place_cursor(&iter);
            }
        }
    }

    view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            gtk::Toolbar {
                margin_top: 10,
                #[name="bold_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::BOLD.name()),
                    clicked => Msg::TextBold
                },
                #[name="italic_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::ITALIC.name()),
                    clicked => Msg::TextItalic
                },
                #[name="strikethrough_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::STRIKETHROUGH.name()),
                    clicked => Msg::TextStrikethrough
                },
                #[name="heading_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::HEADING.name()),
                    clicked => Msg::TextHeading
                },
                #[name="link_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::LINK.name()),
                    clicked => Msg::TextLink
                },
                #[name="password_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::LOCK.name()),
                    clicked => Msg::TextPassword
                },
                #[name="preformat_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::CODE.name()),
                    clicked => Msg::TextPreformat
                },
                #[name="blockquote_btn"]
                gtk::ToolButton {
                    icon_name: Some(Icon::QUOTE.name()),
                    clicked => Msg::TextBlockquote
                },
            },
            gtk::Frame {
                margin_start: 10,
                margin_end: 10,
                margin_bottom: 10,
                hexpand: true,
                vexpand: true,
                gtk::ScrolledWindow {
                    #[name="note_textview"]
                    sourceview::View {
                        editable: true,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
static INIT: Once = Once::new();

// https://stackoverflow.com/a/58006287/516188
#[cfg(test)]
fn tests_init() {
    INIT.call_once(|| {
        gtk::init().unwrap();
    });
}

#[cfg(test)]
fn assert_tv_contents_eq(expected: &'static str, buf: &gtk::TextBuffer) {
    let start_iter = buf.get_start_iter();
    let end_iter = buf.get_end_iter();
    assert_eq!(
        expected,
        buf.get_text(&start_iter, &end_iter, false)
            .unwrap()
            .to_string()
            .as_str()
    );
}

#[test]
fn toggle_snippet_should_add_bold() {
    tests_init();
    let tv = sourceview::View::new();
    NoteEdit::toggle_snippet(&tv, "**", "**");
    let buf = tv.get_buffer().unwrap();
    assert_tv_contents_eq("****", &buf);
    assert_eq!(2, buf.get_property_cursor_position());
}

#[test]
fn toggle_snippet_should_untoggle_bold() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("****");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_snippet(&tv, "**", "**");
    assert_tv_contents_eq("", &buf);
    assert_eq!(0, buf.get_property_cursor_position());
}

#[test]
fn toggle_snippet_should_untoggle_link() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("[](url)");
    let initial_iter = buf.get_iter_at_offset(1);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_snippet(&tv, "[", "](url)");
    assert_tv_contents_eq("", &buf);
    assert_eq!(0, buf.get_property_cursor_position());
}

#[test]
fn toggle_snippet_with_selection_should_wrap_selection() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("my amazing test");
    let select_start_iter = buf.get_iter_at_offset(3);
    let select_end_iter = buf.get_iter_at_offset(10);
    buf.select_range(&select_start_iter, &select_end_iter);
    NoteEdit::toggle_snippet(&tv, "**", "**");
    assert_tv_contents_eq("my **amazing** test", &buf);
    let selection_after = buf.get_selection_bounds().unwrap();
    assert_eq!(5, selection_after.0.get_offset());
    assert_eq!(12, selection_after.1.get_offset());
}

#[test]
fn toggle_snippet_with_selection_should_untoggle_selection() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("my **amazing** test");
    let select_start_iter = buf.get_iter_at_offset(5);
    let select_end_iter = buf.get_iter_at_offset(12);
    buf.select_range(&select_start_iter, &select_end_iter);
    NoteEdit::toggle_snippet(&tv, "**", "**");
    assert_tv_contents_eq("my amazing test", &buf);
    let selection_after = buf.get_selection_bounds().unwrap();
    assert_eq!(3, selection_after.0.get_offset());
    assert_eq!(10, selection_after.1.get_offset());
}

#[test]
fn toggle_heading_should_set_heading() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    NoteEdit::toggle_heading(&tv);
    assert_tv_contents_eq(" # ", &buf);
}

#[test]
fn toggle_heading_should_move_to_next_heading() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\n # my **amazing** test");
    let initial_iter = buf.get_iter_at_offset(10);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_heading(&tv);
    assert_tv_contents_eq("line1\n ## my **amazing** test", &buf);
}

#[test]
fn toggle_heading_should_wipe_heading_at_end_of_cycle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text(" - line1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_heading(&tv);
    assert_tv_contents_eq("line1\nmy **amazing** test", &buf);
}

#[test]
fn toggle_blockquote_with_no_selection_should_toggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_blockquote(&tv);
    assert_tv_contents_eq("> line1\nmy **amazing** test", &buf);
}

#[test]
fn toggle_blockquote_with_no_selection_should_untoggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\n> my **amazing** test");
    let initial_iter = buf.get_iter_at_offset(10);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_blockquote(&tv);
    assert_tv_contents_eq("line1\nmy **amazing** test", &buf);
}

#[test]
fn toggle_blockquote_with_selection_should_toggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    let select_start_iter = buf.get_start_iter();
    let select_end_iter = buf.get_end_iter();
    buf.select_range(&select_start_iter, &select_end_iter);
    NoteEdit::toggle_blockquote(&tv);
    assert_tv_contents_eq("> line1\n> my **amazing** test", &buf);
}

#[test]
fn toggle_blockquote_with_selection_should_untoggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("> line1\n> my **amazing** test");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    let select_start_iter = buf.get_start_iter();
    let select_end_iter = buf.get_end_iter();
    buf.select_range(&select_start_iter, &select_end_iter);
    NoteEdit::toggle_blockquote(&tv);
    assert_tv_contents_eq("line1\nmy **amazing** test", &buf);
}

#[test]
fn toggle_password_with_no_selection_should_toggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(2);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_password(&tv);
    assert_tv_contents_eq("li[pass``]ne1\nmy **amazing** test", &buf);
    assert_eq!(8, buf.get_property_cursor_position());
}

#[test]
fn toggle_password_with_no_selection_should_untoggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("li[pass``]ne1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(8);
    buf.place_cursor(&initial_iter);
    NoteEdit::toggle_password(&tv);
    assert_tv_contents_eq("line1\nmy **amazing** test", &buf);
    assert_eq!(2, buf.get_property_cursor_position());
}

#[test]
fn toggle_password_with_selection_should_toggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy **amazing** test");
    let initial_iter = buf.get_iter_at_offset(9);
    buf.place_cursor(&initial_iter);
    let select_end_iter = buf.get_iter_at_offset(20);
    buf.select_range(&initial_iter, &select_end_iter);
    NoteEdit::toggle_password(&tv);
    assert_tv_contents_eq("line1\nmy [pass`**amazing**`] test", &buf);
}

#[test]
fn toggle_password_with_selection_should_toggle_with_spaces_if_leading_backtick() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy `*amazing** test");
    let initial_iter = buf.get_iter_at_offset(9);
    buf.place_cursor(&initial_iter);
    let select_end_iter = buf.get_iter_at_offset(20);
    buf.select_range(&initial_iter, &select_end_iter);
    NoteEdit::toggle_password(&tv);
    assert_tv_contents_eq("line1\nmy [pass`` `*amazing** ``] test", &buf);
}

#[test]
fn toggle_password_with_selection_should_untoggle() {
    tests_init();
    let tv = sourceview::View::new();
    let buf = tv.get_buffer().unwrap();
    buf.set_text("line1\nmy [pass`**amazing**`] test");
    let initial_iter = buf.get_iter_at_offset(15);
    buf.place_cursor(&initial_iter);
    let select_end_iter = buf.get_iter_at_offset(26);
    buf.select_range(&initial_iter, &select_end_iter);
    NoteEdit::toggle_password(&tv);
    assert_tv_contents_eq("line1\nmy **amazing** test", &buf);
}
