use crate::{
    errors::SearchError,
    post::{MD_OPTIONS, find_all_post_paths},
};
use comrak::{
    Arena,
    arena_tree::Node,
    format_commonmark,
    nodes::{Ast, NodeValue},
    parse_document,
};
use regex::Regex;
use std::{
    borrow::Cow,
    cell::RefCell,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::LazyLock,
};

static YEAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b20\d{2}\b").unwrap());

fn recursion_nodes<'a, F>(root: &'a Node<'a, RefCell<Ast>>, f: &F) -> Result<(), String>
where
    F: Fn(&Node<'_, RefCell<Ast>>) -> Result<(), String>,
{
    f(root)?;
    for child in root.children() {
        recursion_nodes(child, f)?;
    }
    Ok(())
}

pub trait ShorterPath {
    fn shorter_path(&self) -> &Path;
}

impl ShorterPath for Path {
    fn shorter_path(&self) -> &Path {
        match self.strip_prefix(std::env::current_dir().unwrap_or_default()) {
            Ok(p) => p,
            Err(_) => self,
        }
    }
}

pub fn format_all() -> Result<(), SearchError> {
    let paths = find_all_post_paths()?;
    let mut err = false;
    for path in paths {
        if let Err(e) = format_md_file(&path) {
            eprintln!("can not format file {}: {e}", path.shorter_path().display());
            err = true;
        }
    }
    if err {
        Err(SearchError::Internal("format error".into()))
    } else {
        Ok(())
    }
}

pub fn format_md_file(path: &Path) -> Result<(), String> {
    let md = fs::read_to_string(path).map_err(|e| format!("read md: {e}"))?;
    let res = format_md(&md)?;
    fs::write(path, res).map_err(|e| format!("write formatted md: {e}"))
}

pub fn format_md(md: &str) -> Result<String, String> {
    let arena = Arena::new();

    let root = parse_document(&arena, md, &MD_OPTIONS);

    recursion_nodes(root, &|node| {
        let mut data = node.data.borrow_mut();
        if let NodeValue::CodeBlock(ref mut cb) = data.value {
            let info = cb.info.trim().to_ascii_lowercase();
            if info == "rust" {
                let formatted = match format_code(&cb.literal) {
                    Ok(c) => c,
                    Err(e) => return Err(e),
                };
                match formatted {
                    Cow::Borrowed(_) => (),
                    Cow::Owned(s) => cb.literal = s,
                }
            }
        }
        Ok(())
    })?;

    let mut output = String::with_capacity(md.len());
    format_commonmark(root, &MD_OPTIONS, &mut output).map_err(|e| e.to_string())?;
    Ok(output)
}

pub fn format_code(code: &str) -> Result<Cow<'_, str>, String> {
    let Some(first_line) = code.lines().next() else {
        return Ok(String::new().into());
    };
    let mut edition = "2024";
    if let Some(("", comment)) = first_line.trim().split_once("//") {
        for token in comment.trim().split_ascii_whitespace() {
            if YEAR_RE.is_match(token) {
                edition = token;
                break;
            } else if token == "DONT_FORMAT" {
                return Ok(code.into());
            }
        }
    }
    let mut child = Command::new("rustfmt")
        .args(["--emit", "stdout", "--edition", edition])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(code.as_bytes())
        .map_err(|e| e.to_string())?;
    drop(stdin);

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned().into())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

#[test]
fn test_fmt() {
    let code = include_str!("formatter.rs");
    let formatted = format_code(code).unwrap();
    println!("{}", formatted)
}

#[test]
fn test_fmt_md() {
    let md = std::fs::read_to_string("../blog/posts/implement-retain_mut_value/post.md").unwrap();
    let formatted = format_md(&md).unwrap();
    println!("{}", formatted)
}
