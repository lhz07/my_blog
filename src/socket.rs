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
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn run() {
    if let Err(e) = connect().await {
        eprintln!("Socket connection error: {e}");
    }
}

#[derive(Debug, FromPrimitive)]
enum Msg {
    Reload = 1,
    Success,
    Error,
    Exit,
}

async fn connect() -> Result<(), io::Error> {
    let mut stream = TcpStream::connect("127.0.0.1:9002").await?;
    log::info!("Connected to the server: {:?}", stream.peer_addr()?);
    let mut buf = [0u8; 1];
    loop {
        stream.read_exact(&mut buf).await?;
        let msg = Msg::from_u8(buf[0]);
        match msg {
            Some(Msg::Reload) => {
                match reload() {
                    Ok(()) => {
                        buf[0] = Msg::Success as u8;
                    }
                    Err(()) => {
                        buf[0] = Msg::Error as u8;
                    }
                }
                stream.write_all(&mut buf).await?;
            }
            Some(Msg::Exit) => {
                log::info!("Exit command received. Closing connection.");
                return Ok(());
            }
            _ => {
                eprintln!("unexpected message received: {:?}", msg);
            }
        }
    }
}

fn reload() -> Result<(), ()> {
    let ins = std::time::Instant::now();
    // reload tera templates
    TEMPLATES.get_mut().full_reload().map_err(|e| {
        eprintln!("tera error: {e}");
    })?;
    // reload fms
    let map = initial_fm().map_err(|e| {
        eprintln!("frontmatter error: {e}");
    })?;
    *FRONTMATTER.get_mut() = map;
    // reload sorted fms
    *SORT_BY_POSTED_FRONTMATTERS.get_mut() = initial_sort_by_posted_fm();
    *SORT_BY_UPDATED_FRONTMATTERS.get_mut() = initial_sort_by_updated_fm();
    let map = init_archives().map_err(|e| {
        eprintln!("archives error: {e}");
    })?;
    // reload archives
    *ARCHIVES.get_mut() = map;
    log::info!("tera cost: {:?}", ins.elapsed());
    log::info!("Templates reloaded.");
    Ok(())
}
