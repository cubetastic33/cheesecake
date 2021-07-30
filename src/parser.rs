use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_until},
    combinator::{all_consuming, map, map_parser},
    sequence::{delimited, preceded, pair},
    regex::Regex, Slice, IResult,
};
use lazy_static::lazy_static;

// Structure to store the AST
#[derive(Debug)]
pub enum Expression<'a> {
    Text(&'a str),
    CustomEmoji(&'a str, String),
    // Emoji(&'a str),
    User(&'a str),
    Role(&'a str),
    Channel(&'a str),
    Hyperlink(&'a str, &'a str),
    Blockquote(Vec<Expression<'a>>),
    MultilineCode(&'a str),
    InlineCode(&'a str),
    Spoiler(Vec<Expression<'a>>),
    Underline(Vec<Expression<'a>>),
    Strikethrough(Vec<Expression<'a>>),
    Bold(Vec<Expression<'a>>),
    Italics(Vec<Expression<'a>>),
}

lazy_static! {
    static ref CUSTOM_EMOJI_RE: Regex = Regex::new(r"^<(a?):(\w+):(\d+)(>)").unwrap();
    // static ref EMOJI_RE: Regex = Regex::new(r"^\p{Emoji_Presentation}+").unwrap();
    static ref USER_RE: Regex = Regex::new(r"^<@!?(\d+)(>)").unwrap();
    static ref ROLE_RE: Regex = Regex::new(r"^<@&(\d+)(>)").unwrap();
    static ref CHANNEL_RE: Regex = Regex::new(r"^<#(\d+)(>)").unwrap();
    static ref LINK_RE: Regex = Regex::new(r"^(https?|ftp|file)(://[-A-Za-z0-9+&@#/%?=~_|!:,.;]*[A-Za-z0-9+&@#/%=~_|])").unwrap();
}

// Re-implement re_capture from nom, but make it take &'a Regex instead of Regex
// This provides a noticeable speed improvement since we don't have to RE.clone() each time
pub fn re_capture<'a, E>(re: &'a Regex) -> impl Fn(&'a str) -> IResult<&'a str, Vec<&'a str>, E>
    where
        E: nom::error::ParseError<&'a str>,
{
    move |i| {
        if let Some(c) = re.captures(i) {
            let v: Vec<_> = c
                .iter()
                .filter(|el| el.is_some())
                .map(|el| el.unwrap())
                .map(|m| i.slice(m.start()..m.end()))
                .collect();
            let offset = {
                let end = v.last().unwrap();
                end.as_ptr() as usize + end.len() - i.as_ptr() as usize
            };
            Ok((i.slice(offset..), v))
        } else {
            Err(nom::Err::Error(E::from_error_kind(i, nom::error::ErrorKind::RegexpCapture)))
        }
    }
}

/*
// Re-implement re_find from nom, but make it take &'a Regex instead of Regex
pub fn re_find<'a, E>(re: &'a Regex) -> impl Fn(&'a str) -> IResult<&'a str, &'a str, E>
    where
        E: nom::error::ParseError<&'a str>,
{
    move |i| {
        if let Some(m) = re.find(i) {
            Ok((i.slice(m.end()..), i.slice(m.start()..m.end())))
        } else {
            Err(nom::Err::Error(E::from_error_kind(i, nom::error::ErrorKind::RegexpFind)))
        }
    }
}
*/

// Parses custom emoji
fn custom_emoji(input: &str) -> IResult<&str, (&str, String)> {
    let result = re_capture(&CUSTOM_EMOJI_RE)(input)?;
    let extension = if result.1[1] == "a" { "gif" } else { "png" };
    Ok((result.0, (result.1[2], format!("{}.{}", result.1[3], extension))))
}

/*
// Parses unicode emoji
fn emoji(input: &str) -> IResult<&str, &str> {
    re_find(&EMOJI_RE)(input)
}
*/

// Parses user mentions
fn user(input: &str) -> IResult<&str, &str> {
    let result = re_capture(&USER_RE)(input)?;
    Ok((result.0, result.1[1]))
}

// Parses role mentions
fn role(input: &str) -> IResult<&str, &str> {
    let result = re_capture(&ROLE_RE)(input)?;
    Ok((result.0, result.1[1]))
}

// Parses channel links
fn channel(input: &str) -> IResult<&str, &str> {
    let result = re_capture(&CHANNEL_RE)(input)?;
    Ok((result.0, result.1[1]))
}

// Parses hyperlinks
fn hyperlink(input: &str) -> IResult<&str, (&str, &str)> {
    let result = alt((
        re_capture(&LINK_RE),
        delimited(tag("<"), re_capture(&LINK_RE), tag(">")),
    ))(input)?;
    Ok((result.0, (result.1[0], result.1[0])))
}

// Parses hyperlinks with embed rules
fn embed_hyperlink(input: &str) -> IResult<&str, (&str, &str)> {
    alt((
        hyperlink,
        pair(
            delimited(tag("["), take_until("]"), tag("]")),
            delimited(tag("("), |input| {
                let x = hyperlink(input)?;
                Ok((x.0, x.1.0))
            }, tag(")"))
        ),
    ))(input)
}

// Parses blockquotes
fn blockquote(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        alt((
            delimited(tag("> "), take_until("\n"), tag("\n")),
            preceded(tag("> "), take_while(|_| true)),
        )),
        parse_inline,
    )(input)
}

// Parses multiline code
fn multiline_code(input: &str) -> IResult<&str, &str> {
    delimited(tag("```"), take_until("```"), tag("```"))(input)
}

// Parses inline code
fn inline_code(input: &str) -> IResult<&str, &str> {
    alt((
        // If the inline code block is delimited by ``
        delimited(tag("``"), take_until("``"), tag("``")),
        // If the inline code block is delimited by `
        delimited(tag("`"), take_until("`"), tag("`")),
    ))(input)
}

// Parses spoiler text
fn spoiler(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        delimited(tag("||"), take_until("||"), tag("||")),
        parse_inline,
    )(input)
}

// Parses underlined text
fn underline(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        delimited(tag("__"), take_until("__"), tag("__")),
        parse_inline,
    )(input)
}

// Parses strikethroughed text
fn strikethrough(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        delimited(tag("~~"), take_until("~~"), tag("~~")),
        parse_inline,
    )(input)
}

// Parses bold text
fn bold(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        delimited(tag("**"), take_until("**"), tag("**")),
        parse_inline,
    )(input)
}

// Parses italicized text
fn italics(input: &str) -> IResult<&str, Vec<Expression>> {
    map_parser(
        alt((
            // TODO account for bold and underline
            // If the italics is delimited by _
            delimited(tag("_"), take_until("_"), tag("_")),
            // If the italics is delimited by *
            delimited(tag("*"), take_until("*"), tag("*")),
        )),
        parse_inline,
    )(input)
}

fn directive(input: &str) -> IResult<&str, Expression> {
    alt((
        map(custom_emoji, |(name, filename)| Expression::CustomEmoji(name, filename)),
        // map(emoji, Expression::Emoji),
        map(user, Expression::User),
        map(role, Expression::Role),
        map(channel, Expression::Channel),
        map(hyperlink, |(text, href)| Expression::Hyperlink(text, href)),
        map(blockquote, Expression::Blockquote),
        map(multiline_code, Expression::MultilineCode),
        map(inline_code, Expression::InlineCode),
        map(spoiler, Expression::Spoiler),
        map(underline, Expression::Underline),
        map(strikethrough, Expression::Strikethrough),
        map(bold, Expression::Bold),
        map(italics, Expression::Italics),
    ))(input)
}

fn embed_directive(input: &str) -> IResult<&str, Expression> {
    alt((
        map(custom_emoji, |(name, filename)| Expression::CustomEmoji(name, filename)),
        // map(emoji, Expression::Emoji),
        map(user, Expression::User),
        map(role, Expression::Role),
        map(channel, Expression::Channel),
        map(embed_hyperlink, |(text, href)| Expression::Hyperlink(text, href)),
        map(blockquote, Expression::Blockquote),
        map(multiline_code, Expression::MultilineCode),
        map(inline_code, Expression::InlineCode),
        map(spoiler, Expression::Spoiler),
        map(underline, Expression::Underline),
        map(strikethrough, Expression::Strikethrough),
        map(bold, Expression::Bold),
        map(italics, Expression::Italics),
    ))(input)
}

/// Parse a line of text, counting anything that doesn't match a directive as plain text.
fn parse_inline(input: &str) -> IResult<&str, Vec<Expression>> {
    let mut output = Vec::with_capacity(4);

    let mut current_input = input;

    while !current_input.is_empty() {
        let mut found_directive = false;
        for (current_index, c) in current_input.char_indices() {
            // Check for shrug emote
            if c == '¯' && current_input[current_index..].starts_with(r"¯\_(ツ)_/¯") {
                let leading_text = &current_input[0..current_index];
                if !leading_text.is_empty() {
                    output.push(Expression::Text(leading_text));
                }
                // Push the shrug emote as Expression::Text
                output.push(Expression::Text(r"¯\_(ツ)_/¯"));
                // Remove the emote from current_input
                current_input = &current_input[current_index + r"¯\_(ツ)_/¯".len()..];
                found_directive = true;
                break;
            }
            // Check for backslash
            if c == '\\' && current_input[current_index..].len() > 1 {
                let leading_text = &current_input[0..current_index];
                if !leading_text.is_empty() {
                    output.push(Expression::Text(leading_text));
                }
                // Push the escaped character as Expression::Text
                let (char_pos, c) = current_input.char_indices().nth(current_index + 1).unwrap();
                output.push(Expression::Text(&current_input[char_pos..char_pos + c.len_utf8()]));
                // Remove the parsed part from current_input
                current_input = &current_input[char_pos + c.len_utf8()..];
                found_directive = true;
                break;
            }

            match directive(&current_input[current_index..]) {
                Ok((remaining, parsed)) => {
                    let leading_text = &current_input[0..current_index];
                    if !leading_text.is_empty() {
                        output.push(Expression::Text(leading_text));
                    }
                    output.push(parsed);

                    current_input = remaining;
                    found_directive = true;
                    break;
                }
                Err(nom::Err::Error(_)) => {
                    // None of the parsers matched at the current position, so this character is just part of the text.
                    // The iterator will go to the next character so there's nothing to do here.
                }
                Err(e) => {
                    // On any other error, just return the error.
                    return Err(e);
                }
            }
        }

        if !found_directive {
            output.push(Expression::Text(current_input));
            break;
        }
    }

    Ok(("", output))
}

/// Parse a line of text, counting anything that doesn't match a directive as plain text.
fn parse_inline_embed(input: &str) -> IResult<&str, Vec<Expression>> {
    let mut output = Vec::with_capacity(4);

    let mut current_input = input;

    while !current_input.is_empty() {
        let mut found_directive = false;
        for (current_index, c) in current_input.char_indices() {
            // Check for shrug emote
            if c == '¯' && current_input[current_index..].starts_with(r"¯\_(ツ)_/¯") {
                let leading_text = &current_input[0..current_index];
                if !leading_text.is_empty() {
                    output.push(Expression::Text(leading_text));
                }
                // Push the shrug emote as Expression::Text
                output.push(Expression::Text(r"¯\_(ツ)_/¯"));
                // Remove the emote from current_input
                current_input = &current_input[current_index + r"¯\_(ツ)_/¯".len()..];
                found_directive = true;
                break;
            }
            // Check for backslash
            if c == '\\' && current_input[current_index..].len() > 1 {
                let leading_text = &current_input[0..current_index];
                if !leading_text.is_empty() {
                    output.push(Expression::Text(leading_text));
                }
                // Push the escaped character as Expression::Text
                let (char_pos, c) = current_input.char_indices().nth(current_index + 1).unwrap();
                output.push(Expression::Text(&current_input[char_pos..char_pos + c.len_utf8()]));
                // Remove the parsed part from current_input
                current_input = &current_input[char_pos + c.len_utf8()..];
                found_directive = true;
                break;
            }

            match embed_directive(&current_input[current_index..]) {
                Ok((remaining, parsed)) => {
                    let leading_text = &current_input[0..current_index];
                    if !leading_text.is_empty() {
                        output.push(Expression::Text(leading_text));
                    }
                    output.push(parsed);

                    current_input = remaining;
                    found_directive = true;
                    break;
                }
                Err(nom::Err::Error(_)) => {
                    // None of the parsers matched at the current position, so this character is just part of the text.
                    // The iterator will go to the next character so there's nothing to do here.
                }
                Err(e) => {
                    // On any other error, just return the error.
                    return Err(e);
                }
            }
        }

        if !found_directive {
            output.push(Expression::Text(current_input));
            break;
        }
    }

    Ok(("", output))
}

pub fn parse(input: &str) -> Result<Vec<Expression>, nom::Err<nom::error::Error<&str>>> {
    all_consuming(parse_inline)(input).map(|(_, results)| results)
}

pub fn parse_embed(input: &str) -> Result<Vec<Expression>, nom::Err<nom::error::Error<&str>>> {
    all_consuming(parse_inline_embed)(input).map(|(_, results)| results)
}
