use pulldown_cmark::{Event, Options, Parser, Tag};

// TODO
// passwords, eg LECIP SG teamviewer, VPN setup, LIT office VPN, VPN setup, Wifi passwords
// <hr> doesn't exactly look great

// https://stackoverflow.com/a/3705601/516188
fn escape_entities(input: &str) -> String {
    input.replace("&", "&amp;").replace("'", "&apos;")
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
    for event in parser {
        // println!("{:?}", event);
        match event {
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
            Event::Start(Tag::BlockQuote) => result.push_str(r##"<span background="#EED680">\n"##),
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
        }
    }
    result
}
