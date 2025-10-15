use std::fs;

use tantivy::tokenizer::StopWordFilter;

pub mod build_index;
pub mod cleaner;
pub mod jieba;
pub mod search;

lazy_static::lazy_static! {
    static ref STOP_WORD_FILTER_ZH: StopWordFilter = {
        let file = fs::read_to_string("./search/cn_stopwords.txt").unwrap();
        let words = file.lines().map(|s| s.to_string()).collect::<Vec<_>>();
        StopWordFilter::remove(words)
    };
}

pub const INDEX_DIR: &str = "./search/data";
