use glib::translate::ToGlib;
use gtk::prelude::*;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashSet;

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
            .name("link")
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
    tag_table
}

// https://developer.gnome.org/pygtk/stable/pango-markup-language.html
pub fn note_markdown_to_pango_markup(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, options);
    let mut list_cur_idx = None;
    let mut in_item = false; // paragraphs inside bullets don't look nice
    let mut active_tags = HashSet::new();

    let events_with_passwords = get_events_with_passwords(parser);

    for event in events_with_passwords {
        // println!("{:?}", event);
        match event {
            EventExt::StandardEvent(std) => match std {
                Event::Start(Tag::Strong) => active_tags.insert(TAG_BOLD),
                Event::End(Tag::Strong) => active_tags.remove(TAG_BOLD),
                Event::Start(Tag::Emphasis) => active_tags.insert(TAG_ITALICS),
                Event::End(Tag::Emphasis) => active_tags.remove(TAG_ITALICS),
                Event::Start(Tag::Strikethrough) => active_tags.insert(TAG_STRIKETHROUGH),
                Event::End(Tag::Strikethrough) => active_tags.remove(TAG_STRIKETHROUGH),
                Event::Start(Tag::Link(_, url, _title)) => {
                    let escaped_url = url.replace("&", "&amp;").replace("'", "&apos;");
                    result.push_str(format!(r#"<a href="{}">"#, &escaped_url).as_str())
                }
                Event::End(Tag::Link(_, _, _)) => result.push_str("</a>"),
                Event::Start(Tag::Image(_, _, _)) => {}
                Event::End(Tag::Image(_, _, _)) => {}
                Event::Start(Tag::List(start_idx)) => {
                    list_cur_idx = start_idx;
                }
                Event::End(Tag::List(_)) => {
                    list_cur_idx = None;
                }
                Event::Start(Tag::Item) => {
                    in_item = true;
                    if let Some(idx) = list_cur_idx {
                        result.push_str(format!("\n{}. ", idx).as_str());
                        list_cur_idx = Some(idx + 1);
                    } else {
                        result.push_str("\n• ");
                    }
                }
                Event::End(Tag::Item) => {
                    in_item = false;
                }
                Event::Start(Tag::Paragraph) => {
                    if !in_item {
                        result.push_str("\n")
                    }
                }
                Event::End(Tag::Paragraph) => {
                    if !in_item {
                        result.push_str("\n")
                    }
                }
                // color is accent yellow from https://developer.gnome.org/hig-book/unstable/design-color.html.en
                Event::Start(Tag::BlockQuote) => {
                    result.push_str(r##"<span background="#EED680">\n"##)
                }
                Event::End(Tag::BlockQuote) => result.push_str("</span>\n"),
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
                Event::Start(Tag::Heading(1)) => active_tags.insert(TAG_HEADER1),
                Event::Start(Tag::Heading(2)) => active_tags.insert(TAG_HEADER2),
                Event::Start(Tag::Heading(3)) => active_tags.insert(TAG_HEADER3),
                Event::End(Tag::Heading(1)) => active_tags.remove(TAG_HEADER1),
                Event::End(Tag::Heading(2)) => active_tags.remove(TAG_HEADER2),
                Event::End(Tag::Heading(3)) => active_tags.remove(TAG_HEADER3),
                Event::Start(Tag::CodeBlock(_)) => active_tags.insert(TAG_CODE),
                Event::End(Tag::CodeBlock(_)) => active_tags.remove(TAG_CODE),
                Event::Text(t) => {
                    let escaped = glib::markup_escape_text(&t).to_string();
                    result.push_str(&escaped);
                }
                Event::Code(t) => result.push_str(&t),
                Event::Html(t) => {
                    let escaped = glib::markup_escape_text(&t).to_string();
                    result.push_str(&escaped);
                }
                Event::Rule => result.push_str("\n-----\n"),
                Event::HardBreak | Event::SoftBreak => result.push_str("\n"),
                Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            },
            EventExt::Password(p) => {
                result.push_str(&format!(
                    // the emoji doesn't look nice in the link on my machine, the underline
                    // doesn't line up with the rest of the string => put it out of the link
                    r#"<span size="x-small">🔒</span><a href="pass://{}">[Password]</a>"#,
                    p
                ));
            }
        }
    }
    result
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
