use crate::MD_OPTIONS;
use comrak::{
    Arena,
    nodes::{AstNode, NodeValue},
    parse_document,
};
use regex::Regex;
use std::sync::LazyLock;

/// Recursively walk the AST and collect only plain text.
fn render_plain<'a>(node: &'a AstNode<'a>, output: &mut String) {
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Text(t) => output.push_str(t),
            NodeValue::Code(t) => output.push_str(&t.literal),
            NodeValue::LineBreak | NodeValue::SoftBreak => output.push('\n'),
            NodeValue::Paragraph
            | NodeValue::Heading(_)
            | NodeValue::Item(_)
            | NodeValue::BlockQuote
            | NodeValue::List(_)
            | NodeValue::Table(_)
            | NodeValue::TableRow(_)
            | NodeValue::TableCell
            | NodeValue::FootnoteDefinition(_) => {
                render_plain(child, output);
                output.push('\n');
            }
            NodeValue::CodeBlock(block) => {
                output.push_str(&block.literal);
                output.push('\n');
            }
            NodeValue::Link(_)
            | NodeValue::Image(_)
            | NodeValue::Emph
            | NodeValue::Strong
            | NodeValue::Strikethrough
            | NodeValue::Superscript
            | NodeValue::Subscript => {
                // Just render the children, ignore formatting/URLs
                render_plain(child, output);
            }
            _ => render_plain(child, output),
        }
    }
}

pub fn md_to_plain(md: &str) -> String {
    let arena = Arena::new();
    let root = parse_document(&arena, md, &MD_OPTIONS);
    let mut output = String::new();
    render_plain(root, &mut output);
    preprocess_text(&output)
}

pub fn preprocess_text(text: &str) -> String {
    static RE1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([a-zA-Z])(\p{Han})").unwrap());
    static RE2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\p{Han})([a-zA-Z])").unwrap());
    static RE3: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
    let iter1 = RE1.replace_all(text, "$1 $2");
    let iter2 = RE2.replace_all(&iter1, "$1 $2");
    RE3.replace_all(&iter2, " ").to_string()
}
