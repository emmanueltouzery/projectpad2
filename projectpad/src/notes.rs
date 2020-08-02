use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::*;
#[cfg(test)]
use nom::multi::many0;
use nom::multi::many_till;
use nom::IResult;

#[derive(Debug, PartialEq, Eq)]
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
