use crate::{
    CONTEXT, MD_OPTIONS,
    errors::{CatError, RespError},
    handlers::home_handler::FrontMatter,
};
use actix_web::{HttpRequest, HttpResponse, get, web};
use std::fs;
use tera::Tera;

pub fn extract_md(post_name: &str) -> Result<String, CatError> {
    let s = fs::read_to_string(format!("./posts/{}/post.md", post_name))?;
    Ok(s)
}

pub fn extract_frontmatter(post_name: &str) -> Result<FrontMatter, CatError> {
    let content = fs::read_to_string(format!("./posts/{}/post_frontmatter.toml", post_name))?;
    let fm = toml::from_str(&content)?;
    Ok(fm)
}

#[get("/posts/{post_name}")]
pub async fn post(
    templates: web::Data<Tera>,
    post_name: web::Path<String>,
    request: HttpRequest,
) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    let md_text = extract_md(&post_name).map_err(|e| {
        eprintln!("{e}");
        RespError::NotFound
    })?;
    let frontmatter = extract_frontmatter(&post_name).inspect_err(|e| eprintln!("{e}"))?;

    let md_html = comrak::markdown_to_html(&md_text, &MD_OPTIONS);
    if let Some(header) = request
        .headers()
        .get("Referer")
        .and_then(|h| h.to_str().ok())
        && header != request.full_url().as_str()
    {
        context.insert("referer", header);
    }
    context.insert("post", &md_html);
    context.insert("meta_data", &frontmatter);
    let html = templates.render("post.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
