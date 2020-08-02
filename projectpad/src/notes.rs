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
    PlainText(&'a str),
}

const PLAINTEXT_STOP_CHARS: &str = "*[]\n\\`~";

fn plaintext(input: &str) -> IResult<&str, LineItem> {
    map(is_not(PLAINTEXT_STOP_CHARS), LineItem::PlainText)(input)
}

fn bold(input: &str) -> IResult<&str, LineItem> {
    let (input, _) = tag("**")(input)?;
    let (input, (o, _)) = many_till(line_item, tag("**"))(input)?;
    Ok((input, LineItem::Bold(o)))
}

fn line_item(input: &str) -> IResult<&str, LineItem> {
    alt((plaintext, bold))(input)
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
