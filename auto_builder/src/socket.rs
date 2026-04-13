use bitcode::{Decode, Encode};

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::{io::AsyncReadExt, net::TcpListener, sync::mpsc::Receiver};

use crate::Message;

pub async fn run(rx: Receiver<Message>) {
    if let Err(e) = listen(rx).await {
        eprintln!("Socket server error: {}", e);
    }
}

#[derive(Debug, Encode, Decode)]
pub enum SocketMsg {
    Reload(Vec<String>),
    Success,
    Error,
    Exit,
}

async fn listen(mut rx: Receiver<Message>) -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9002").await?;
    println!("Socket server listening on {}", listener.local_addr()?);
    let mut socket = Socket::new_none();

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (stream, addr) = res?;
                socket.set_stream(stream);
                println!("New connection from {}", addr);
            }
            res = rx.recv() => {
                match res {
                    Some(Message::Reload(ins, path)) => {
                        let msg = SocketMsg::Reload(path.into_iter().filter(|p|p.ends_with("post.md")).map(|p|p.into_os_string().into_string().unwrap()).collect());
                        let content = bitcode::encode(&msg);
                        if let Err(e) = socket.write_all(&(content.len() as u32).to_be_bytes()).await {
                            eprintln!("Failed to send data to {}: {}", socket.addr().unwrap(), e);
                        }
                        if let Err(e) = socket.write_all(&content).await {
                            eprintln!("Failed to send data to {}: {}", socket.addr().unwrap(), e);
                        }
                        let mut len_buf = [0u8; 4];
                        if let Err(e) = socket.read_exact(&mut len_buf).await {
                            eprintln!("Failed to read data from {}: {}", socket.addr().unwrap(), e);
                        }
                        let mut content_buf = vec![0u8; u32::from_be_bytes(len_buf) as usize];
                        if let Err(e) = socket.read_exact(&mut content_buf).await {
                            eprintln!("Failed to read data from {}: {}", socket.addr().unwrap(), e);
                        }
                        let msg = bitcode::decode(&content_buf).unwrap();
                        if matches!(msg, SocketMsg::Success) {
                            println!("Done in {:?}", ins.elapsed());
                        } else {
                            eprintln!("tera reload error");
                        }
                    }
                    Some(Message::Exit) => {
                        let msg  = SocketMsg::Exit;
                        let content = bitcode::encode(&msg);
                        if let Err(e) = socket.write_all(&(content.len() as u32).to_be_bytes()).await {
                            eprintln!("Failed to send data to {}: {}", socket.addr().unwrap(), e);
                        }
                        if let Err(e) = socket.write_all(&content).await {
                            eprintln!("Failed to send exit to {}: {}", socket.addr().unwrap(), e);
                        }
                        return Ok(());
                    }
                    None => {
                        println!("Channel closed, stopping server.");
                        return Ok(());
                    }
                }
            }
        }
    }
}

struct Socket {
    stream: Option<TcpStream>,
}

impl Socket {
    fn new_none() -> Self {
        Self { stream: None }
    }
    fn set_stream(&mut self, stream: TcpStream) {
        self.stream = Some(stream);
    }
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.write_all(buf).await?;
        }
        Ok(())
    }
    async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.read_exact(buf).await?;
        }
        Ok(())
    }
    fn addr(&self) -> Option<std::net::SocketAddr> {
        self.stream.as_ref().and_then(|s| s.peer_addr().ok())
    }
}
