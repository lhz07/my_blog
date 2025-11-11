use std::{io, net::TcpListener};

use my_blog::start_blog;
#[actix_web::main]
async fn main() -> io::Result<()> {
    unsafe {
        #[cfg(debug_assertions)]
        std::env::set_var("RUST_LOG", "info");
        #[cfg(not(debug_assertions))]
        std::env::set_var("RUST_LOG", "warn");
    }
    env_logger::init();
    initialize_static_vars();
    #[cfg(debug_assertions)]
    {
        use tokio::net::UdpSocket;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect("192.168.1.1:2333").await?;
        let local_addr = socket.local_addr()?;
        log::info!("Local IP: {}", local_addr.ip());
    }
    let path = "0.0.0.0:8000";
    let listener = TcpListener::bind(path)?;
    start_blog(listener)?.await?;
    Ok(())
}

fn initialize_static_vars() {
    use my_blog::handlers::{archive_handler::ARCHIVES, post_handler::FRONTMATTER};
    use std::sync::LazyLock;

    LazyLock::force(&my_blog::TEMPLATES);
    LazyLock::force(&ARCHIVES);
    LazyLock::force(&FRONTMATTER);
}
