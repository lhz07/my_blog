use crate::{
    CONTEXT, errors::RespError, handlers::post_handler::SORT_BY_UPDATED_FRONTMATTERS, lock::Lock,
    timestamp::TimeStamp,
};
use actix_web::{HttpResponse, get, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tera::Tera;

const POSTS_PER_PAGE: usize = 5;

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

#[get("/")]
pub async fn index(templates: web::Data<Arc<Lock<Tera>>>) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();

    let frontmatters = SORT_BY_UPDATED_FRONTMATTERS.get();
    let page_count = frontmatters.len().div_ceil(POSTS_PER_PAGE);
    let frontmatters = frontmatters.iter().take(POSTS_PER_PAGE).collect::<Vec<_>>();
    context.insert("posts", &frontmatters);
    context.insert("page", "home");
    context.insert("current_page", &1);
    context.insert("page_count", &page_count);
    let html = templates.get().render("home.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[get("/pages/{page_num}")]
pub async fn page(
    templates: web::Data<Arc<Lock<Tera>>>,
    page_num: web::Path<usize>,
) -> Result<HttpResponse, RespError> {
    let frontmatters = SORT_BY_UPDATED_FRONTMATTERS.get();
    let page_count = frontmatters.len().div_ceil(POSTS_PER_PAGE);
    let page_num = page_num.into_inner();
    if page_num > page_count || page_num < 1 {
        return Err(RespError::NotFound);
    }
    let mut context = CONTEXT.clone();
    let frontmatters = frontmatters
        .iter()
        .skip((page_num - 1) * POSTS_PER_PAGE)
        .take(POSTS_PER_PAGE)
        .collect::<Vec<_>>();
    context.insert("posts", &frontmatters);
    context.insert("page", "home");
    context.insert("current_page", &page_num);
    context.insert("page_count", &page_count);
    let html = templates.get().render("home.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
