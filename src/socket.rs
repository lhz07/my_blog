use crate::{
    TEMPLATES,
    handlers::{
        archive_handler::{ARCHIVES, init_archives},
        post_handler::{
            FRONTMATTER, SORT_BY_POSTED_FRONTMATTERS, SORT_BY_UPDATED_FRONTMATTERS, initial_fm,
            initial_sort_by_posted_fm, initial_sort_by_updated_fm,
        },
    },
};
use actix_web::rt::net::TcpStream;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
            // reload tera templates
            if let Err(e) = TEMPLATES.get_mut().full_reload() {
                eprintln!("tera error: {e}");
                buf[0] = 10;
                stream.write_all(&mut buf).await?;
                continue;
            }
            // reload fms
            match initial_fm() {
                Ok(map) => *FRONTMATTER.get_mut() = map,
                Err(e) => eprintln!("frontmatter error: {e}"),
            }
            // reload sorted fms
            *SORT_BY_POSTED_FRONTMATTERS.get_mut() = initial_sort_by_posted_fm();
            *SORT_BY_UPDATED_FRONTMATTERS.get_mut() = initial_sort_by_updated_fm();
            // reload archives
            match init_archives() {
                Ok(map) => *ARCHIVES.get_mut() = map,
                Err(e) => eprintln!("archives error: {e}"),
            }
            println!("tera cost: {:?}", ins.elapsed());
            println!("Templates reloaded.");
            buf[0] = 2;
            stream.write_all(&mut buf).await?;
        }
    }
}
