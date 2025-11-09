use crate::{
    CONTEXT,
    errors::RespError,
    handlers::post_handler::SORT_BY_POSTED_FRONTMATTERS,
    lock::Lock,
    search_utils::search::{search_index, search_tags},
};
use actix_web::{HttpResponse, get, web};
use rand::seq::IndexedRandom;
use serde::{
    Deserialize,
    de::{self},
};
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};
use tera::Tera;

#[derive(Debug, Deserialize)]
struct QueryParam {
    #[serde(deserialize_with = "deserialize_tags", default)]
    tag: Option<HashSet<String>>,
    #[serde(deserialize_with = "deserialize_str", default)]
    q: Option<String>,
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Option<HashSet<String>>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let res = Option::<String>::deserialize(deserializer)?;
    let s = match res {
        Some(s) => s,
        None => return Ok(None),
    };
    let params = s.split(',');
    let mut tags = HashSet::new();
    for tag in params {
        tags.insert(tag.to_lowercase());
    }
    if tags.is_empty() {
        return Ok(None);
    }
    Ok(Some(tags))
}

fn deserialize_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let res = Option::<String>::deserialize(deserializer)?;
    match res {
        Some(s) if !s.is_empty() => Ok(Some(s)),
        _ => Ok(None),
    }
}

static ALL_TAGS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let frontmatters = SORT_BY_POSTED_FRONTMATTERS.get();
    let mut tags = HashSet::new();
    for fm in frontmatters.iter() {
        tags.extend(fm.tags.clone());
    }
    let mut tags = tags.into_iter().collect::<Vec<_>>();
    tags.sort_by_key(|t| t.to_lowercase());
    tags
});

fn handle_tag(
    templates: web::Data<Arc<Lock<Tera>>>,
    tags: HashSet<String>,
) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    context.insert("selected_tags", &tags);
    log::info!("{:?}", tags);
    let result = search_tags(tags, 10, 0)?;
    // let frontmatters = find_all_frontmatters().inspect_err(|e| eprintln!("{e}"))?;
    // let mut filtered_posts = filter_tags(tags, frontmatters);
    // filtered_posts.sort_by(|a, b| b.posted.cmp(&a.posted));
    // println!("{:?}", filtered_posts);
    let time_cost = result.time_cost.as_secs_f64();
    let time_cost = (time_cost * 10000.0).round() / 10000.0;
    context.insert("tag_result", &result.terms);
    context.insert("matched_count", &result.count);
    context.insert("time_cost", &time_cost);
    context.insert("page", "search");

    context.insert("all_tags", &*ALL_TAGS);
    context.insert("current_page", &1);
    context.insert("page_count", &1);
    // context.insert("search_tags", );
    let html = templates.get().render("search_text.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn handle_query_text(
    templates: web::Data<Arc<Lock<Tera>>>,
    query_text: String,
    tags: Option<HashSet<String>>,
) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    if let Some(tags) = &tags {
        context.insert("selected_tags", tags);
    }
    log::info!("{}", query_text);
    let search_result = search_index(&query_text, tags, 10, 0)
        .await
        .inspect_err(|e| eprintln!("{e}"))?;
    let time_cost = search_result.time_cost.as_secs_f64();
    let time_cost = (time_cost * 1000.0).round() / 1000.0;
    context.insert("search_result", &search_result.terms);
    context.insert("matched_count", &search_result.count);
    context.insert("time_cost", &time_cost);
    context.insert("query", &query_text);
    context.insert("page", "search");

    context.insert("all_tags", &*ALL_TAGS);
    context.insert("current_page", &1);
    context.insert("page_count", &1);
    let html = templates.get().render("search_text.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[get("/search")]
pub async fn search(
    templates: web::Data<Arc<Lock<Tera>>>,
    query: web::Query<QueryParam>,
) -> Result<HttpResponse, RespError> {
    log::info!("query: {:?}", query);
    match (query.0.tag, query.0.q) {
        (Some(tags), None) => handle_tag(templates, tags),
        (tags, Some(query_text)) => handle_query_text(templates, query_text, tags).await,
        (None, None) => {
            let mut context = CONTEXT.clone();
            context.insert("page", "search");
            context.insert("all_tags", &*ALL_TAGS);
            let html = templates.get().render("search_text.html", &context)?;
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
    }
}

#[get("/lucky")]
pub async fn search_lucky(query: web::Query<QueryParam>) -> Result<HttpResponse, RespError> {
    match (query.0.tag, query.0.q) {
        (Some(tags), None) => {
            let result = search_tags(tags, 10, 0)?;
            let mut rng = rand::rng();
            match result.terms.choose(&mut rng) {
                Some(luck) => Ok(HttpResponse::Found()
                    .append_header(("Location", format!("/posts/{}", luck.file_name)))
                    .finish()),
                None => Ok(HttpResponse::Found()
                    .append_header(("Location", "/search"))
                    .finish()),
            }
        }
        (tags, Some(query_text)) => {
            let search_result = search_index(&query_text, tags, 10, 0)
                .await
                .inspect_err(|e| eprintln!("{e}"))?;
            match search_result.terms.first() {
                Some(first) => Ok(HttpResponse::Found()
                    .append_header(("Location", format!("/posts/{}", first.fm.file_name)))
                    .finish()),
                None => Ok(HttpResponse::Found()
                    .append_header(("Location", "/search"))
                    .finish()),
            }
        }
        (None, None) => Ok(HttpResponse::Found()
            .append_header(("Location", "/search"))
            .finish()),
    }
}
