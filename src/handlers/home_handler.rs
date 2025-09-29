use crate::{
    CONTEXT,
    errors::{CatError, RespError},
    timestamp::TimeStamp,
};
use actix_web::{HttpResponse, get, web};
use ignore::{WalkBuilder, types::TypesBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use tera::Tera;

const POSTS_PER_PAGE: usize = 5;

#[derive(Serialize, Deserialize)]
pub struct FrontMatter {
    title: String,
    file_name: String,
    description: String,
    posted: TimeStamp,
    tags: Vec<String>,
    author: String,
    estimated_reading_time: u32,
    cover_image: Option<String>,
}

#[get("/")]
pub async fn index(templates: web::Data<Tera>) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();

    let mut frontmatters = find_all_frontmatters().inspect_err(|e| eprintln!("{e}"))?;
    let page_count = frontmatters.len().div_ceil(POSTS_PER_PAGE);
    frontmatters.sort_by(|a, b| b.posted.cmp(&a.posted));
    let frontmatters = frontmatters
        .into_iter()
        .take(POSTS_PER_PAGE)
        .collect::<Vec<_>>();
    context.insert("posts", &frontmatters);
    context.insert("page", "home");
    context.insert("current_page", &1);
    context.insert("page_count", &page_count);
    let html = templates.render("home.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[get("/pages/{page_num}")]
pub async fn page(
    templates: web::Data<Tera>,
    page_num: web::Path<usize>,
) -> Result<HttpResponse, RespError> {
    let mut frontmatters = find_all_frontmatters().inspect_err(|e| eprintln!("{e}"))?;
    let page_count = frontmatters.len().div_ceil(POSTS_PER_PAGE);
    let page_num = page_num.into_inner();
    if page_num > page_count || page_num < 1 {
        return Err(RespError::NotFound);
    }
    let mut context = CONTEXT.clone();
    frontmatters.sort_by(|a, b| b.posted.cmp(&a.posted));
    let frontmatters = frontmatters
        .into_iter()
        .skip((page_num - 1) * POSTS_PER_PAGE)
        .take(POSTS_PER_PAGE)
        .collect::<Vec<_>>();
    context.insert("posts", &frontmatters);
    context.insert("page", "home");
    context.insert("current_page", &page_num);
    context.insert("page_count", &page_count);
    let html = templates.render("home.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn find_all_frontmatters() -> Result<Vec<FrontMatter>, CatError> {
    let mut t = TypesBuilder::new();
    t.add_defaults();
    let toml = t.select("toml").build().unwrap();
    let file_walker = WalkBuilder::new("./posts").types(toml).build();
    let mut frontmatters = Vec::new();
    for entry in file_walker {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let content = fs::read_to_string(path)?;
            let fm: FrontMatter = toml::from_str(&content)?;
            frontmatters.push(fm);
        }
    }
    Ok(frontmatters)
}
