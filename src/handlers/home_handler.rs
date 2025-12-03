use crate::{
    CONTEXT, errors::RespError, handlers::post_handler::SORT_BY_UPDATED_FRONTMATTERS, lock::Lock,
    timestamp::TimeStamp,
};
use actix_web::{HttpResponse, route, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tera::{Context, Tera};

const POSTS_PER_PAGE: usize = 5;

pub struct PageUtil {
    pub start: usize,
    pub end: usize,
    pub skip_back: usize,
    pub skip_forward: usize,
}

impl PageUtil {
    const HALF: usize = 1;
    pub fn insert(context: &mut Context, page_num: usize, page_count: usize) {
        let half = Self::HALF;
        let neighbors = half * 2;

        let mut left = half;
        let mut right = half;

        if page_num <= half {
            left = page_num - 1;
            right = neighbors - left;
        } else if page_num + half > page_count {
            right = page_count - page_num;
            left = neighbors - right;
        }
        let start = 2.max(page_num.saturating_sub(left));
        let end = (page_count - 1).min(page_num + right);
        let skip_back = 1.max(start.saturating_sub(2));
        let skip_forward = page_count.min(end + 2);
        context.insert("current_page", &page_num);
        context.insert("page_count", &page_count);
        context.insert("start_page", &start);
        context.insert("end_page", &end);
        context.insert("skip_back", &skip_back);
        context.insert("skip_forward", &skip_forward);
    }
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

pub async fn render_page(
    templates: web::Data<Arc<Lock<Tera>>>,
    page_num: usize,
) -> Result<HttpResponse, RespError> {
    let frontmatters = SORT_BY_UPDATED_FRONTMATTERS.get();
    let page_count = frontmatters.len().div_ceil(POSTS_PER_PAGE);
    if page_num > page_count || page_num < 1 {
        return Err(RespError::NotFound);
    }
    let mut context = CONTEXT.clone();
    let fms = frontmatters
        .iter()
        .skip((page_num - 1) * POSTS_PER_PAGE)
        .take(POSTS_PER_PAGE)
        .collect::<Vec<_>>();

    PageUtil::insert(&mut context, page_num, page_count);
    context.insert("posts", &fms);
    context.insert("page", "home");
    let html = templates.get().render("home.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[route("/", method = "GET", method = "HEAD")]
pub async fn index(templates: web::Data<Arc<Lock<Tera>>>) -> Result<HttpResponse, RespError> {
    render_page(templates, 1).await
}

#[route("/pages/{page_num}", method = "GET", method = "HEAD")]
pub async fn page(
    templates: web::Data<Arc<Lock<Tera>>>,
    page_num: web::Path<usize>,
) -> Result<HttpResponse, RespError> {
    let page_num = page_num.into_inner();
    render_page(templates, page_num).await
}
