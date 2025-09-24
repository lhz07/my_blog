use std::{io, net::TcpListener};

use actix_files::Files;
use actix_web::{App, HttpResponse, HttpServer, dev::Server, middleware, web};
use once_cell::sync::Lazy;
use tera::Tera;

pub mod errors;
pub mod handlers;

pub static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    let mut tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            std::process::exit(1);
        }
    };
    tera.autoescape_on(vec!["html", ".sql"]);
    tera
});

pub fn start_blog(listener: TcpListener) -> Result<Server, io::Error> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(TEMPLATES.clone()))
            .service(Files::new("/static", "static/").use_last_modified(true))
            .wrap(middleware::Logger::default())
            .route("/health", web::get().to(HttpResponse::Ok))
            .service(handlers::index)
            .service(handlers::post)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
