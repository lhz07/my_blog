use std::net::SocketAddr;

use futures::SinkExt;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::Receiver,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Bytes, Message},
};

pub async fn init(tx: tokio::sync::broadcast::Sender<Bytes>) {
    println!("Hello, I'm server!");
    let addr = "0.0.0.0:9001";
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Can not bind listen address, error: {}", error);
            return;
        }
    };
    println!("listening: {}", addr);
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let rx = tx.subscribe();
                tokio::spawn(handle_connection(stream, addr, rx));
            }
            Err(error) => {
                eprintln!("Error: {}", error);
                return;
            }
        }
    }
}

async fn handle_connection(stream: TcpStream, peer: SocketAddr, mut rx: Receiver<Bytes>) {
    println!("Connection from {:#?}", peer);
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(error) => {
            eprintln!("Can not accept Websocket Stream, Error: {}", error);
            return;
        }
    };
    while let Ok(msg) = rx.recv().await {
        if ws_stream.send(Message::Binary(msg)).await.is_err() {
            return;
        }
    }
}
