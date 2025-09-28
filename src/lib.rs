use actix_files::Files;
use actix_web::{App, HttpServer, dev::Server, middleware, web};
use once_cell::sync::Lazy;
use std::{io, net::TcpListener};
use tera::Tera;

pub mod errors;
pub mod handlers;
pub mod timestamp;

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

pub fn start_blog(listener: TcpListener) -> Result<Server, io::Error> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(TEMPLATES.clone()))
            .service(Files::new("/static", "static/").use_last_modified(true))
            .wrap(middleware::Logger::default())
            .service(handlers::index)
            .service(handlers::post)
            .service(handlers::friend_links)
            .service(handlers::post_link)
            .service(handlers::about)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
