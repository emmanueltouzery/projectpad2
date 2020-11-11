use glib::translate::ToGlib;
use gtk::prelude::*;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashMap;

// TODO
// <hr> doesn't exactly look great

// cmark parses the passwords like so:
// Text(Borrowed("[")) <-- opening bracket
// Text(Borrowed("pass|XXX|")) <-- body
// Text(Borrowed("]")) <-- closing bracket
#[derive(PartialEq, Debug, Clone)]
enum PassState<'a> {
    None,
    AfterOpeningBracket(EventExt<'a>),
    AfterPass(Vec<EventExt<'a>>),
    AfterBody(Vec<EventExt<'a>>, String),
}

#[derive(PartialEq, Debug, Clone)]
enum EventExt<'a> {
    StandardEvent(Event<'a>),
    Password(String),
}

fn get_events_with_passwords(parser: Parser) -> Vec<EventExt> {
    let mut pass_state = PassState::None;
    parser.fold(vec![], |mut sofar, evt| match (&pass_state, evt) {
        (_, Event::Text(v)) if v.as_ref() == "[" => {
            pass_state = PassState::AfterOpeningBracket(EventExt::StandardEvent(Event::Text(v)));
            sofar
        }
        (PassState::AfterOpeningBracket(e0), Event::Text(v)) if v.as_ref() == "pass" => {
            pass_state = PassState::AfterPass(vec![
                e0.clone(),
                EventExt::StandardEvent(Event::Text(v.clone())),
            ]);
            sofar
        }
        (PassState::AfterPass(vec0), Event::Code(ref v)) => {
            let pass = v.to_string();
            let mut vec = vec0.clone();
            vec.push(EventExt::StandardEvent(Event::Code(v.clone())));
            pass_state = PassState::AfterBody(vec, pass);
            sofar
        }
        (PassState::AfterBody(_, p), Event::Text(v)) if v.as_ref() == "]" => {
            sofar.push(EventExt::Password(p.clone()));
            pass_state = PassState::None;
            sofar
        }
        (ps, evt) => {
            // in case we were in the process of parsing a password and the parsing
            // didn't conclude positively, flush back the events that I held back
            match ps {
                PassState::AfterOpeningBracket(e) => sofar.push(e.clone()),
                PassState::AfterPass(es) => sofar.extend(es.clone()),
                PassState::AfterBody(es, _) => sofar.extend(es.clone()),
                _ => {}
            }
            pass_state = PassState::None;
            sofar.push(EventExt::StandardEvent(evt.clone()));
            sofar
        }
    })
}

const TAG_BOLD: &str = "bold";
const TAG_ITALICS: &str = "italics";
const TAG_STRIKETHROUGH: &str = "strikethrough";
const TAG_HEADER1: &str = "header1";
const TAG_HEADER2: &str = "header2";
const TAG_HEADER3: &str = "header3";
const TAG_CODE: &str = "code";
pub const TAG_LINK: &str = "link";
pub const TAG_PASSWORD: &str = "password";
const TAG_LIST_ITEM: &str = "list_item";
const TAG_PARAGRAPH: &str = "paragraph";
const TAG_BLOCKQUOTE1: &str = "blockquote1";
const TAG_BLOCKQUOTE2: &str = "blockquote2";
const TAG_BLOCKQUOTE3: &str = "blockquote3";

// TODO call only once in the app lifetime
pub fn build_tag_table() -> gtk::TextTagTable {
    let tag_table = gtk::TextTagTable::new();
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_BOLD)
            .wrap_mode(gtk::WrapMode::Word)
            .weight(pango::Weight::Bold.to_glib())
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_STRIKETHROUGH)
            .wrap_mode(gtk::WrapMode::Word)
            .strikethrough(true)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_ITALICS)
            .wrap_mode(gtk::WrapMode::Word)
            .style(pango::Style::Italic)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER1)
            .weight(pango::Weight::Bold.to_glib())
            .wrap_mode(gtk::WrapMode::Word)
            .scale(pango::SCALE_XX_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER2)
            .weight(pango::Weight::Bold.to_glib())
            .wrap_mode(gtk::WrapMode::Word)
            .scale(pango::SCALE_X_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER3)
            .weight(pango::Weight::Bold.to_glib())
            .wrap_mode(gtk::WrapMode::Word)
            .scale(pango::SCALE_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_LINK)
            .underline(pango::Underline::Single)
            .wrap_mode(gtk::WrapMode::Word)
            .foreground("blue")
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_PASSWORD)
            .wrap_mode(gtk::WrapMode::Word)
            .foreground("orange")
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_CODE)
            .family("monospace")
            .wrap_mode(gtk::WrapMode::None)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_BLOCKQUOTE1)
            .wrap_mode(gtk::WrapMode::Word)
            .left_margin(30)
            .left_margin(30)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_BLOCKQUOTE2)
            .wrap_mode(gtk::WrapMode::Word)
            .left_margin(40)
            .left_margin(40)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_BLOCKQUOTE3)
            .wrap_mode(gtk::WrapMode::Word)
            .left_margin(50)
            .left_margin(50)
            .build(),
    );

    // explanation on how the list items are implemented:
    // https://stackoverflow.com/a/63291090/516188
    let mut tab_ar = pango::TabArray::new(2, true);
    tab_ar.set_tab(0, pango::TabAlign::Left, 0);
    tab_ar.set_tab(1, pango::TabAlign::Left, 14);
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_LIST_ITEM)
            .indent(-14)
            .left_margin(14)
            .wrap_mode(gtk::WrapMode::Word) // this should be the default...
            .tabs(&tab_ar)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_PARAGRAPH)
            .wrap_mode(gtk::WrapMode::Word)
            .build(),
    );
    tag_table
}

#[derive(Debug)]
pub struct ItemDataInfo {
    pub start_offset: i32,
    pub end_offset: i32,
    pub data: String,
}

pub struct NoteBufferInfo {
    pub buffer: gtk::TextBuffer,
    pub links: Vec<ItemDataInfo>,
    pub passwords: Vec<ItemDataInfo>,
    pub separator_anchors: Vec<gtk::TextChildAnchor>,
}

pub fn note_markdown_to_quick_preview(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, options);
    let events_with_passwords = get_events_with_passwords(parser);
    let mut result = "".to_string();
    for event in events_with_passwords {
        match event {
            EventExt::StandardEvent(Event::Text(t)) => result.push_str(&t),
            EventExt::StandardEvent(Event::Code(t)) => result.push_str(&t),
            EventExt::StandardEvent(Event::Start(Tag::Paragraph)) => result.push_str("\n"),
            EventExt::StandardEvent(Event::End(Tag::Paragraph)) => result.push_str("\n"),
            EventExt::StandardEvent(Event::End(Tag::Heading(_))) => result.push_str("\n"),
            EventExt::Password(_) => result.push_str("[password]"),
            _ => {}
        }
    }
    result
}

// https://developer.gnome.org/pygtk/stable/pango-markup-language.html
pub fn note_markdown_to_text_buffer(input: &str, table: &gtk::TextTagTable) -> NoteBufferInfo {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, options);
    let mut list_cur_idx = None;
    let mut in_item = false; // paragraphs inside bullets don't look nice
    let mut active_tags = HashMap::new();
    let mut separator_anchors = Vec::new();

    let buffer = gtk::TextBuffer::new(Some(table));
    let mut end_iter = buffer.get_end_iter();
    let mut links = vec![];
    let mut passwords = vec![];
    let mut blockquote_level = 0;

    let events_with_passwords = get_events_with_passwords(parser);

    for event in events_with_passwords {
        // println!("{:?}", event);
        match event {
            EventExt::StandardEvent(std) => match std {
                // TODO code duplication
                Event::Start(Tag::Strong) => {
                    active_tags.insert(TAG_BOLD, end_iter.get_offset());
                }
                Event::End(Tag::Strong) => {
                    if let Some(start_offset) = active_tags.remove(TAG_BOLD) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_BOLD, &start_iter, &end_iter);
                    }
                }
                Event::Start(Tag::Emphasis) => {
                    active_tags.insert(TAG_ITALICS, end_iter.get_offset());
                }
                Event::End(Tag::Emphasis) => {
                    if let Some(start_offset) = active_tags.remove(TAG_ITALICS) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_ITALICS, &start_iter, &end_iter);
                    }
                }
                Event::Start(Tag::Strikethrough) => {
                    active_tags.insert(TAG_STRIKETHROUGH, end_iter.get_offset());
                }
                Event::End(Tag::Strikethrough) => {
                    if let Some(start_offset) = active_tags.remove(TAG_STRIKETHROUGH) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_STRIKETHROUGH, &start_iter, &end_iter);
                    }
                }
                Event::Start(Tag::Link(_, _, _title)) => {
                    active_tags.insert(TAG_LINK, end_iter.get_offset());
                }
                Event::End(Tag::Link(_, url, _)) => {
                    if let Some(start_offset) = active_tags.remove(TAG_LINK) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_LINK, &start_iter, &end_iter);
                        links.push(ItemDataInfo {
                            start_offset,
                            end_offset: end_iter.get_offset(),
                            data: url.to_string(),
                        });
                    }
                }
                Event::Start(Tag::Image(_, _, _)) => {}
                Event::End(Tag::Image(_, _, _)) => {}
                Event::Start(Tag::List(start_idx)) => {
                    list_cur_idx = start_idx;
                }
                Event::End(Tag::List(_)) => {
                    list_cur_idx = None;
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::Start(Tag::Item) => {
                    active_tags.insert(TAG_LIST_ITEM, end_iter.get_offset());
                    in_item = true;
                    if let Some(idx) = list_cur_idx {
                        buffer.insert(&mut end_iter, format!("\n{}.\t", idx).as_str());
                        list_cur_idx = Some(idx + 1);
                    } else {
                        buffer.insert(&mut end_iter, "\n•\t");
                    }
                }
                Event::End(Tag::Item) => {
                    if let Some(start_offset) = active_tags.remove(TAG_LIST_ITEM) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_LIST_ITEM, &start_iter, &end_iter);
                    }
                    in_item = false;
                }
                Event::Start(Tag::Paragraph) => {
                    if !in_item {
                        // i wanted to use gtktextview's pixels-above-lines and things
                        // like that, instead of inserting \n for vertical spacing,
                        // but i think gtktextview doesn't support soft carriage returns,
                        // meaning that any \n starts a new paragraph as far as textview
                        // is concerned, so you get lots of extra in-paragraph spacing.
                        if buffer.get_char_count() != 0 {
                            buffer.insert(&mut end_iter, "\n");
                        }
                        active_tags.insert(TAG_PARAGRAPH, end_iter.get_offset());
                    }
                }
                Event::End(Tag::Paragraph) => {
                    if !in_item {
                        if let Some(start_offset) = active_tags.remove(TAG_PARAGRAPH) {
                            let start_iter = buffer.get_iter_at_offset(start_offset);
                            buffer.apply_tag_by_name(TAG_PARAGRAPH, &start_iter, &end_iter);
                            buffer.insert(&mut end_iter, "\n");
                        }
                    }
                }
                Event::Start(Tag::BlockQuote) => {
                    blockquote_level += 1;
                    if let Some(tag) = get_blockquote_tag(blockquote_level) {
                        active_tags.insert(tag, end_iter.get_offset());
                    }
                }
                Event::End(Tag::BlockQuote) => {
                    if let Some(tag) = get_blockquote_tag(blockquote_level) {
                        if let Some(start_offset) = active_tags.remove(tag) {
                            let start_iter = buffer.get_iter_at_offset(start_offset);
                            buffer.apply_tag_by_name(tag, &start_iter, &end_iter);
                        }
                    }
                    blockquote_level -= 1;
                }
                Event::Start(Tag::FootnoteDefinition(_)) => {}
                Event::End(Tag::FootnoteDefinition(_)) => {}
                Event::Start(Tag::TableHead) => {}
                Event::End(Tag::TableHead) => {}
                Event::Start(Tag::TableRow) => {}
                Event::End(Tag::TableRow) => {}
                Event::Start(Tag::TableCell) => {}
                Event::End(Tag::TableCell) => {}
                Event::Start(Tag::Table(_)) => {}
                Event::End(Tag::Table(_)) => {}
                Event::Start(Tag::Heading(1)) => {
                    buffer.insert(&mut end_iter, "\n");
                    active_tags.insert(TAG_HEADER1, end_iter.get_offset());
                }
                Event::Start(Tag::Heading(2)) => {
                    buffer.insert(&mut end_iter, "\n");
                    active_tags.insert(TAG_HEADER2, end_iter.get_offset());
                }
                Event::Start(Tag::Heading(_)) => {
                    buffer.insert(&mut end_iter, "\n");
                    active_tags.insert(TAG_HEADER3, end_iter.get_offset());
                }
                Event::End(Tag::Heading(1)) => {
                    if let Some(start_offset) = active_tags.remove(TAG_HEADER1) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_HEADER1, &start_iter, &end_iter);
                    }
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::End(Tag::Heading(2)) => {
                    if let Some(start_offset) = active_tags.remove(TAG_HEADER2) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_HEADER2, &start_iter, &end_iter);
                    }
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::End(Tag::Heading(_)) => {
                    if let Some(start_offset) = active_tags.remove(TAG_HEADER3) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_HEADER3, &start_iter, &end_iter);
                    }
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    if buffer.get_char_count() != 0 {
                        buffer.insert(&mut end_iter, "\n");
                    }
                    active_tags.insert(TAG_CODE, end_iter.get_offset());
                }
                Event::End(Tag::CodeBlock(_)) => {
                    if let Some(start_offset) = active_tags.remove(TAG_CODE) {
                        let start_iter = buffer.get_iter_at_offset(start_offset);
                        buffer.apply_tag_by_name(TAG_CODE, &start_iter, &end_iter);
                    }
                }
                Event::Text(t) => {
                    buffer.insert(&mut end_iter, &t);
                }
                Event::Code(t) => {
                    active_tags.insert(TAG_CODE, end_iter.get_offset());
                    buffer.insert(&mut end_iter, &t);
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_CODE).unwrap());
                    buffer.apply_tag_by_name(TAG_CODE, &start_iter, &end_iter);
                }
                Event::Html(t) => {
                    buffer.insert(&mut end_iter, &t);
                }
                Event::Rule => {
                    let anchor = buffer.create_child_anchor(&mut end_iter).unwrap();
                    separator_anchors.push(anchor);
                }
                Event::HardBreak | Event::SoftBreak => {
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            },
            EventExt::Password(p) => {
                let start_offset = end_iter.get_offset();
                buffer.insert(&mut end_iter, "🔒[Password]");
                buffer.apply_tag_by_name(
                    TAG_PASSWORD,
                    &buffer.get_iter_at_offset(start_offset),
                    &end_iter,
                );
                passwords.push(ItemDataInfo {
                    start_offset,
                    end_offset: end_iter.get_offset(),
                    data: p,
                });
            }
        }
    }
    NoteBufferInfo {
        buffer,
        links,
        passwords,
        separator_anchors,
    }
}

fn get_blockquote_tag(blockquote_level: i32) -> Option<&'static str> {
    match blockquote_level {
        1 => Some(TAG_BLOCKQUOTE1),
        2 => Some(TAG_BLOCKQUOTE2),
        3 => Some(TAG_BLOCKQUOTE3),
        4..=i32::MAX => Some(TAG_BLOCKQUOTE3),
        i32::MIN..=0 => None,
    }
}

#[test]
fn add_password_events() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass`se*c~~r*et`]*", options);
    let evts = get_events_with_passwords(parser);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "se*c~~r*et" => true,
        _ => false,
    }));
}

#[test]
fn add_password_events_backticks() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass``sec`ret``]*", options);
    let evts = get_events_with_passwords(parser);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "sec`ret" => true,
        _ => false,
    }));
}

#[test]
fn add_password_events_double_backticks() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass```sec``ret```]*", options);
    let evts = get_events_with_passwords(parser);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "sec``ret" => true,
        _ => false,
    }));
}

#[test]
fn add_password_events_triple_backticks() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass````sec```ret````]*", options);
    let evts = get_events_with_passwords(parser);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "sec```ret" => true,
        _ => false,
    }));
}

#[test]
fn add_password_events_leading_trailing_backtick_space_approach() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass`` `sec`ret` ``]*", options);
    let evts = get_events_with_passwords(parser);
    println!("{:?}", evts);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "`sec`ret`" => true,
        _ => false,
    }));
}

#[test]
fn incomplete_passwords_dont_drop_items() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass`secret]*", options);
    let evts = get_events_with_passwords(parser);
    assert_eq!(
        "hello world [pass`secret]",
        evts.iter()
            .filter_map(|e| match e {
                EventExt::StandardEvent(Event::Text(t)) => Some(t.to_string()),
                _ => None,
            })
            .fold("".to_string(), |sofar, cur| sofar + &cur)
    );
}
