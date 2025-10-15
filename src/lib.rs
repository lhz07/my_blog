use actix_files::Files;
use actix_web::{
    App, HttpResponse, HttpServer,
    dev::Server,
    middleware::{self, Compress},
    web,
};
use once_cell::sync::Lazy;
use rand::seq::IndexedRandom;
use std::{fs, io, net::TcpListener, path::PathBuf};
use tera::Tera;

use crate::errors::{CatError, RespError};

pub mod errors;
pub mod handlers;
pub mod search_utils;
pub mod timestamp;

lazy_static::lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                std::process::exit(1);
            }
        };
        tera.autoescape_on(vec!["html", ".sql"]);
        tera
    };
}

pub static CONTEXT: Lazy<tera::Context> = Lazy::new(|| {
    #[cfg(debug_assertions)]
    {
        let mut context = tera::Context::new();
        context.insert("debug_mode", &true);
        context
    }
    #[cfg(not(debug_assertions))]
    {
        tera::Context::new()
    }
});

pub static MD_OPTIONS: Lazy<comrak::Options> = Lazy::new(|| {
    let mut options = comrak::Options::default();
    options.render.hardbreaks = true;
    options.extension.table = true;
    options.extension.cjk_friendly_emphasis = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    options.extension.underline = true;
    options.extension.superscript = true;
    options
});

#[inline(always)]
async fn not_found_handler() -> Result<HttpResponse, RespError> {
    not_found_page()
}

fn not_found_page() -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    context.insert("page", "not_found");
    let random_file = || -> Result<Vec<PathBuf>, CatError> {
        let mut stickers = Vec::new();
        for entry in fs::read_dir("./static/img/stickers")? {
            stickers.push(entry?.path());
        }
        Ok(stickers)
    };
    let stickers = random_file()?;
    let sticker = stickers
        .choose(&mut rand::rng())
        .and_then(|p| p.file_name())
        .ok_or(RespError::InternalServerError)?;
    context.insert("sticker", sticker.to_string_lossy().as_ref());
    let html = TEMPLATES.render("not_found.html", &context)?;
    Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body(html))
}

pub fn start_blog(listener: TcpListener) -> Result<Server, io::Error> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(TEMPLATES.clone()))
            .default_service(web::route().to(not_found_handler))
            .service(Files::new("/static", "static/").use_last_modified(true))
            .wrap(middleware::Logger::default())
            .wrap(Compress::default())
            .service(handlers::index)
            .service(handlers::page)
            .service(handlers::post)
            .service(handlers::search)
            .service(handlers::search_lucky)
            .service(handlers::friend_links)
            .service(handlers::post_link)
            .service(handlers::about)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
