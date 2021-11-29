use core::messages::detector::OutMessages;

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Error, Debug)]
pub enum DetectorError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
    #[error("Outgoing messages channel was closed: {0}")]
    OutgoingChannelClosed(#[source] SendError<OutMessages>),
}

impl From<SendError<OutMessages>> for DetectorError {
    fn from(e: SendError<OutMessages>) -> Self {
        DetectorError::OutgoingChannelClosed(e)
    }
}