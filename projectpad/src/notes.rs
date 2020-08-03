use pulldown_cmark::{Event, Options, Parser, Tag};

// TODO
// Github tricks
// Podcasts
// Zeal
// hack _vimrc remote windows
// typescript enums
// bullets in "maps test asset" and in "Job"
// passwords, eg LECIP SG teamviewer, VPN setup, LIT office VPN, VPN setup, Wifi passwords
// <hr> doesn't exactly look great

// https://developer.gnome.org/pygtk/stable/pango-markup-language.html
pub fn note_markdown_to_pango_markup(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let escaped_input = glib::markup_escape_text(input).to_string();
    let parser = Parser::new_ext(&escaped_input, options);
    let mut result = "".to_string();
    let mut list_cur_idx = None;
    for event in parser {
        match event {
            Event::Start(Tag::Strong) => result.push_str("<b>"),
            Event::End(Tag::Strong) => result.push_str("</b>"),
            Event::Start(Tag::Emphasis) => result.push_str("<i>"),
            Event::End(Tag::Emphasis) => result.push_str("</i>"),
            Event::Start(Tag::Strikethrough) => result.push_str("<s>"),
            Event::End(Tag::Strikethrough) => result.push_str("</s>"),
            Event::Start(Tag::Link(_, url, _title)) => {
                result.push_str(format!(r#"<a href="{}">"#, url).as_str())
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
                if let Some(idx) = list_cur_idx {
                    result.push_str(format!("\n{}. ", idx).as_str());
                    list_cur_idx = Some(idx + 1);
                } else {
                    result.push_str("\n• ");
                }
            }
            Event::End(Tag::Item) => {}
            Event::Start(Tag::Paragraph) => result.push_str("\n"),
            Event::End(Tag::Paragraph) => result.push_str("\n"),
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
            Event::Start(Tag::CodeBlock(_)) => result.push_str("<tt>"),
            Event::End(Tag::CodeBlock(_)) => result.push_str("</tt>"),
            Event::Text(t) => result.push_str(&t),
            Event::Code(t) => result.push_str(&t),
            Event::Html(t) => result.push_str(&t),
            Event::Rule => result.push_str("\n-----\n"),
            Event::HardBreak | Event::SoftBreak => result.push_str("\n"),
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
        }
    }
    result
}
