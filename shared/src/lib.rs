#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::fmt::Debug;

use tokio::sync::mpsc::error::{SendError, TrySendError};
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use thiserror::Error;

pub use tracing::info as tracing_info;
pub use tracing::warn as tracing_warn;
pub use tracing::error as tracing_error;

pub mod detector_params;
pub mod http_client;
pub mod messages;
pub mod types;
pub mod youtube_regexes;

#[derive(Error, Debug)]
pub enum ChannelSendError<T> {
    #[error("Channel has been closed")]
    Closed(T),
}

impl<T> From<SendError<T>> for ChannelSendError<T> {
    fn from(e: SendError<T>) -> Self {
        Self::Closed(e.0)
    }
}

#[derive(Debug)]
enum Backpressure {
    Off,
    On {
        until_switching_off: u8
    }
}

pub struct AlkSender<T: Debug> {
    tx: Sender<T>,
    state: Backpressure,
    name: String,
}

impl<T: Debug> AlkSender<T> {
    pub fn new(tx: Sender<T>, name: String) -> Self {
        Self {
            tx,
            name,
            state: Backpressure::Off
        }
    }

    pub async fn send(&mut self, message: T) -> Result<(), ChannelSendError<T>> {
        let old_state = std::mem::replace(&mut self.state, Backpressure::Off);

        let (new_state, result) = AlkSender::do_send(
            &self.tx, 
            message, 
            old_state, 
            &self.name
        ).await;
        self.state = new_state;
        result
    }

    #[tracing::instrument(skip(tx))]
    async fn do_send(tx: &Sender<T>, message: T, state: Backpressure, name: &str) -> (Backpressure, Result<(), ChannelSendError<T>>) {
        match state {
            Backpressure::Off => {
                match tx.try_send(message) {
                    Ok(r) => (Backpressure::Off, Ok(r)),
                    Err(e) => {
                        match e {
                            TrySendError::Full(m) => {
                                tracing_warn!("The channel is full, enabling backpressure...");
                                let result = tx
                                    .send(m)
                                    .await
                                    .map_err(|e| ChannelSendError::Closed(e.0));

                                (Backpressure::On { until_switching_off: 9 }, result)
                            },
                            TrySendError::Closed(m) => {
                                (Backpressure::Off, Err(ChannelSendError::Closed(m)))
                            },
                        }
                    },
                }
            },
            Backpressure::On { 
                mut until_switching_off 
            } => {
                until_switching_off -= 1;
                let result = tx
                    .send(message)
                    .await
                    .map_err(ChannelSendError::from);

                if until_switching_off == 0 {
                    tracing_info!("Disabling backpressure...");
                    return (Backpressure::Off, result);
                }
                (Backpressure::On { until_switching_off }, result)
            }
        }
    }
}

impl<T: Debug> Clone for AlkSender<T> {
    fn clone(&self) -> Self {
        Self { 
            tx: self.tx.clone(), 
            state: Backpressure::Off,
            name: self.name.clone()
        }
    }
}

pub struct ActorWrapper<T: Debug> {
    pub join_handle: JoinHandle<()>,
    pub tx: AlkSender<T>,
}