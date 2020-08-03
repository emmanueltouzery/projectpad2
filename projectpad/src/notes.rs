use pulldown_cmark::{Event, Options, Parser, Tag};

// TODO
// passwords, eg LECIP SG teamviewer, VPN setup, LIT office VPN, VPN setup, Wifi passwords
// <hr> doesn't exactly look great

// https://stackoverflow.com/a/3705601/516188
fn escape_entities(input: &str) -> String {
    input.replace("&", "&amp;").replace("'", "&apos;")
}

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

// https://developer.gnome.org/pygtk/stable/pango-markup-language.html
pub fn note_markdown_to_pango_markup(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&input, options);
    let mut result = "".to_string();
    let mut list_cur_idx = None;
    let mut in_item = false; // paragraphs inside bullets don't look nice
    let mut in_preformat = false; // don't escape & and ' in preformat blocks

    let events_with_passwords = get_events_with_passwords(parser);

    for event in events_with_passwords {
        // println!("{:?}", event);
        match event {
            EventExt::StandardEvent(std) => match std {
                Event::Start(Tag::Strong) => result.push_str("<b>"),
                Event::End(Tag::Strong) => result.push_str("</b>"),
                Event::Start(Tag::Emphasis) => result.push_str("<i>"),
                Event::End(Tag::Emphasis) => result.push_str("</i>"),
                Event::Start(Tag::Strikethrough) => result.push_str("<s>"),
                Event::End(Tag::Strikethrough) => result.push_str("</s>"),
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
                Event::Start(Tag::Heading(1)) => result.push_str("\n<span size=\"xx-large\">"),
                Event::Start(Tag::Heading(2)) => result.push_str("\n<span size=\"x-large\">"),
                Event::Start(Tag::Heading(3)) => result.push_str("\n<span size=\"large\">"),
                Event::Start(Tag::Heading(_)) => result.push_str("\n<span size=\"large\">"),
                Event::End(Tag::Heading(_)) => result.push_str("</span>\n"),
                Event::Start(Tag::CodeBlock(_)) => {
                    in_preformat = true;
                    result.push_str("<tt>")
                }
                Event::End(Tag::CodeBlock(_)) => {
                    result.push_str("</tt>");
                    in_preformat = false;
                }
                Event::Text(t) => {
                    if in_preformat {
                        // escape html for pango
                        let escaped_input = glib::markup_escape_text(&t).to_string();
                        result.push_str(&escaped_input);
                    } else {
                        let escaped = glib::markup_escape_text(&escape_entities(&t)).to_string();
                        result.push_str(&escaped);
                    }
                }
                Event::Code(t) => result.push_str(&t),
                Event::Html(t) => {
                    let escaped = glib::markup_escape_text(&escape_entities(&t)).to_string();
                    result.push_str(&escaped);
                }
                Event::Rule => result.push_str("\n-----\n"),
                Event::HardBreak | Event::SoftBreak => result.push_str("\n"),
                Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            },
            EventExt::Password(p) => {
                result.push_str(&format!(r#"<a href="pass://{}">password</a>"#, p));
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
