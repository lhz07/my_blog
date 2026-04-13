use crate::{
    TEMPLATES,
    handlers::{
        archive_handler::{ARCHIVES, init_archives},
        post_handler::{
            SORT_BY_POSTED_FRONTMATTERS, SORT_BY_UPDATED_FRONTMATTERS, initial_sort_by_posted_fm,
            initial_sort_by_updated_fm,
        },
    },
};
use actix_web::rt::net::TcpStream;
use auto_builder::{bitcode, socket::SocketMsg};
use search_utils::{
    formatter::{self, ShorterPath},
    post::{FRONTMATTER, initial_fm},
};
use std::{
    io,
    path::Path,
    time::{self, Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn run() {
    if let Err(e) = connect().await {
        log::error!("Socket connection error: {e}");
    }
}

async fn connect() -> Result<(), io::Error> {
    let mut stream = TcpStream::connect("127.0.0.1:9002").await?;
    log::info!("Connected to the server: {:?}", stream.peer_addr()?);
    let mut last_format = time::Instant::now();
    loop {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let mut content_buf = vec![0u8; u32::from_be_bytes(len_buf) as usize];
        stream.read_exact(&mut content_buf).await?;
        let msg = bitcode::decode(&content_buf).map_err(io::Error::other)?;
        match msg {
            SocketMsg::Reload(paths) => {
                let msg = match reload(paths, &mut last_format) {
                    Ok(()) => SocketMsg::Success,
                    Err(()) => SocketMsg::Error,
                };
                let content = bitcode::encode(&msg);
                stream
                    .write_all(&(content.len() as u32).to_be_bytes())
                    .await?;
                stream.write_all(&content).await?;
            }
            SocketMsg::Exit => {
                log::info!("Exit command received. Closing connection.");
                return Ok(());
            }
            _ => {
                log::error!("unexpected message received: {:?}", msg);
            }
        }
    }
}

fn reload(paths: Vec<String>, last_format: &mut Instant) -> Result<(), ()> {
    let ins = std::time::Instant::now();
    // reload tera templates
    TEMPLATES.get_mut().full_reload().map_err(|e| {
        log::error!("tera error: {e}");
    })?;
    // reload fms
    let map = initial_fm().map_err(|e| {
        log::error!("frontmatter error: {e}");
    })?;
    *FRONTMATTER.get_mut() = map;
    // reload sorted fms
    *SORT_BY_POSTED_FRONTMATTERS.get_mut() = initial_sort_by_posted_fm();
    *SORT_BY_UPDATED_FRONTMATTERS.get_mut() = initial_sort_by_updated_fm();
    let map = init_archives().map_err(|e| {
        log::error!("archives error: {e}");
    })?;
    *ARCHIVES.get_mut() = map;
    log::info!("tera cost: {:?}", ins.elapsed());
    log::info!("Templates reloaded.");
    if last_format.elapsed() > Duration::from_secs(3) {
        for path in paths {
            if let Err(e) = formatter::format_md_file(path.as_ref()) {
                let path = Path::new(&path);
                log::error!("format md file {}: {e}", path.shorter_path().display())
            }
        }
    }
    *last_format = time::Instant::now();
    Ok(())
}
