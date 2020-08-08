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
        (PassState::AfterOpeningBracket(e0), Event::Text(v))
            if v.as_ref().starts_with("pass|") && v.as_ref().ends_with("|") =>
        {
            let mut pass = v[5..].to_string();
            pass.pop();
            pass_state = PassState::AfterBody(
                vec![e0.clone(), EventExt::StandardEvent(Event::Text(v))],
                pass,
            );
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
                PassState::AfterBody(es, _) => sofar.extend(es.clone()),
                _ => {}
            }
            pass_state = PassState::None;
            sofar.push(EventExt::StandardEvent(evt.clone()));
            sofar
        }
    })
}

// https://github.com/gtk-rs/pango/issues/193
const PANGO_SCALE_LARGE: f64 = 1.2;
const PANGO_SCALE_X_LARGE: f64 = 1.44;
const PANGO_SCALE_XX_LARGE: f64 = 1.728;

const TAG_BOLD: &str = "bold";
const TAG_ITALICS: &str = "italics";
const TAG_STRIKETHROUGH: &str = "strikethrough";
const TAG_HEADER1: &str = "header1";
const TAG_HEADER2: &str = "header2";
const TAG_HEADER3: &str = "header3";
const TAG_CODE: &str = "code";
pub const TAG_LINK: &str = "link";
const TAG_LIST_ITEM: &str = "list_item";
const TAG_PARAGRAPH: &str = "paragraph";

// TODO call only once in the app lifetime
pub fn build_tag_table() -> gtk::TextTagTable {
    let tag_table = gtk::TextTagTable::new();
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_BOLD)
            .weight(pango::Weight::Bold.to_glib())
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_STRIKETHROUGH)
            .strikethrough(true)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_ITALICS)
            .style(pango::Style::Italic)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER1)
            .weight(pango::Weight::Bold.to_glib())
            .scale(PANGO_SCALE_XX_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER2)
            .weight(pango::Weight::Bold.to_glib())
            .scale(PANGO_SCALE_X_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_HEADER3)
            .weight(pango::Weight::Bold.to_glib())
            .scale(PANGO_SCALE_LARGE)
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_LINK)
            .underline(pango::Underline::Single)
            .foreground("blue")
            .build(),
    );
    tag_table.add(
        &gtk::TextTagBuilder::new()
            .name(TAG_CODE)
            .family("monospace")
            .wrap_mode(gtk::WrapMode::None)
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
pub struct LinkInfo {
    pub start_offset: i32,
    pub end_offset: i32,
    pub url: String,
}

pub struct NoteBufferInfo {
    pub buffer: gtk::TextBuffer,
    pub links: Vec<LinkInfo>,
}

// https://developer.gnome.org/pygtk/stable/pango-markup-language.html
pub fn note_markdown_to_text_buffer(input: &str, table: &gtk::TextTagTable) -> NoteBufferInfo {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, options);
    let mut list_cur_idx = None;
    let mut in_item = false; // paragraphs inside bullets don't look nice
    let mut active_tags = HashMap::new();

    let buffer = gtk::TextBuffer::new(Some(table));
    let mut end_iter = buffer.get_end_iter();
    let mut links = vec![];

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
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_BOLD).unwrap());
                    buffer.apply_tag_by_name(TAG_BOLD, &start_iter, &end_iter);
                }
                Event::Start(Tag::Emphasis) => {
                    active_tags.insert(TAG_ITALICS, end_iter.get_offset());
                }
                Event::End(Tag::Emphasis) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_ITALICS).unwrap());
                    buffer.apply_tag_by_name(TAG_ITALICS, &start_iter, &end_iter);
                }
                Event::Start(Tag::Strikethrough) => {
                    active_tags.insert(TAG_STRIKETHROUGH, end_iter.get_offset());
                }
                Event::End(Tag::Strikethrough) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_STRIKETHROUGH).unwrap());
                    buffer.apply_tag_by_name(TAG_STRIKETHROUGH, &start_iter, &end_iter);
                }
                Event::Start(Tag::Link(_, _, _title)) => {
                    active_tags.insert(TAG_LINK, end_iter.get_offset());
                    // let escaped_url = url.replace("&", "&amp;").replace("'", "&apos;");
                    // result.push_str(format!(r#"<a href="{}">"#, &escaped_url).as_str())
                }
                Event::End(Tag::Link(_, url, _)) => {
                    // result.push_str("</a>")
                    let start_offset = active_tags.remove(TAG_LINK).unwrap();
                    let start_iter = buffer.get_iter_at_offset(start_offset);
                    buffer.apply_tag_by_name(TAG_LINK, &start_iter, &end_iter);
                    links.push(LinkInfo {
                        start_offset,
                        end_offset: end_iter.get_offset(),
                        url: url.to_string(),
                    });
                }
                Event::Start(Tag::Image(_, _, _)) => {}
                Event::End(Tag::Image(_, _, _)) => {}
                Event::Start(Tag::List(start_idx)) => {
                    list_cur_idx = start_idx;
                }
                Event::End(Tag::List(_)) => {
                    list_cur_idx = None;
                }
                Event::Start(Tag::Item) => {
                    active_tags.insert(TAG_LIST_ITEM, end_iter.get_offset());
                    // in_item = true;
                    if let Some(idx) = list_cur_idx {
                        buffer.insert(&mut end_iter, format!("\n{}.\t", idx).as_str());
                        list_cur_idx = Some(idx + 1);
                    } else {
                        buffer.insert(&mut end_iter, "\n•\t");
                    }
                }
                Event::End(Tag::Item) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_LIST_ITEM).unwrap());
                    buffer.apply_tag_by_name(TAG_LIST_ITEM, &start_iter, &end_iter);
                    in_item = false;
                }
                Event::Start(Tag::Paragraph) => {
                    // if !in_item {
                    buffer.insert(&mut end_iter, "\n");
                    active_tags.insert(TAG_PARAGRAPH, end_iter.get_offset());
                    // }
                }
                Event::End(Tag::Paragraph) => {
                    // if !in_item {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_PARAGRAPH).unwrap());
                    buffer.apply_tag_by_name(TAG_PARAGRAPH, &start_iter, &end_iter);
                    buffer.insert(&mut end_iter, "\n");
                    // }
                }
                // color is accent yellow from https://developer.gnome.org/hig-book/unstable/design-color.html.en
                Event::Start(Tag::BlockQuote) => {
                    // result.push_str(r##"<span background="#EED680">\n"##)
                }
                Event::End(Tag::BlockQuote) => {
                    // result.push_str("</span>\n")
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
                    active_tags.insert(TAG_HEADER1, end_iter.get_offset());
                }
                Event::Start(Tag::Heading(2)) => {
                    active_tags.insert(TAG_HEADER2, end_iter.get_offset());
                }
                Event::Start(Tag::Heading(_)) => {
                    active_tags.insert(TAG_HEADER3, end_iter.get_offset());
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::End(Tag::Heading(1)) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_HEADER1).unwrap());
                    buffer.apply_tag_by_name(TAG_HEADER1, &start_iter, &end_iter);
                }
                Event::End(Tag::Heading(2)) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_HEADER2).unwrap());
                    buffer.apply_tag_by_name(TAG_HEADER2, &start_iter, &end_iter);
                }
                Event::End(Tag::Heading(_)) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_HEADER3).unwrap());
                    buffer.apply_tag_by_name(TAG_HEADER3, &start_iter, &end_iter);
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    active_tags.insert(TAG_CODE, end_iter.get_offset());
                }
                Event::End(Tag::CodeBlock(_)) => {
                    let start_iter =
                        buffer.get_iter_at_offset(active_tags.remove(TAG_CODE).unwrap());
                    buffer.apply_tag_by_name(TAG_CODE, &start_iter, &end_iter);
                }
                Event::Text(t) => {
                    buffer.insert(&mut end_iter, &t);
                }
                Event::Code(t) => {
                    buffer.insert(&mut end_iter, &t);
                }
                Event::Html(t) => {
                    buffer.insert(&mut end_iter, &t);
                }
                Event::Rule => {
                    buffer.insert(&mut end_iter, "---"); // TODO surely can do way better than that
                }
                Event::HardBreak | Event::SoftBreak => {
                    buffer.insert(&mut end_iter, "\n");
                }
                Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            },
            EventExt::Password(p) => {
                buffer.insert(&mut end_iter, "[PASSWORD]"); // TODO
                                                            // result.push_str(&format!(
                                                            //     // the emoji doesn't look nice in the link on my machine, the underline
                                                            //     // doesn't line up with the rest of the string => put it out of the link
                                                            //     r#"<span size="x-small">🔒</span><a href="pass://{}">[Password]</a>"#,
                                                            //     p
                                                            // ));
            }
        }
    }
    NoteBufferInfo { buffer, links }
}

#[test]
fn add_password_events() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass|secret|]*", options);
    let evts = get_events_with_passwords(parser);
    assert!(evts.iter().any(|e| match e {
        EventExt::Password(p) if p == "secret" => true,
        _ => false,
    }));
}

#[test]
fn incomplete_passwords_dont_drop_items() {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext("hello *world [pass|secret]*", options);
    let evts = get_events_with_passwords(parser);
    println!("{:?}", evts);
    assert_eq!(
        "hello world [pass|secret]",
        evts.iter()
            .filter_map(|e| match e {
                EventExt::StandardEvent(Event::Text(t)) => Some(t.to_string()),
                _ => None,
            })
            .fold("".to_string(), |sofar, cur| sofar + &cur)
    );
}
