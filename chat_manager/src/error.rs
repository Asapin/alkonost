use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatManagerError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
}
