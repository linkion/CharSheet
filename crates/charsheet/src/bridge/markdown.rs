//! Markdown for one-line summaries. Multi-line text renders with Qt's own Markdown support
//! (Text.MarkdownText), but that can't be elided, so rows that truncate show the first block
//! as Qt's StyledText subset instead.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Markdown = super::MarkdownRust;

        /// The first block of some Markdown as StyledText: bold, italic, strikethrough,
        /// code and links, on one line.
        #[qinvokable]
        #[cxx_name = "toLine"]
        fn to_line(self: &Self, text: &QString) -> QString;
    }
}

use cxx_qt_lib::QString;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Default)]
pub struct MarkdownRust;

impl qobject::Markdown {
    fn to_line(&self, text: &QString) -> QString {
        QString::from(&inline_html(&text.to_string()))
    }
}

/// Converts the first block of `text` to StyledText. Headings and lists are flattened to
/// their text, and any HTML in the source shows literally.
pub fn inline_html(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    let mut items = 0;
    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Start(tag) => {
                if is_block(&tag) {
                    depth += 1;
                }
                match tag {
                    Tag::Item => {
                        out.push_str(if items == 0 { "• " } else { "  • " });
                        items += 1;
                    }
                    Tag::Emphasis => out.push_str("<i>"),
                    Tag::Strong => out.push_str("<b>"),
                    Tag::Strikethrough => out.push_str("<s>"),
                    Tag::Link { dest_url, .. } => {
                        out.push_str("<a href=\"");
                        out.push_str(&escape(&dest_url));
                        out.push_str("\">");
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Emphasis => out.push_str("</i>"),
                    TagEnd::Strong => out.push_str("</b>"),
                    TagEnd::Strikethrough => out.push_str("</s>"),
                    TagEnd::Link => out.push_str("</a>"),
                    _ => {}
                }
                if is_block_end(&tag) {
                    depth -= 1;
                    if depth == 0 && !out.is_empty() {
                        break;
                    }
                }
            }
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => out.push_str(&escape(&t)),
            Event::Code(t) => {
                out.push_str("<code>");
                out.push_str(&escape(&t));
                out.push_str("</code>");
            }
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out.trim_end().to_owned()
}

fn is_block(tag: &Tag) -> bool {
    !matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn is_block_end(tag: &TagEnd) -> bool {
    !matches!(
        tag,
        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link | TagEnd::Image
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::inline_html;

    #[test]
    fn inline_styles() {
        assert_eq!(
            inline_html("**Heavy**, *two-handed* ~~light~~ `1d8` [SRD](https://x.y/?a=1&b=2)"),
            "<b>Heavy</b>, <i>two-handed</i> <s>light</s> <code>1d8</code> \
             <a href=\"https://x.y/?a=1&amp;b=2\">SRD</a>"
        );
    }

    #[test]
    fn escapes_html() {
        assert_eq!(
            inline_html("a < b & <b>c</b>"),
            "a &lt; b &amp; &lt;b&gt;c&lt;/b&gt;"
        );
        assert_eq!(inline_html("<div>x</div>"), "&lt;div&gt;x&lt;/div&gt;");
    }

    #[test]
    fn first_block_only() {
        assert_eq!(inline_html("One\nline\n\nSecond paragraph"), "One line");
        assert_eq!(inline_html("# Title\n\nBody"), "Title");
    }

    #[test]
    fn lists_flatten() {
        assert_eq!(
            inline_html("- one\n- **two**\n\nafter"),
            "• one  • <b>two</b>"
        );
        assert_eq!(inline_html("1. a\n\n2. b"), "• a  • b");
    }

    #[test]
    fn empty() {
        assert_eq!(inline_html(""), "");
        assert_eq!(inline_html("\n\n"), "");
    }
}
