use auto_builder::{Message, socket, ws};
use futures::future::join;
use notify::event::ModifyKind;
use notify::{Event, EventHandler, EventKind, RecursiveMode, Watcher};
use std::process::Command;
use std::time::Instant;
use tokio::sync::mpsc::{UnboundedSender, channel, unbounded_channel};
use tokio_tungstenite::tungstenite::Bytes;

struct Sender(UnboundedSender<Result<Event, notify::Error>>);

impl EventHandler for Sender {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        let _ = self.0.send(event);
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let pwd = std::env::current_dir()?;
    let sh = pwd.join("watch_tailwind.sh");
    Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("blog")
        .spawn()?
        .wait()?;
    Command::new("bash").arg(&sh).spawn()?;
    Command::new(pwd.join("./target/debug/blog")).spawn()?;

    let exclude_files = ["./blog/static/css/index.css"]
        .iter()
        .map(|p| pwd.join(p))
        .collect::<Vec<_>>();
    let include_files = [
        "./blog/other_data",
        "./blog/posts",
        "./blog/static",
        "./blog/tailwind",
        "./blog/templates",
    ]
    .iter()
    .map(|p| pwd.join(p))
    .collect::<Vec<_>>();

    let (tx1, _) = tokio::sync::broadcast::channel::<Bytes>(100);
    let (reload_tx, reload_rx) = channel::<Message>(100);
    let ws_handle = tokio::spawn(ws::init(tx1.clone()));
    let socket_handle = tokio::spawn(socket::run(reload_rx));
    let reload_tx_clone = reload_tx.clone();
    let handle_events = async move {
        let (tx, mut rx) = unbounded_channel::<Result<Event, notify::Error>>();
        // Use recommended_watcher() to automatically select the best implementation
        // for your platform. The `EventHandler` passed to this constructor can be a
        // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
        // another type the trait is implemented for.
        let mut watcher = notify::recommended_watcher(Sender(tx))?;
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(&pwd, RecursiveMode::Recursive)?;
        while let Some(res) = rx.recv().await {
            // println!("receive event");
            match res {
                Ok(event) => {
                    // println!("{:?}", event.paths);
                    // println!("{:?}", event.kind);
                    if !matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                        // println!("continue");
                        continue;
                    }
                    if event.paths.len() == 1
                        && exclude_files.iter().any(|e| event.paths[0].starts_with(e))
                    {
                        continue;
                    } else if event
                        .paths
                        .iter()
                        .any(|p| include_files.iter().any(|e| p.starts_with(e)))
                    {
                        // println!("event: {:?}", event);
                        // Here to trigger the build process or any other action
                        let ins = Instant::now();
                        if let Err(e) = reload_tx.send(Message::Reload(ins)).await {
                            eprintln!("Can not send reload msg: {e}");
                        }
                        if let Err(e) = tx1.send(Bytes::new()) {
                            eprintln!("can not send refresh msg: {e}");
                        }
                        // println!("sent!");
                    }
                    // println!("continue");
                    continue;
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
        // println!("finished");
        Ok::<(), anyhow::Error>(())
    };
    let handle = tokio::spawn(handle_events);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl-C received, shutting down.");
            reload_tx_clone.send(Message::Exit).await?;
            socket_handle.await?;
        }

        (a, b) = join(handle, ws_handle) => {
            a??;
            b?;
        }
    }
    Ok(())
}
