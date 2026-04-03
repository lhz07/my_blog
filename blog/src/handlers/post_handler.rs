use crate::{CONTEXT, errors::RespError};
use actix_web::{HttpResponse, route, web};
use search_utils::{
    lock::Lock,
    post::{FRONTMATTER, FrontMatter, MD_OPTIONS, extract_frontmatter, extract_md},
};
use serde::Serialize;
use std::sync::{Arc, LazyLock};
use tera::Tera;

pub fn initial_sort_by_posted_fm() -> Vec<Arc<FrontMatter>> {
    let mut fms = FRONTMATTER
        .get()
        .values()
        .cloned()
        .collect::<Vec<Arc<FrontMatter>>>();
    fms.sort_by(|a, b| b.posted.cmp(&a.posted));
    fms
}

pub fn initial_sort_by_updated_fm() -> Vec<Arc<FrontMatter>> {
    let mut fms = FRONTMATTER
        .get()
        .values()
        .cloned()
        .collect::<Vec<Arc<FrontMatter>>>();
    fms.sort_by(|a, b| b.updated.cmp(&a.updated));
    fms
}

pub static SORT_BY_POSTED_FRONTMATTERS: LazyLock<Lock<Vec<Arc<FrontMatter>>>> =
    LazyLock::new(|| Lock::new(initial_sort_by_posted_fm()));

pub static SORT_BY_UPDATED_FRONTMATTERS: LazyLock<Lock<Vec<Arc<FrontMatter>>>> =
    LazyLock::new(|| Lock::new(initial_sort_by_updated_fm()));

#[derive(Debug, Serialize)]
pub struct FrontMatterWithRfc2822 {
    pub fm: Arc<FrontMatter>,
    pub date: String,
}

pub static SORT_BY_UPDATED_WITH_RFC2822: LazyLock<Vec<FrontMatterWithRfc2822>> =
    LazyLock::new(|| {
        SORT_BY_UPDATED_FRONTMATTERS
            .get()
            .iter()
            .map(|fm| {
                let fm = fm.clone();
                let date = fm.updated.to_rfc2822();
                FrontMatterWithRfc2822 { fm, date }
            })
            .collect::<Vec<FrontMatterWithRfc2822>>()
    });

#[route("/posts/{post_name}", method = "GET", method = "HEAD")]
pub async fn post(
    templates: web::Data<Arc<Lock<Tera>>>,
    post_name: web::Path<String>,
) -> Result<HttpResponse, RespError> {
    render_a_post(
        templates,
        post_name,
        &SORT_BY_UPDATED_FRONTMATTERS,
        "/",
        "Home",
        "/posts",
    )
}

pub fn render_a_post(
    templates: web::Data<Arc<Lock<Tera>>>,
    post_name: web::Path<String>,
    fms: &Lock<Vec<Arc<FrontMatter>>>,
    back: &str,
    back_text: &str,
    current: &str,
) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    let md_text = extract_md(&post_name).map_err(|e| {
        log::error!("{e}");
        RespError::NotFound
    })?;
    let frontmatter = extract_frontmatter(&post_name).inspect_err(|e| log::error!("{e}"))?;

    let md_html = comrak::markdown_to_html(&md_text, &MD_OPTIONS);
    context.insert("post", &md_html);
    context.insert("meta_data", frontmatter.as_ref());
    context.insert("back", back);
    context.insert("back_text", back_text);
    context.insert("current", current);
    let index = fms
        .get()
        .iter()
        .position(|f| f.file_name == frontmatter.file_name);
    if let Some(index) = index {
        if let Some(next) = fms.get().get(index + 1) {
            context.insert("next", next);
        }
        if index > 0 {
            let prev = &fms.get()[index - 1];
            context.insert("prev", prev);
        }
    }
    let html = templates.get().render("post.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
