use crate::{errors::CatError, handlers::home_handler::FrontMatter};
use actix_web::{HttpResponse, Responder, get, web};
use comrak::Options;
use std::{fs, io};
use tera::Tera;

fn extract_md(post_name: &str) -> Result<String, io::Error> {
    fs::read_to_string(format!("./posts/{}/post.md", post_name))
}

fn extract_frontmatter(post_name: &str) -> Result<FrontMatter, CatError> {
    let content = fs::read_to_string(format!("./posts/{}/post_frontmatter.toml", post_name))?;
    let fm = toml::from_str(&content)?;
    Ok(fm)
}

#[get("/posts/{post_name}")]
pub async fn post(templates: web::Data<Tera>, post_name: web::Path<String>) -> impl Responder {
    let mut context = tera::Context::new();
    let mut options = Options::default();
    options.render.hardbreaks = true;
    options.extension.table = true;
    options.extension.cjk_friendly_emphasis = true;
    options.extension.strikethrough = true;
    let md_text = match extract_md(&post_name) {
        Ok(s) => s,
        Err(e) => {
            println!("{:?}", e);
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Could not find post - sorry!</p>");
        }
    };
    let frontmatter = match extract_frontmatter(&post_name) {
        Ok(s) => s,
        Err(e) => {
            println!("{:?}", e);
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Could not find post - sorry!</p>");
        }
    };

    let md_html = comrak::markdown_to_html(&md_text, &options);

    context.insert("post", &md_html);
    context.insert("meta_data", &frontmatter);

    match templates.render("post.html", &context) {
        Ok(s) => HttpResponse::Ok().content_type("text/html").body(s),
        Err(e) => {
            println!("{:?}", e);
            return HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Could not find post - sorry!</p>");
        }
    }
}
