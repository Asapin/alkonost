use core::messages::detector::OutMessage;

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Error, Debug)]
pub enum DetectorError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
    #[error("Outgoing messages channel was closed: {0}")]
    OutgoingChannelClosed(#[source] SendError<OutMessage>),
}

impl From<SendError<OutMessage>> for DetectorError {
    fn from(e: SendError<OutMessage>) -> Self {
        DetectorError::OutgoingChannelClosed(e)
    }
}