use crate::errors::{CatError, RespError};
use actix_web::{HttpResponse, get, web};
use serde::{Deserialize, Serialize};
use tera::Tera;

#[derive(Debug, Deserialize, Serialize)]
struct Contact {
    name: String,
    url: String,
    icon: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repo {
    name: String,
    url: String,
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Profile {
    subtitle: String,
    description: String,
    pic: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AboutInfo {
    repo: Vec<Repo>,
    contact: Vec<Contact>,
    tech_stack: Vec<String>,
    profile: Profile,
}

fn extract_about() -> Result<AboutInfo, CatError> {
    let content = std::fs::read_to_string("./other_data/about.toml")?;
    let about_info = toml::from_str(&content)?;
    Ok(about_info)
}

#[get("/about")]
pub async fn about(templates: web::Data<Tera>) -> Result<HttpResponse, RespError> {
    let mut context = tera::Context::new();
    let about_info = extract_about().inspect_err(|e| eprintln!("{e}"))?;
    context.insert("page", "about");
    context.insert("repos", &about_info.repo);
    context.insert("contacts", &about_info.contact);
    context.insert("tech_stack", &about_info.tech_stack);
    context.insert("profile", &about_info.profile);
    let html = templates.render("about.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
