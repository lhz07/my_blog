use crate::{
    errors::CatError,
    handlers::{home_handler::find_all_frontmatters, post_handler::extract_md},
    search_utils::{
        INDEX_DIR,
        cleaner::{md_to_plain, preprocess_text},
        jieba::JIEBA_ANALYZER,
    },
};
use std::{fs, path::Path};
use tantivy::{
    Index, TantivyDocument,
    schema::{
        Facet, FacetOptions, IndexRecordOption, STORED, Schema, TextFieldIndexing, TextOptions,
    },
};

/// Create schema, register tokenizers, and index .txt files.
pub fn build_index() -> Result<(), CatError> {
    // schema
    let mut schema_builder = Schema::builder();

    // prepare indexing options per-field with tokenizer name
    let zh_indexing = TextFieldIndexing::default()
        .set_tokenizer("jieba")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);

    let text_options_zh = TextOptions::default()
        .set_indexing_options(zh_indexing)
        .set_stored();

    let content_zh = schema_builder.add_text_field("content_zh", text_options_zh.clone());
    let title_field = schema_builder.add_text_field("title", text_options_zh);
    let tag_facet = schema_builder.add_facet_field("tags", FacetOptions::default());
    let path_field = schema_builder.add_text_field("path", STORED);

    let schema = schema_builder.build();

    // create index folder (overwrite if exists)
    let index_path = Path::new(INDEX_DIR);
    let processed_path = index_path.join("processed_text");
    if index_path.exists() && index_path.is_dir() {
        fs::remove_dir_all(index_path)?;
    }
    fs::create_dir_all(index_path)?;
    fs::create_dir(&processed_path)?;

    let index = Index::create_in_dir(index_path, schema.clone())?;

    // Register jieba tokenizer for Chinese
    index.tokenizers().register("jieba", JIEBA_ANALYZER.clone());

    // writer
    let mut writer = index.writer(50_000_000)?;
    let fms = find_all_frontmatters()?;
    for fm in fms {
        let content = extract_md(&fm.file_name)?;
        let text = md_to_plain(&content);
        let description = preprocess_text(&fm.description);
        let text = format!("{} {}", description, text);
        // save processed text for debugging
        fs::write(
            processed_path.join(&fm.file_name).with_extension("txt"),
            &text,
        )?;
        let mut doc = TantivyDocument::default();
        for tag in fm.tags.iter() {
            let facet = Facet::from(&format!("/{}", tag.to_lowercase()));
            doc.add_facet(tag_facet, facet);
        }
        // store the same raw text into all analysis fields: per-field tokenizer will break it differently
        doc.add_text(content_zh, &text);
        doc.add_text(title_field, &fm.title);
        doc.add_text(path_field, &fm.file_name);
        writer.add_document(doc)?;
    }

    writer.commit()?;
    println!("Index built at '{}'", INDEX_DIR);
    Ok(())
}
