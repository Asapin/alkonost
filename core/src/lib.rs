#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use tokio::{sync::mpsc::Sender, task::JoinHandle};

pub mod types;
pub mod http_client;
pub mod youtube_regexes;
pub mod messages;

pub struct ActorWrapper<T> {
    pub join_handle: JoinHandle<()>,
    pub tx: Sender<T>,
}

#[derive(Debug, Clone)]
pub enum SuspicionReason {
    TooFast(f32),
    TooLong(f32),
    Similar,
    Blocked,
    TooManyDeletedMessages
}