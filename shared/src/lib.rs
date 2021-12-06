#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use tokio::{sync::mpsc::Sender, task::JoinHandle};

pub use tracing::info as tracing_info;
pub use tracing::warn as tracing_warn;
pub use tracing::error as tracing_error;
pub use vec1::Vec1 as Vec1;

pub mod detector_params;
pub mod http_client;
pub mod messages;
pub mod types;
pub mod youtube_regexes;

pub struct ActorWrapper<T> {
    pub join_handle: JoinHandle<()>,
    pub tx: Sender<T>,
}