use crate::{
    errors::{CatError, RespError},
    timestamp::TimeStamp,
};
use actix_web::{HttpResponse, get, web};
use ignore::{WalkBuilder, types::TypesBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use tera::Tera;

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

#[get("/favicon.ico")]
pub async fn icon() -> Result<HttpResponse, RespError> {
    let icon = fs::read("./static/img/favicon.ico").map_err(|_| RespError::NotFound)?;
    println!("receive");
    Ok(HttpResponse::Ok().content_type("image/x-icon").body(icon))
}

#[get("/")]
pub async fn index(templates: web::Data<Tera>) -> Result<HttpResponse, RespError> {
    let mut context = tera::Context::new();

    let mut frontmatters = find_all_frontmatters().inspect_err(|e| eprintln!("{e}"))?;
    frontmatters.sort_by(|a, b| b.posted.cmp(&a.posted));
    // let frontmatters = frontmatters.into_iter().take(10).collect::<Vec<_>>();
    context.insert("posts", &frontmatters);
    context.insert("page", "home");
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
