use std::time::Instant;

pub mod socket;
pub mod ws;

#[derive(Debug)]
pub enum Message {
    Reload(Instant),
    Exit,
}
