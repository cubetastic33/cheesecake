use html_escape;
use crate::parser::Expression;

trait Callback: Fn(&str) -> (String, Option<String>) {}

impl<T: Fn(&str) -> (String, Option<String>)> Callback for T {}

// Store all the callbacks in a struct so we can pass it around easily during recursion
struct Callbacks<A, B, C, D> {
    emoji: A,
    user: B,
    role: C,
    channel: D,
}

// Generates HTML from the AST
fn traverse(ast: Vec<Expression>, callbacks: &Callbacks<impl Callback, impl Callback, impl Callback, impl Callback>, first: bool) -> String {
    // String to store the final HTML
    let mut final_html = String::new();
    // Wumboji
    let mut wumboji = " wumboji";
    // Don't do this if we've started recursion
    if first {
        // If there is any text other than whitespace, don't wumboji
        for expression in &ast {
            match expression {
                Expression::CustomEmoji(_, _) /*| Expression::Emoji(_)*/ => {}
                Expression::Text(text) => {
                    if !text.chars().all(char::is_whitespace) {
                        wumboji = "";
                        break;
                    }
                }
                _ => {
                    wumboji = "";
                    break;
                }
            }
        }
    }
    for expression in ast {
        let html = match expression {
            Expression::Text(text) => format!("<span>{}</span>", html_escape::encode_text(&text.to_string()).to_string()), // Escape HTML
            Expression::CustomEmoji(name, id) => {
                // Use user-provided callback to get emoji path
                let path = (callbacks.emoji)(&id).0;
                format!("<img src=\"{0}\" alt=\"{1}\" class=\"emoji{2}\" title=\"{1}\"></img>", path, name, wumboji)
            }
            // Expression::Emoji(emoji) => format!("<span class=\"emoji{}\">{}</span>", wumboji, emoji),
            Expression::User(id) => format!("<span class=\"user\">@{}</span>", (callbacks.user)(id).0),
            Expression::Role(id) => {
                let (name, color) = (callbacks.role)(id);
                format!(
                    "<div class=\"role\" style=\"color: {0}\">@{1}<span style=\"background-color: {0}\"></span></div>",
                    color.unwrap_or(String::from("#afafaf")),
                    name,
                )
            },
            Expression::Channel(id) => format!("<span class=\"channel\" data-id=\"{}\">#{}</span>", id, (callbacks.channel)(id).0),
            Expression::Hyperlink(text, href) => format!("<a href=\"{}\" target=\"_blank\">{}</a>", href, text),
            Expression::MultilineCode(text) => format!("<pre class=\"multiline_code\">{}</pre>", text),
            Expression::InlineCode(text) => format!("<span class=\"inline_code\">{}</span>", text),
            Expression::Blockquote(a) => format!("<blockquote>{}</blockquote>", traverse(a, callbacks, false)),
            Expression::Spoiler(a) => format!("<span class=\"spoiler\">{}</span>", traverse(a, callbacks, false)),
            Expression::Underline(a) => format!("<span class=\"underline\">{}</span>", traverse(a, callbacks, false)),
            Expression::Strikethrough(a) => format!("<span class=\"strikethrough\">{}</span>", traverse(a, callbacks, false)),
            Expression::Bold(a) => format!("<strong>{}</strong>", traverse(a, callbacks, false)),
            Expression::Italics(a) => format!("<em>{}</em>", traverse(a, callbacks, false)),
        };
        final_html.push_str(&html.replace("\n", "<br>"));
    }
    final_html
}

// Wrapper function for traverse
pub fn to_html(
    ast: Vec<Expression>,
    emoji: impl Fn(&str) -> (String, Option<String>),
    user: impl Fn(&str) -> (String, Option<String>),
    role: impl Fn(&str) -> (String, Option<String>),
    channel: impl Fn(&str) -> (String, Option<String>),
) -> String {
    traverse(ast, &Callbacks {
        emoji,
        user,
        role,
        channel,
    }, true)
}
