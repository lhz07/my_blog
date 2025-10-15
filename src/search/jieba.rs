use once_cell::sync::Lazy;
use tantivy::tokenizer::{
    LowerCaser, RemoveLongFilter, Stemmer, StopWordFilter, TextAnalyzer, Token, TokenStream,
    Tokenizer,
};

use crate::search::STOP_WORD_FILTER_ZH;

pub trait JiebaTokenize {
    fn basic_tokenize<'a>(words: Vec<&'a str>) -> Vec<jieba_rs::Token<'a>> {
        let mut tokens = Vec::with_capacity(words.len());
        let mut start = 0;
        for word in words {
            let width = word.chars().count();
            tokens.push(jieba_rs::Token {
                word,
                start,
                end: start + width,
            });
            start += width;
        }
        tokens
    }
    fn tokenize_all<'a>(&self, sentence: &'a str) -> Vec<jieba_rs::Token<'a>>;
    fn tokenize_default<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<jieba_rs::Token<'a>>;
    fn tokenize_search<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<jieba_rs::Token<'a>>;
}

impl JiebaTokenize for jieba_rs::Jieba {
    fn tokenize_all<'a>(&self, sentence: &'a str) -> Vec<jieba_rs::Token<'a>> {
        let words = self.cut_all(sentence);
        Self::basic_tokenize(words)
    }
    fn tokenize_default<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<jieba_rs::Token<'a>> {
        let words = self.cut(sentence, hmm);
        Self::basic_tokenize(words)
    }
    fn tokenize_search<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<jieba_rs::Token<'a>> {
        let words = self.cut_for_search(sentence, hmm);
        Self::basic_tokenize(words)
    }
}

pub struct JiebaTokenStream<'str> {
    text: &'str str,
    jieba_tokens: Vec<jieba_rs::Token<'str>>,
    index: usize,
    token: Token,
}

impl TokenStream for JiebaTokenStream<'_> {
    fn advance(&mut self) -> bool {
        if self.index >= self.jieba_tokens.len() {
            return false;
        }
        let jieba_token = &self.jieba_tokens[self.index];
        self.token.offset_from = jieba_token.word.as_ptr() as usize - self.text.as_ptr() as usize;
        self.token.offset_to = self.token.offset_from + jieba_token.word.len();
        self.token.position = jieba_token.start;
        self.token.position_length = jieba_token.end - jieba_token.start;
        self.token.text.clear(); // avoid realloc
        self.token.text.push_str(jieba_token.word);
        self.index += 1;
        true
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

#[derive(Clone, Copy)]
pub enum JiebaMode {
    Default,
    CutAll,
    Search,
}

#[derive(Clone, Copy)]
pub struct JiebaTokenizer {
    hmm: bool,
    mode: JiebaMode,
}

impl Default for JiebaTokenizer {
    fn default() -> Self {
        Self {
            hmm: false,
            mode: JiebaMode::Default,
        }
    }
}

impl JiebaTokenizer {
    pub fn new(mode: JiebaMode, hmm: bool) -> Self {
        Self { hmm, mode }
    }
    pub fn with_mode(mode: JiebaMode) -> Self {
        Self { mode, hmm: false }
    }
    pub fn set_hmm(&mut self, hmm: bool) {
        self.hmm = hmm;
    }
    pub fn set_mode(&mut self, mode: JiebaMode) {
        self.mode = mode;
    }
}

pub static JIEBA: Lazy<jieba_rs::Jieba> = Lazy::new(|| jieba_rs::Jieba::new());

pub static JIEBA_ANALYZER: Lazy<TextAnalyzer> = Lazy::new(|| {
    tantivy::tokenizer::TextAnalyzer::builder(JiebaTokenizer::with_mode(JiebaMode::CutAll))
        .filter(RemoveLongFilter::limit(40))
        .filter(STOP_WORD_FILTER_ZH.clone())
        .filter(Stemmer::new(tantivy::tokenizer::Language::English))
        .filter(StopWordFilter::new(tantivy::tokenizer::Language::English).unwrap())
        .filter(LowerCaser)
        .build()
});

impl Tokenizer for JiebaTokenizer {
    type TokenStream<'str> = JiebaTokenStream<'str>;

    fn token_stream<'str>(&mut self, text: &'str str) -> JiebaTokenStream<'str> {
        let jieba_tokens = match self.mode {
            JiebaMode::Default => JIEBA.tokenize_default(text, self.hmm),
            JiebaMode::CutAll => JIEBA.tokenize_all(text),
            JiebaMode::Search => JIEBA.tokenize_search(text, self.hmm),
        };
        let token = jieba_tokens
            .first()
            .map(|token| Token {
                offset_from: token.word.as_ptr() as usize - text.as_ptr() as usize,
                offset_to: token.word.as_ptr() as usize - text.as_ptr() as usize + token.word.len(),
                text: token.word.to_string(),
                position: token.start,
                position_length: token.end - token.start,
            })
            .unwrap_or_default();
        JiebaTokenStream {
            text,
            jieba_tokens,
            index: 0,
            token,
        }
    }
}
