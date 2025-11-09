use crate::{
    CONTEXT, MD_OPTIONS,
    errors::{CatError, RespError},
    handlers::home_handler::FrontMatter,
    lock::Lock,
};
use actix_web::{HttpResponse, get, web};
use ignore::{WalkBuilder, types::TypesBuilder};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, LazyLock},
};
use tera::Tera;

pub fn find_all_frontmatters() -> Result<Vec<FrontMatter>, CatError> {
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

pub static FRONTMATTER: LazyLock<Lock<HashMap<String, Arc<FrontMatter>>>> = LazyLock::new(|| {
    let map = match initial_fm() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Can not find frontmatters!, error: {e}");
            std::process::exit(1);
        }
    };
    Lock::new(map)
});

pub fn initial_fm() -> Result<HashMap<String, Arc<FrontMatter>>, CatError> {
    let fm = find_all_frontmatters()?;
    let map = fm
        .into_iter()
        .map(|f| (f.file_name.clone(), Arc::new(f)))
        .collect::<HashMap<_, _>>();
    Ok(map)
}

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

pub fn extract_md(post_name: &str) -> Result<String, CatError> {
    let s = fs::read_to_string(format!("./posts/{}/post.md", post_name))?;
    Ok(s)
}

pub fn extract_frontmatter(post_name: &str) -> Result<Arc<FrontMatter>, CatError> {
    let fm = FRONTMATTER
        .get()
        .get(post_name)
        .ok_or_else(|| {
            CatError::IO(std::io::Error::other(format!(
                "Frontmatter for post '{}' not found",
                post_name
            )))
        })?
        .clone();
    Ok(fm)
}

#[get("/posts/{post_name}")]
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
        eprintln!("{e}");
        RespError::NotFound
    })?;
    let frontmatter = extract_frontmatter(&post_name).inspect_err(|e| eprintln!("{e}"))?;

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
