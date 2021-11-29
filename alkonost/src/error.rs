use core::{http_client::HttpClientInitError, messages};

use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

#[derive(Error, Debug)]
pub enum AlkonostInitError {
    #[error("Couldn't initialize http client: {0}")]
    HttpClientInit(#[source] HttpClientInitError),
}

impl From<HttpClientInitError> for AlkonostInitError {
    fn from(e: HttpClientInitError) -> Self {
        Self::HttpClientInit(e)
    }
}

#[derive(Error, Debug)]
pub enum AlkonostError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
    #[error("Couldn't send message to the StreamFinder: {0}")]
    StreamFinderChannel(#[source] SendError<messages::stream_finder::IncMessages>),
    #[error("Couldn't send message to the ChatManager: {0}")]
    ChatManagerChannel(#[source] SendError<messages::chat_manager::IncMessages>),
    #[error("Couldn't send message to the Detector: {0}")]
    DetectorChannel(#[source] SendError<messages::detector::IncMessages>),
    #[error("Couldn't join child task: {0}")]
    JoinTask(#[source] JoinError)
}

impl From<SendError<messages::stream_finder::IncMessages>> for AlkonostError {
    fn from(e: SendError<messages::stream_finder::IncMessages>) -> Self {
        Self::StreamFinderChannel(e)
    }
}

impl From<SendError<messages::chat_manager::IncMessages>> for AlkonostError {
    fn from(e: SendError<messages::chat_manager::IncMessages>) -> Self {
        Self::ChatManagerChannel(e)
    }
}

impl From<SendError<messages::detector::IncMessages>> for AlkonostError {
    fn from(e: SendError<messages::detector::IncMessages>) -> Self {
        Self::DetectorChannel(e)
    }
}

impl From<JoinError> for AlkonostError {
    fn from(e: JoinError) -> Self {
        Self::JoinTask(e)
    }
}