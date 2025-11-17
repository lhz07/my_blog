use my_blog::start_blog;
use std::{io, net::TcpListener};

#[actix_web::main]
async fn main() -> io::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            #[cfg(debug_assertions)]
            std::env::set_var("RUST_LOG", "info");
            #[cfg(not(debug_assertions))]
            std::env::set_var("RUST_LOG", "warn");
        }
    }
    env_logger::init();
    initialize_static_vars();
    let path = "0.0.0.0:8000";
    let listener = TcpListener::bind(path)?;
    #[cfg(debug_assertions)]
    {
        use tokio::net::UdpSocket;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect("192.168.1.1:2333").await?;
        let local_addr = socket.local_addr()?;
        // link: \e]8;;<URL>\a<label>\e]8;;\a
        // e: x1b, a: x07
        // underline + blue: \x1b[4;34m<label>\x1b[0m
        // 4: underline, 34: blue, 0: reset
        log::info!(
            "Local IP: \x1b]8;;http://{0}:{1}\x1b\\http://{0}:{1}\x1b]8;;\x1b\\",
            local_addr.ip(),
            listener.local_addr()?.port(),
        );
    }
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
