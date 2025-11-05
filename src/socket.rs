use actix_web::rt::net::TcpStream;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    TEMPLATES,
    handlers::post_handler::{FRONTMATTER, SORTED_FRONTMATTERS, initial_fm, initial_sorted_fm},
};

pub async fn run() {
    if let Err(e) = connect().await {
        eprintln!("Socket connection error: {e}");
    }
}

async fn connect() -> Result<(), io::Error> {
    let mut stream = TcpStream::connect("127.0.0.1:9002").await?;
    println!("Connected to the server: {:?}", stream.peer_addr()?);
    let mut buf = [0u8; 1];
    loop {
        stream.read_exact(&mut buf).await?;
        if buf[0] == 1 {
            let ins = std::time::Instant::now();
            if let Err(e) = TEMPLATES.get_mut().full_reload() {
                eprintln!("tera error: {e}");
                buf[0] = 10;
                stream.write_all(&mut buf).await?;
                continue;
            }
            match initial_fm() {
                Ok(map) => *FRONTMATTER.get_mut() = map,
                Err(e) => eprintln!("frontmatter error: {e}"),
            }
            *SORTED_FRONTMATTERS.get_mut() = initial_sorted_fm();
            println!("tera cost: {:?}", ins.elapsed());
            println!("Templates reloaded.");
            buf[0] = 2;
            stream.write_all(&mut buf).await?;
        }
    }
}
