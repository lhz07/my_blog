use std::{io, net::TcpListener};

use my_blog::start_blog;

#[actix_web::main]
async fn main() -> io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();
    let path = "0.0.0.0:8000";
    let listener = TcpListener::bind(path)?;
    println!("listening on {}", path);
    start_blog(listener)?.await?;
    Ok(())
}
