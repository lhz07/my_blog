use crate::{BLOG_PATH, blog_path, errors::SearchError, lock::Lock, timestamp::TimeStamp};
use comrak::options::{Extension, Render};
use ignore::{WalkBuilder, types::TypesBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, LazyLock},
};

pub static MD_OPTIONS: LazyLock<comrak::Options> = LazyLock::new(|| comrak::Options {
    extension: Extension {
        table: true,
        cjk_friendly_emphasis: true,
        strikethrough: true,
        footnotes: true,
        tasklist: true,
        underline: true,
        superscript: true,
        ..Default::default()
    },
    render: Render {
        hardbreaks: true,
        r#unsafe: true,
        ..Default::default()
    },
    ..Default::default()
});

pub fn extract_md(post_name: &str) -> Result<String, SearchError> {
    let s = fs::read_to_string(format!("{}/posts/{}/post.md", BLOG_PATH, post_name))?;
    Ok(s)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub file_name: String,
    pub description: String,
    pub posted: TimeStamp,
    pub updated: TimeStamp,
    pub tags: Vec<String>,
    pub author: String,
    pub estimated_reading_time: u32,
    pub cover_image: Option<String>,
}

pub fn find_all_frontmatters() -> Result<Vec<FrontMatter>, SearchError> {
    let mut t = TypesBuilder::new();
    t.add_defaults();
    let toml = t.select("toml").build().unwrap();
    let file_walker = WalkBuilder::new(blog_path!("/posts")).types(toml).build();
    let mut frontmatters = Vec::new();
    for entry in file_walker {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let content = fs::read_to_string(path)?;
            let fm: FrontMatter = toml::from_str(&content)?;
            frontmatters.push(fm);
        }
    }
    Ok(frontmatters)
}

pub fn extract_frontmatter(post_name: &str) -> Result<Arc<FrontMatter>, SearchError> {
    let fm = FRONTMATTER
        .get()
        .get(post_name)
        .ok_or_else(|| {
            SearchError::IO(std::io::Error::other(format!(
                "Frontmatter for post '{}' not found",
                post_name
            )))
        })?
        .clone();
    Ok(fm)
}
pub static FRONTMATTER: LazyLock<Lock<HashMap<String, Arc<FrontMatter>>>> = LazyLock::new(|| {
    let map = match initial_fm() {
        Ok(f) => f,
        Err(e) => {
            log::error!("Can not find frontmatters!, error: {e}");
            std::process::exit(1);
        }
    };
    Lock::new(map)
});

pub fn initial_fm() -> Result<HashMap<String, Arc<FrontMatter>>, SearchError> {
    let fm = find_all_frontmatters()?;
    let map = fm
        .into_iter()
        .map(|f| (f.file_name.clone(), Arc::new(f)))
        .collect::<HashMap<_, _>>();
    Ok(map)
}
