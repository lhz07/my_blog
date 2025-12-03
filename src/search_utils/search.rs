use crate::{
    errors::CatError,
    handlers::{home_handler::FrontMatter, post_handler::extract_frontmatter},
    search_utils::{
        INDEX_DIR, STOP_WORD_FILTER_ZH,
        jieba::{self, JIEBA_ANALYZER},
    },
};
use actix_web::web;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tantivy::{
    Index, IndexReader, TantivyDocument, Term,
    collector::TopDocs,
    query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery},
    schema::{Facet, IndexRecordOption, Value},
    snippet::SnippetGenerator,
    tokenizer::{LowerCaser, RemoveLongFilter, Stemmer, StopWordFilter, TextAnalyzer},
};

#[derive(Debug, Serialize)]
pub struct SearchTerm {
    pub score: f32,
    pub fm: Arc<FrontMatter>,
    pub snippet: String,
}

#[derive(Debug)]
pub struct SearchResult<T> {
    pub count: usize,
    pub time_cost: Duration,
    pub terms: Vec<T>,
}

impl<T> Default for SearchResult<T> {
    fn default() -> Self {
        SearchResult {
            count: 0,
            time_cost: Duration::default(),
            terms: Vec::new(),
        }
    }
}

/// detect whether a token contains any CJK character
pub fn contains_cjk(s: &str) -> bool {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{Han}]+$").unwrap());
    RE.is_match(s)
}

pub fn is_cjk_or_en(s: &str) -> bool {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z\p{Han}]+$").unwrap());
    RE.is_match(s)
}

// Use lazy static to avoid reopening the index and reloading reader every time
lazy_static::lazy_static! {
    static ref INDEX: Index = {
        let index = Index::open_in_dir(INDEX_DIR).unwrap();
        index.tokenizers().register("jieba", JIEBA_ANALYZER.clone());
        index
    };
    static ref READER: IndexReader = INDEX.reader().unwrap();
}

pub async fn search_index(
    query_text: &str,
    tags: Option<&HashSet<String>>,
    limit: usize,
    offset: usize,
) -> Result<SearchResult<SearchTerm>, CatError> {
    let instant_sum = Instant::now();

    let schema = INDEX.schema();

    let content_zh = schema.get_field("content_zh")?;
    let title_field = schema.get_field("title")?;
    let path_field = schema.get_field("path")?;

    let searcher = READER.searcher();
    log::info!("{:?}", instant_sum.elapsed());

    let mut jieba_analyzer = JIEBA_ANALYZER.clone();
    let mut token_stream = jieba_analyzer.token_stream(query_text);
    let mut tokens = Vec::new();
    while let Some(token) = token_stream.next() {
        if !token.text.trim().is_empty() && is_cjk_or_en(&token.text) {
            tokens.push(token.text.to_string());
        }
    }

    if tokens.is_empty() {
        log::info!("empty query");
        return Ok(SearchResult::default());
    }
    // phrase query
    let proximity_subs = {
        let mut jieba_analyzer =
            TextAnalyzer::builder(jieba::JiebaTokenizer::with_mode(jieba::JiebaMode::Search))
                .filter(RemoveLongFilter::limit(40))
                .filter(STOP_WORD_FILTER_ZH.clone())
                .filter(Stemmer::new(tantivy::tokenizer::Language::English))
                .filter(StopWordFilter::new(tantivy::tokenizer::Language::English).unwrap())
                .filter(LowerCaser)
                .build();
        let mut token_stream = jieba_analyzer.token_stream(query_text);
        let mut tokens = Vec::new();
        while let Some(token) = token_stream.next() {
            if !token.text.trim().is_empty() && is_cjk_or_en(&token.text) {
                let term_cn = tantivy::Term::from_field_text(content_zh, &token.text);
                tokens.push(term_cn);
            }
        }
        tokens
    };

    log::info!("tokens: {:?}", tokens);
    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![];

    for tk in tokens.iter() {
        if contains_cjk(tk) {
            // Chinese token: use exact TermQuery and boost against content_zh
            let term = tantivy::Term::from_field_text(content_zh, tk);
            let term_title = tantivy::Term::from_field_text(title_field, tk);
            let query = BoostQuery::new(
                Box::new(TermQuery::new(
                    term,
                    IndexRecordOption::WithFreqsAndPositions,
                )),
                1.5,
            );
            let title_query = BoostQuery::new(
                Box::new(TermQuery::new(
                    term_title,
                    IndexRecordOption::WithFreqsAndPositions,
                )),
                2.0,
            );
            clauses.push((Occur::Should, Box::new(query)));
            clauses.push((Occur::Should, Box::new(title_query)));
        } else {
            // short token
            let term = tantivy::Term::from_field_text(content_zh, tk);
            let term_title = tantivy::Term::from_field_text(title_field, tk);
            let title_query = BoostQuery::new(
                Box::new(TermQuery::new(
                    term_title,
                    IndexRecordOption::WithFreqsAndPositions,
                )),
                2.0,
            );
            let q1 = TermQuery::new(term, IndexRecordOption::WithFreqsAndPositions);
            clauses.push((Occur::Must, Box::new(q1)));
            clauses.push((Occur::Should, Box::new(title_query)));
        }
    }

    log::info!("basic query");

    let mut boolean_query = BooleanQuery::from(clauses);
    // boost proximity matches
    if proximity_subs.len() > 1 {
        let mut proximity_query = tantivy::query::PhraseQuery::new(proximity_subs);
        proximity_query.set_slop(10); // allow some distance between terms
        let proximity_query = BoostQuery::new(Box::new(proximity_query), 5.0);
        boolean_query = BooleanQuery::from(vec![
            (Occur::Must, Box::new(boolean_query) as Box<dyn Query>),
            (Occur::Should, Box::new(proximity_query)),
        ]);
    }
    if let Some(tags) = tags {
        let tag_facet = schema.get_field("tags")?;
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(tags.len());
        for tag in tags {
            let facet = Facet::from(&format!("/{}", tag));
            let term = Term::from_facet(tag_facet, &facet);
            let tag_query = TermQuery::new(term, IndexRecordOption::Basic);
            queries.push((Occur::Must, Box::new(tag_query)));
        }
        let tag_boolean_query = BooleanQuery::from(queries);
        boolean_query = BooleanQuery::from(vec![
            (Occur::Must, Box::new(boolean_query) as Box<dyn Query>),
            (Occur::Must, Box::new(tag_boolean_query)),
        ]);
    }
    let top_docs = searcher.search(
        &boolean_query,
        &TopDocs::with_limit(limit).and_offset(offset),
    )?;

    if top_docs.is_empty() {
        log::info!("No results");
        return Ok(SearchResult::default());
    }
    let mut zh_snippet_gen =
        SnippetGenerator::create(&searcher, &boolean_query, content_zh).unwrap();
    zh_snippet_gen.set_max_num_chars(200);
    let zh_snippet_gen = Arc::new(zh_snippet_gen);
    let mut search_futures = Vec::with_capacity(top_docs.len());
    // get total matched results count
    let count = searcher.search(&boolean_query, &tantivy::collector::Count)?;
    log::info!("total matched: {}", count);
    for (score, doc_addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(doc_addr)?;
        let zh_gen = zh_snippet_gen.clone();
        let generate_snippet = move || {
            let file_name = doc
                .get_first(path_field)
                .and_then(|v| v.as_str())
                .ok_or(CatError::internal("Can not get file name"))?;

            let text_zh = doc
                .get_first(content_zh)
                .and_then(|v| v.as_str())
                .ok_or(CatError::internal("Can not get file content"))?;

            // through testing, we find that snippet is a very expensive operation
            let ins = Instant::now();
            let snippet_zh = zh_gen.snippet(text_zh);
            log::info!("Snippet gen took: {:?}", ins.elapsed());

            let fm = extract_frontmatter(file_name)?;
            let res = SearchTerm {
                score,
                fm,
                snippet: snippet_zh.to_html().trim().to_string(),
            };
            log::info!("---\nscore: {:.3} title: {}", res.score, res.fm.title);
            log::info!("HTML snippet:\n{}\n", res.snippet);
            Ok::<SearchTerm, CatError>(res)
        };
        let fut = web::block(generate_snippet);
        search_futures.push(fut);
    }
    let terms = futures::future::join_all(search_futures)
        .await
        .into_iter()
        .map(|res| res.unwrap())
        .collect::<Result<_, _>>()?;

    let duration = instant_sum.elapsed();
    log::info!("Search took: {:?}", duration);
    let search_result = SearchResult {
        time_cost: duration,
        count,
        terms,
    };
    Ok(search_result)
}
