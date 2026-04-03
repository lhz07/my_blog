use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::{io::AsyncReadExt, net::TcpListener, sync::mpsc::Receiver};

use crate::Message;

pub async fn run(rx: Receiver<Message>) {
    if let Err(e) = listen(rx).await {
        eprintln!("Socket server error: {}", e);
    }
}

#[derive(Debug)]
pub enum SocketMsg {
    Reload = 1,
    Success,
    Error,
    Exit,
}

async fn listen(mut rx: Receiver<Message>) -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9002").await?;
    println!("Socket server listening on {}", listener.local_addr()?);
    let mut socket = Socket::new_none();
    let mut buf = [0u8; 1];
    loop {
        tokio::select! {
            res = listener.accept() => {
                let (stream, addr) = res?;
                socket.set_stream(stream);
                println!("New connection from {}", addr);
            }
            res = rx.recv() => {
                match res {
                    Some(Message::Reload(ins)) => {
                        buf[0] = SocketMsg::Reload as u8;
                        if let Err(e) = socket.write_all(&buf).await {
                            eprintln!("Failed to send data to {}: {}", socket.addr().unwrap(), e);
                        }
                        if let Err(e) = socket.read_exact(&mut buf).await {
                            eprintln!("Failed to read data from {}: {}", socket.addr().unwrap(), e);
                        }
                        if buf[0] == SocketMsg::Success as u8 {
                            println!("Done in {:?}", ins.elapsed());
                        } else {
                            eprintln!("tera reload error");
                        }
                    }
                    Some(Message::Exit) => {
                        buf[0] = SocketMsg::Exit as u8;
                        if let Err(e) = socket.write_all(&buf).await {
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
