use crate::{
    errors::{CatError, RespError},
    lock::Lock,
};
use actix_files::Files;
use actix_web::{
    App, HttpResponse, HttpResponseBuilder, HttpServer,
    dev::{Server, ServiceResponse},
    http::StatusCode,
    middleware::{self, Compress, ErrorHandlerResponse, ErrorHandlers},
    web,
};
use rand::seq::IndexedRandom;
use std::{
    fs, io,
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tera::Tera;

pub mod errors;
pub mod handlers;
pub mod lock;
pub mod search_utils;
pub mod timestamp;

#[cfg(debug_assertions)]
pub mod socket;

pub static TEMPLATES: LazyLock<Arc<Lock<Tera>>> = LazyLock::new(|| {
    let mut tera = match Tera::new("templates/**/*.{html,xml}") {
        Ok(t) => t,
        Err(e) => {
            log::error!("Parsing error(s): {}", e);
            std::process::exit(1);
        }
    };
    tera.autoescape_on(vec!["html"]);
    let lock = Lock::new(tera);
    Arc::new(lock)
});

pub static CONTEXT: LazyLock<tera::Context> = LazyLock::new(|| {
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

pub static MD_OPTIONS: LazyLock<comrak::Options> = LazyLock::new(|| {
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

async fn not_found_handler() -> Result<HttpResponse, RespError> {
    not_found_page()
}

fn error_page(title: &str, mut kind: HttpResponseBuilder) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    context.insert("page", "not_found");
    context.insert("title", title);
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
    let html = TEMPLATES.get().render("not_found.html", &context)?;
    Ok(kind.content_type("text/html").body(html))
}

fn not_found_page() -> Result<HttpResponse, RespError> {
    error_page("Not Found", HttpResponse::NotFound())
}

fn render_400<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let new_resp = error_page("Bad Request", HttpResponse::BadRequest())
        .map_err(actix_web::error::ErrorBadRequest)?;
    let new_service_resp = res.into_response(new_resp.map_into_right_body());

    Ok(ErrorHandlerResponse::Response(new_service_resp))
}

fn render_500<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let new_resp = error_page("Internal Server Error", HttpResponse::InternalServerError())
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let new_service_resp = res.into_response(new_resp.map_into_right_body());

    Ok(ErrorHandlerResponse::Response(new_service_resp))
}

fn render_404<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let new_resp = not_found_page().map_err(actix_web::error::ErrorBadRequest)?;
    let new_service_resp = res.into_response(new_resp.map_into_right_body());

    Ok(ErrorHandlerResponse::Response(new_service_resp))
}

pub fn start_blog(listener: TcpListener) -> Result<Server, io::Error> {
    #[cfg(debug_assertions)]
    actix_web::rt::spawn(socket::run());
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(TEMPLATES.clone()))
            .default_service(web::route().to(not_found_handler))
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::BAD_REQUEST, render_400)
                    .handler(StatusCode::NOT_FOUND, render_404)
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, render_500),
            )
            .service(Files::new("/static", "static/").use_last_modified(!cfg!(debug_assertions)))
            .wrap(middleware::Logger::default())
            .wrap(Compress::default())
            .service(handlers::index)
            .service(handlers::page)
            .service(handlers::post)
            .service(handlers::archive_post)
            .service(handlers::search)
            .service(handlers::search_lucky)
            .service(handlers::friend_links)
            .service(handlers::post_link)
            .service(handlers::archive)
            .service(handlers::about)
            .service(handlers::favicon)
            .service(handlers::rss)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
