use crate::errors::CatError;
use actix_web::{HttpResponse, Responder, get, web};
use ignore::{WalkBuilder, types::TypesBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use tera::Tera;

#[derive(Debug, Serialize, Deserialize)]
pub struct FrontMatter {
    title: String,
    file_name: String,
    description: String,
    posted: String,
    tags: Vec<String>,
    author: String,
    estimated_reading_time: u32,
    order: u32,
}

#[get("/")]
pub async fn index(templates: web::Data<Tera>) -> impl Responder {
    let mut context = tera::Context::new();

    let mut frontmatters = match find_all_frontmatters() {
        Ok(fm) => fm,
        Err(e) => {
            println!("{:?}", e);
            return HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("<p>Something went wrong!</p>");
        }
    };
    frontmatters.sort_by(|a, b| b.order.cmp(&a.order));

    context.insert("posts", &frontmatters);
    match templates.render("home.html", &context) {
        Ok(s) => HttpResponse::Ok().content_type("text/html").body(s),
        Err(e) => {
            println!("{:?}", e);
            HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("<p>Something went wrong!</p>")
        }
    }
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
