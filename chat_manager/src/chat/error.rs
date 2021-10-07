use core::{http_client::HttpClientLoadError, messages::{AlkonostMessage, SpamDetectorMessages}};

use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

use crate::type_converter::ConverterError;

use super::inner_messages::{ManagerMessages, PollingResultMessages};

#[derive(Error, Debug)]
pub enum ChatManagerError {
    #[error("Incoming channel was closed before receiving `Close` message. Should never happen")]
    ReceiverClosed,
    #[error("Couldn't send message to the spam detector: {0}")]
    DetectorChannelClosed(#[source] SendError<SpamDetectorMessages>),
    #[error("Couldn't send message to the alkonost channel: {0}")]
    AlkonostChannelClosed(#[source] SendError<AlkonostMessage>),
    #[error("Couldn't join Tokio task: {0}")]
    JoinTask(#[source] JoinError),
    #[error("Stream finder has attempted to send another message after if already sent `Close` message. Should never happen")]
    UseAfterClosing,
    #[error("Chat poller disappeared from the inprogress_chats before it sent `StreamEnded` message. Should never happen")]
    ChatPollerDisappeared,
    #[error("Processing message {0:?} took too long")]
    SlowProcessing(ManagerMessages),
}

impl From<SendError<SpamDetectorMessages>> for ChatManagerError {
    fn from(e: SendError<SpamDetectorMessages>) -> Self {
        ChatManagerError::DetectorChannelClosed(e)
    }
}

impl From<SendError<AlkonostMessage>> for ChatManagerError {
    fn from(e: SendError<AlkonostMessage>) -> Self {
        ChatManagerError::AlkonostChannelClosed(e)
    }
}

impl From<JoinError> for ChatManagerError {
    fn from(e: JoinError) -> Self {
        ChatManagerError::JoinTask(e)
    }
}

#[derive(Error, Debug)]
pub enum ParamsExtractingError {
    #[error("Couldn't load chat page: {0}")]
    LoadChatPage(#[source] HttpClientLoadError),
    #[error("Couldn't find <gl> param: {0}")]
    ExtractGl(String),
    #[error("Couldn't find <remoteHost> param: {0}")]
    RemoteHost(String),
    #[error("Couldn't find <visitorData> param: {0}")]
    VisitorData(String),
    #[error("Couldn't find <continuation> param: {0}")]
    Continuation(String),
    #[error("Couldn't find <clientVersion> param: {0}")]
    ClientVersion(String),
    #[error("Couldn't find <INNERTUBE_API_KEY> param: {0}")]
    ChatKey(String),
}

impl From<HttpClientLoadError> for ParamsExtractingError {
    fn from(e: HttpClientLoadError) -> Self {
        ParamsExtractingError::LoadChatPage(e)
    }
}

#[derive(Error, Debug)]
pub enum PollerError {
    #[error("Couldn't serialize request body: {0}")]
    SerializeBody(#[source] serde_json::Error),
    #[error("Couldn't load chat messages: {0}")]
    LoadingMessages(#[source] HttpClientLoadError),
    #[error("Couldn't send message to chat manager: {0}")]
    SendToDetector(#[source] SendError<PollingResultMessages>),
    #[error("Couldn't dump error {0} due to another error {1}")]
    DumpError(ActionExtractorError, std::io::Error),
    #[error("Error while extracting actions from json {0}")]
    Extractor(ActionExtractorError),
    #[error("Channel closed")]
    ChannelClosed,
}

impl From<serde_json::Error> for PollerError {
    fn from(e: serde_json::Error) -> Self {
        PollerError::SerializeBody(e)
    }
}

impl From<SendError<PollingResultMessages>> for PollerError {
    fn from(e: SendError<PollingResultMessages>) -> Self {
        PollerError::SendToDetector(e)
    }
}

impl From<ActionExtractorError> for PollerError {
    fn from(e: ActionExtractorError) -> Self {
        PollerError::Extractor(e)
    }
}

impl From<HttpClientLoadError> for PollerError {
    fn from(e: HttpClientLoadError) -> Self {
        PollerError::LoadingMessages(e)
    }
}

#[derive(Error, Debug)]
pub enum ActionExtractorError {
    #[error("Couldn't deserialize chat json: {0}")]
    DeserializeChat(#[source] serde_json::Error),
    #[error("Couldn't convert actions to core types: {0}")]
    Converter(#[source] ConverterError),
}

impl From<serde_json::Error> for ActionExtractorError {
    fn from(e: serde_json::Error) -> Self {
        ActionExtractorError::DeserializeChat(e)
    }
}

impl From<ConverterError> for ActionExtractorError {
    fn from(e: ConverterError) -> Self {
        ActionExtractorError::Converter(e)
    }
}
