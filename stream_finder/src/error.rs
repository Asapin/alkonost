use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamFinderError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
}
