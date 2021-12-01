#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use tokio::{sync::mpsc::Sender, task::JoinHandle};

pub use tracing::info as tracing_info;
pub use tracing::warn as tracing_warn;
pub use tracing::error as tracing_error;

pub mod detector_params;
pub mod http_client;
pub mod messages;
pub mod types;
pub mod youtube_regexes;

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
    TooManyDeletedMessages,
}