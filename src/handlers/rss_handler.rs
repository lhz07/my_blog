use std::sync::Arc;

use actix_web::{HttpResponse, route, web};
use tera::Tera;

use crate::{
    CONTEXT, MD_OPTIONS,
    errors::{CatError, RespError},
    handlers::post_handler::{SORT_BY_UPDATED_WITH_RFC2822, extract_md},
    lock::Lock,
};

#[route("/index.xml", method = "GET", method = "HEAD")]
pub async fn rss(templates: web::Data<Arc<Lock<Tera>>>) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    if let Some(first) = SORT_BY_UPDATED_WITH_RFC2822.first() {
        context.insert("latest_update", first.date.as_str());
    }
    let mut contents = Vec::new();
    for item in SORT_BY_UPDATED_WITH_RFC2822.iter() {
        let md_text = extract_md(&item.fm.file_name)?;
        let md = comrak::markdown_to_html(&md_text, &MD_OPTIONS);
        contents.push(md);
    }
    context.insert("posts", &*SORT_BY_UPDATED_WITH_RFC2822);
    context.insert("contents", &contents);
    let html = templates
        .get()
        .render("rss.xml", &context)
        .inspect_err(|e| eprintln!("{e}"))?;
    Ok(HttpResponse::Ok()
        .content_type("text/xml; charset=utf-8")
        .body(html))
}

#[route("/favicon.ico", method = "GET", method = "HEAD")]
async fn favicon() -> Result<HttpResponse, RespError> {
    let data = std::fs::read("static/img/favicon.ico").map_err(|e| -> CatError { e.into() })?;
    Ok(HttpResponse::Ok().content_type("image/x-icon").body(data))
}
