use core::{http_client::HttpClientLoadError, messages::chat_poller::OutMessages};

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::type_converter::ConverterError;

#[derive(Error, Debug)]
pub enum InitError {
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
    #[error("Couldn't chat poller init: {0}")]
    NotifyingAboutInit(SendError<OutMessages>)
}

impl From<HttpClientLoadError> for InitError {
    fn from(e: HttpClientLoadError) -> Self {
        InitError::LoadChatPage(e)
    }
}

impl From<SendError<OutMessages>> for InitError {
    fn from(e: SendError<OutMessages>) -> Self {
        InitError::NotifyingAboutInit(e)
    }
}

#[derive(Error, Debug)]
pub enum PollerError {
    #[error("Couldn't serialize request body: {0}")]
    SerializeBody(#[source] serde_json::Error),
    #[error("Couldn't load chat messages: {0}")]
    LoadingMessages(#[source] HttpClientLoadError),
    #[error("Couldn't send message to chat manager: {0}")]
    SendToDetector(#[source] SendError<OutMessages>),
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

impl From<SendError<OutMessages>> for PollerError {
    fn from(e: SendError<OutMessages>) -> Self {
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
