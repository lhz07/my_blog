use std::{collections::HashSet, fs, sync::LazyLock};

pub mod build_index;
pub mod cleaner;
pub mod jieba;
pub mod search;

static STOP_WORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let file = fs::read_to_string("./search/cn_stopwords.txt").unwrap();
    file.lines().map(|s| s.to_string()).collect()
});

pub const INDEX_DIR: &str = "./search/data";
