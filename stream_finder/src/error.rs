use core::{http_client::HttpClientLoadError, messages::ChatManagerMessages};

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Error, Debug)]
pub enum StreamFinderError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
    #[error("Channel to the ChatManager has been closed: {0}")]
    OutgoingChannelClosed(#[source] SendError<ChatManagerMessages>),
}

impl From<SendError<ChatManagerMessages>> for StreamFinderError {
    fn from(e: SendError<ChatManagerMessages>) -> Self {
        StreamFinderError::OutgoingChannelClosed(e)
    }
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("{0}: Couldn't load channel content: {1}")]
    LoadContent(String, #[source] HttpClientLoadError),
    #[error("{0}: Couldn't dump error {1} due to another error {2}")]
    DumpError(String, serde_json::Error, std::io::Error),
    #[error("{0}: Couldn't extract video list: {1}")]
    VideoList(String, #[source] serde_json::Error),
}