use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::{many0, many1, many_till};
use nom::IResult;

#[derive(Debug, PartialEq, Eq)]
enum NoteElementNoBlockQuote<'a> {
    Header1(&'a str),
    Header2(&'a str),
    Header3(&'a str),
    Paragraph(Vec<LineItem<'a>>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum LineItem<'a> {
    Bold(Vec<LineItem<'a>>),
    Italics(Vec<LineItem<'a>>),
    Link(&'a str, Vec<LineItem<'a>>),
    PlainText(&'a str),
}

const PLAINTEXT_STOP_CHARS: &str = "*[]\n\\`~";

fn plaintext(input: &str) -> IResult<&str, LineItem> {
    map(is_not(PLAINTEXT_STOP_CHARS), LineItem::PlainText)(input)
}

fn bold(input: &str) -> IResult<&str, LineItem> {
    text_toggle(input, "**", LineItem::Bold)
}

fn italics(input: &str) -> IResult<&str, LineItem> {
    text_toggle(input, "*", LineItem::Italics)
}

// eat simple carriage returns before & after headers.
fn headers(input: &str) -> IResult<&str, NoteElementNoBlockQuote> {
    let (input, _) = many0(is_a("\n\r"))(input)?;
    let (input, o) = alt((
        header("###", NoteElementNoBlockQuote::Header3),
        header("##", NoteElementNoBlockQuote::Header2),
        header("#", NoteElementNoBlockQuote::Header1),
    ))(input)?;
    let (input, _) = many0(is_a("\n\r"))(input)?;
    Ok((input, o))
}

fn header<'a>(
    tag_val: &'static str,
    ctor: impl Fn(&'a str) -> NoteElementNoBlockQuote<'a>,
) -> impl Fn(&'a str) -> IResult<&'a str, NoteElementNoBlockQuote<'a>> {
    move |input| {
        let (input, _) = tag(format!("{} ", tag_val).as_str())(input)?;
        let (input, o) = is_not("\r\n")(input)?;
        let (input, _) = is_a("\r\n")(input)?;
        Ok((input, ctor(o)))
    }
}

fn text_toggle<'a>(
    input: &'a str,
    separator: &'static str,
    ctor: impl Fn(Vec<LineItem<'a>>) -> LineItem<'a>,
) -> IResult<&'a str, LineItem<'a>> {
    let (input, _) = tag(separator)(input)?;
    let (input, (o, _)) = many_till(line_item, tag(separator))(input)?;
    Ok((input, ctor(o)))
}

fn link(input: &str) -> IResult<&str, LineItem> {
    let (input, _) = tag("[")(input)?;
    let (input, (contents, _)) = many_till(line_item, tag("]"))(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, title) = is_not(")")(input)?;
    let (input, _) = tag(")")(input)?;
    Ok((input, LineItem::Link(title, contents)))
}

fn line_item(input: &str) -> IResult<&str, LineItem> {
    alt((plaintext, bold, italics, link))(input)
}

fn paragraph(input: &str) -> IResult<&str, NoteElementNoBlockQuote> {
    // the haskell parser included endOfInput https://github.com/emmanueltouzery/projectpad/blob/master/src/Notes.hs#L100
    let (input, (items, _)) = many_till(
        line_item,
        alt((
            end_of_paragraph,
            upcoming_preformat,
            upcoming_blockquote,
            upcoming_headers,
        )),
    )(input)?;
    Ok((
        input,
        // NoteElementNoBlockQuote::Paragraph(merge_plaintexts(&items)),
        NoteElementNoBlockQuote::Paragraph(items),
    ))
}

// fn merge_plaintexts<'a, 'b>(items: &'b [LineItem<'a>]) -> Vec<LineItem<'a>> {
//     items
//         .iter()
//         .fold((vec![], None), |mut sofar, cur| match (sofar.1, cur) {
//             (Some(LineItem::PlainText(a)), LineItem::PlainText(b)) => {
//                 (sofar.0, Some(LineItem::PlainText(&(a.to_string() + b))))
//             }
//             (Some(LineItem::PlainText(a)), _) => {
//                 sofar.0.push(sofar.1.unwrap());
//                 sofar
//             }
//             (None, p @ LineItem::PlainText(_)) => (sofar.0, Some(p.clone())),
//             (None, other) => {
//                 sofar.0.push(other.clone());
//                 sofar
//             }
//         })
//         .0 // !!!!!
// }

fn end_of_paragraph(input: &str) -> IResult<&str, ()> {
    let (input, _) = is_a("\r\n")(input)?;
    let (input, _) = is_a("\t ")(input)?;
    let (input, _) = is_a("\r\n")(input)?;
    Ok((input, ()))
}

fn upcoming_preformat(input: &str) -> IResult<&str, ()> {
    let (input, _) = is_a("\r\n")(input)?;
    let (input, _) = peek(tag("    "))(input)?;
    Ok((input, ()))
}

fn upcoming_blockquote(input: &str) -> IResult<&str, ()> {
    let (input, _) = is_a("\r\n")(input)?;
    let (input, _) = peek(tag("> "))(input)?;
    Ok((input, ()))
}

fn upcoming_headers(input: &str) -> IResult<&str, ()> {
    let (input, _) = is_a("\r\n")(input)?;
    let (input, _) = peek(headers)(input)?;
    Ok((input, ()))
}

fn note_element(input: &str) -> IResult<&str, NoteElementNoBlockQuote> {
    alt((headers, paragraph))(input)
}

fn note_document(input: &str) -> IResult<&str, Vec<NoteElementNoBlockQuote>> {
    many1(note_element)(input)
}

#[test]
fn parse_plaintext() {
    assert_eq!(
        many0(line_item)("hello world"),
        Ok(("", vec![LineItem::PlainText("hello world"),]))
    )
}

#[test]
fn parse_bold() {
    assert_eq!(
        many0(line_item)("hello **world**"),
        Ok((
            "",
            vec![
                LineItem::PlainText("hello "),
                LineItem::Bold(vec![LineItem::PlainText("world")])
            ]
        ))
    )
}

#[test]
fn parse_link() {
    assert_eq!(
        many0(line_item)("he[llo world](my-url) demo"),
        Ok((
            "",
            vec![
                LineItem::PlainText("he"),
                LineItem::Link("my-url", vec![LineItem::PlainText("llo world")]),
                LineItem::PlainText(" demo")
            ]
        ))
    )
}

#[test]
fn parse_link_with_rich_contents() {
    assert_eq!(
        many0(line_item)("he[llo **w*or*ld** demo](my-url)"),
        Ok((
            "",
            vec![
                LineItem::PlainText("he"),
                LineItem::Link(
                    "my-url",
                    vec![
                        LineItem::PlainText("llo "),
                        LineItem::Bold(vec![
                            LineItem::PlainText("w"),
                            LineItem::Italics(vec![LineItem::PlainText("or")]),
                            LineItem::PlainText("ld")
                        ]),
                        LineItem::PlainText(" demo")
                    ]
                ),
            ]
        ))
    )
}

#[test]
fn parse_note_document_simple_header() {
    assert_eq!(
        note_document("# hello world"),
        Ok(("", vec![NoteElementNoBlockQuote::Header1("hello world")]))
    );
}
