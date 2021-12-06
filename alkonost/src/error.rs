use shared::{http_client::HttpClientInitError, messages, ChannelSendError};
use thiserror::Error;
use tokio::task::JoinError;

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
    StreamFinderChannel(#[source] ChannelSendError<messages::stream_finder::IncMessage>),
    #[error("Couldn't send message to the ChatManager: {0}")]
    ChatManagerChannel(#[source] ChannelSendError<messages::chat_manager::IncMessage>),
    #[error("Couldn't send message to the Detector: {0}")]
    DetectorChannel(#[source] ChannelSendError<messages::detector::IncMessage>),
    #[error("Couldn't join child task: {0}")]
    JoinTask(#[source] JoinError),
}

impl From<ChannelSendError<messages::stream_finder::IncMessage>> for AlkonostError {
    fn from(e: ChannelSendError<messages::stream_finder::IncMessage>) -> Self {
        Self::StreamFinderChannel(e)
    }
}

impl From<ChannelSendError<messages::chat_manager::IncMessage>> for AlkonostError {
    fn from(e: ChannelSendError<messages::chat_manager::IncMessage>) -> Self {
        Self::ChatManagerChannel(e)
    }
}

impl From<ChannelSendError<messages::detector::IncMessage>> for AlkonostError {
    fn from(e: ChannelSendError<messages::detector::IncMessage>) -> Self {
        Self::DetectorChannel(e)
    }
}

impl From<JoinError> for AlkonostError {
    fn from(e: JoinError) -> Self {
        Self::JoinTask(e)
    }
}
