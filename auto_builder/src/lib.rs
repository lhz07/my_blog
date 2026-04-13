use std::{path::PathBuf, time::Instant};

pub mod socket;
pub mod ws;
pub use bitcode;

#[derive(Debug)]
pub enum Message {
    Reload(Instant, Vec<PathBuf>),
    Exit,
}
