use crate::{
    CONTEXT,
    errors::{CatError, RespError},
    handlers::home_handler::FrontMatter,
};
use actix_web::{HttpResponse, get, web};
use comrak::Options;
use std::fs;
use tera::Tera;

fn extract_md(post_name: &str) -> Result<String, CatError> {
    let s = fs::read_to_string(format!("./posts/{}/post.md", post_name))?;
    Ok(s)
}

fn extract_frontmatter(post_name: &str) -> Result<FrontMatter, CatError> {
    let content = fs::read_to_string(format!("./posts/{}/post_frontmatter.toml", post_name))?;
    let fm = toml::from_str(&content)?;
    Ok(fm)
}

#[get("/posts/{post_name}")]
pub async fn post(
    templates: web::Data<Tera>,
    post_name: web::Path<String>,
) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    let mut options = Options::default();
    options.render.hardbreaks = true;
    options.extension.table = true;
    options.extension.cjk_friendly_emphasis = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    options.extension.underline = true;
    options.extension.superscript = true;
    let md_text = extract_md(&post_name).inspect_err(|e| eprintln!("{e}"))?;
    let frontmatter = extract_frontmatter(&post_name).inspect_err(|e| eprintln!("{e}"))?;

    let md_html = comrak::markdown_to_html(&md_text, &options);

    context.insert("post", &md_html);
    context.insert("meta_data", &frontmatter);
    let html = templates.render("post.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
