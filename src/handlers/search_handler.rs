use crate::{
    CONTEXT,
    errors::RespError,
    handlers::{
        home_handler::{FrontMatter, PageUtil},
        post_handler::{SORT_BY_POSTED_FRONTMATTERS, SORT_BY_UPDATED_FRONTMATTERS},
    },
    lock::Lock,
    search_utils::search::search_index,
};
use actix_web::{HttpRequest, HttpResponse, route, web};
use rand::seq::IndexedRandom;
use serde::{Deserialize, de};
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::{Arc, LazyLock},
};
use tera::Tera;

const SEARCH_RESULTS_PER_PAGE: usize = 7;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryParam {
    #[serde(deserialize_with = "deserialize_tags", default)]
    tag: Option<HashSet<String>>,
    /// q is guaranteed to be not an empty string
    #[serde(deserialize_with = "deserialize_str", default)]
    q: Option<String>,
    page: Option<usize>,
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

fn filter_tags(tags: &HashSet<String>) -> Vec<Arc<FrontMatter>> {
    let fm = SORT_BY_UPDATED_FRONTMATTERS.get();
    fm.iter()
        .filter(|fm| {
            // TODO: cache the tags in lowercase?
            let fm_tags = fm
                .tags
                .iter()
                .map(|t| t.to_lowercase())
                .collect::<HashSet<_>>();
            tags.is_subset(&fm_tags)
        })
        .cloned()
        .collect()
}

fn handle_tag(
    templates: web::Data<Arc<Lock<Tera>>>,
    tags: HashSet<String>,
    page: usize,
    query_param: Cow<'static, str>,
) -> Result<HttpResponse, RespError> {
    if page < 1 {
        return Err(RespError::NotFound);
    }
    let mut context = CONTEXT.clone();
    context.insert("selected_tags", &tags);
    log::info!("{:?}", tags);
    let ins = std::time::Instant::now();
    // search by only tags is unstable, their order is not guaranteed
    let result = filter_tags(&tags);
    let page_count = result.len().div_ceil(SEARCH_RESULTS_PER_PAGE);
    if page > page_count {
        if page_count > 0 {
            return Err(RespError::NotFound);
        }
    } else {
        let time_cost = ins.elapsed().as_secs_f64();
        let time_cost = (time_cost * 10000.0).round() / 10000.0;
        PageUtil::insert(&mut context, page, page_count);
        context.insert("matched_count", &result.len());
        context.insert("time_cost", &time_cost);
    }

    let render_result = &result
        [(page - 1) * SEARCH_RESULTS_PER_PAGE..(page * SEARCH_RESULTS_PER_PAGE).min(result.len())];

    context.insert("tag_result", &render_result);
    context.insert("page", "search");
    context.insert("query_param", &query_param);
    context.insert("all_tags", &*ALL_TAGS);
    let html = templates.get().render("search_text.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn handle_query_text(
    templates: web::Data<Arc<Lock<Tera>>>,
    query_text: String,
    tags: Option<HashSet<String>>,
    page: usize,
    query_param: Cow<'static, str>,
) -> Result<HttpResponse, RespError> {
    if page < 1 {
        return Err(RespError::NotFound);
    }
    let mut context = CONTEXT.clone();
    if let Some(tags) = &tags {
        context.insert("selected_tags", tags);
    }
    let search_result = search_index(
        &query_text,
        tags.as_ref(),
        SEARCH_RESULTS_PER_PAGE,
        (page - 1) * SEARCH_RESULTS_PER_PAGE,
    )
    .await
    .inspect_err(|e| eprintln!("{e}"))?;

    let time_cost = search_result.time_cost.as_secs_f64();
    let time_cost = (time_cost * 1000.0).round() / 1000.0;

    let page_count = search_result.count.div_ceil(SEARCH_RESULTS_PER_PAGE);
    if page > page_count {
        if page_count > 0 {
            return Err(RespError::NotFound);
        }
    } else {
        let time_cost = (time_cost * 10000.0).round() / 10000.0;
        PageUtil::insert(&mut context, page, page_count);
        context.insert("matched_count", &search_result.count);
        context.insert("time_cost", &time_cost);
    }

    context.insert("query", &query_text);
    context.insert("page", "search");
    context.insert("search_result", &search_result.terms);
    context.insert("query_param", &query_param);
    context.insert("all_tags", &*ALL_TAGS);

    let html = templates.get().render("search_text.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn search_inner(
    templates: web::Data<Arc<Lock<Tera>>>,
    query: web::Query<QueryParam>,
    request: HttpRequest,
) -> Result<HttpResponse, RespError> {
    log::info!("query: {:?}", query);
    let page = match query.page {
        Some(p) => {
            if let (None, None) = (&query.0.q, &query.0.tag) {
                return Err(RespError::BadRequest);
            } else {
                p
            }
        }
        None => 1,
    };
    let url = request.full_url();
    let query_param = match url.query() {
        Some(_) => {
            let mut new_url = url.clone();
            let mut query_pairs = new_url.query_pairs_mut();
            query_pairs.clear();
            for (k, v) in url.query_pairs() {
                if k != "page" {
                    query_pairs.append_pair(&k, &v);
                }
            }
            Cow::from(query_pairs.finish().query().unwrap().to_string())
        }
        None => Cow::from(""),
    };

    match (query.0.tag, query.0.q) {
        (Some(tags), None) => handle_tag(templates, tags, page, query_param),
        (tags, Some(query_text)) => {
            handle_query_text(templates, query_text, tags, page, query_param).await
        }
        (None, None) => {
            let mut context = CONTEXT.clone();
            context.insert("page", "search");
            context.insert("all_tags", &*ALL_TAGS);
            let html = templates.get().render("search_text.html", &context)?;
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
    }
}

#[route("/search", method = "GET", method = "HEAD")]
pub async fn search(
    templates: web::Data<Arc<Lock<Tera>>>,
    query: web::Query<QueryParam>,
    request: HttpRequest,
) -> Result<HttpResponse, RespError> {
    search_inner(templates, query, request).await
}

#[route("/lucky", method = "GET", method = "HEAD")]
pub async fn search_lucky(
    query: web::Query<QueryParam>,
    templates: web::Data<Arc<Lock<Tera>>>,
    request: HttpRequest,
) -> Result<HttpResponse, RespError> {
    match (&query.0.tag, &query.0.q) {
        (Some(tags), None) => {
            let result = filter_tags(tags);
            let mut rng = rand::rng();
            match result.choose(&mut rng) {
                Some(luck) => Ok(HttpResponse::Found()
                    .append_header(("Location", format!("/posts/{}", luck.file_name)))
                    .finish()),
                None => search_inner(templates, query, request).await,
            }
        }
        (tags, Some(query_text)) => {
            let search_result = search_index(&query_text, tags.as_ref(), 1, 0)
                .await
                .inspect_err(|e| eprintln!("{e}"))?;
            match search_result.terms.first() {
                Some(first) => Ok(HttpResponse::Found()
                    .append_header(("Location", format!("/posts/{}", first.fm.file_name)))
                    .finish()),
                None => search_inner(templates, query, request).await,
            }
        }
        (None, None) => Ok(HttpResponse::Found()
            .append_header(("Location", "/search"))
            .finish()),
    }
}
