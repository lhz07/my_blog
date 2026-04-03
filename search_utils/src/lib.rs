use std::{collections::HashSet, fs, sync::LazyLock};

use const_format::formatcp;

pub mod build_index;
pub mod cleaner;
pub mod errors;
pub mod jieba;
pub mod lock;
pub mod post;
pub mod search;
pub mod timestamp;

const SEARCH_PATH: &str = "./search_utils";
pub const BLOG_PATH: &str = "./blog";

#[macro_export]
macro_rules! blog_path {
    ($name:literal) => {
        const_format::formatcp!("{}{}", $crate::BLOG_PATH, $name)
    };
}

static STOP_WORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let file = fs::read_to_string(formatcp!("{}/search/cn_stopwords.txt", SEARCH_PATH)).unwrap();
    file.lines().map(|s| s.to_string()).collect()
});

pub const INDEX_DIR: &str = formatcp!("{}/search/data", SEARCH_PATH);
