use core::{
    messages::{DetectorResults, SpamDetectorMessages},
    ActorWrapper,
};
use std::collections::HashMap;

use detector_params::DetectorParams;
use spam_detector::SpamDetector;
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError, Receiver, Sender};

mod message_data;
mod spam_detector;
mod user_data;

pub mod detector_params;

#[derive(Error, Debug)]
enum DetectorError {
    #[error("Incoming messages channel was closed. That should never happen.")]
    IncomingChannelClosed,
    #[error("Outgoing messages channel was closed: {0}")]
    OutgoingChannelClosed(#[source] SendError<DetectorResults>),
}

impl From<SendError<DetectorResults>> for DetectorError {
    fn from(e: SendError<DetectorResults>) -> Self {
        DetectorError::OutgoingChannelClosed(e)
    }
}

pub struct DetectorManager {
    streams: HashMap<String, SpamDetector>,
    rx: Receiver<SpamDetectorMessages>,
    result_tx: Sender<DetectorResults>,
    params: DetectorParams,
}

impl DetectorManager {
    pub fn init(
        detector_params: DetectorParams,
        result_tx: Sender<DetectorResults>,
    ) -> ActorWrapper<SpamDetectorMessages> {
        let (tx, rx) = mpsc::channel(32);
        let manager = Self {
            streams: HashMap::new(),
            rx,
            result_tx,
            params: detector_params,
        };

        let join_handle = tokio::spawn(async move {
            manager.run().await;
        });

        ActorWrapper { join_handle, tx }
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // Manager finished it's work due to incoming `Close` message
            }
            Err(e) => {
                println!("DetectorManager: Error, while processing messages: {}", &e);
            }
        }

        println!("DetectorManager: Sending `Close` message down the line...");
        match self.result_tx.send(DetectorResults::Close).await {
            Ok(_r) => {
                // Successfully sent a message to the receiver
                // Nothing else to do
            }
            Err(e) => {
                println!("DetectorManager: Couldn't send `Close` message: {}", &e);
            }
        }
        println!("DetectorManager has been closed");
    }

    async fn do_run(&mut self) -> Result<(), DetectorError> {
        loop {
            let message = match self.rx.recv().await {
                Some(message) => message,
                None => {
                    return Err(DetectorError::IncomingChannelClosed);
                }
            };

            match message {
                SpamDetectorMessages::Close => {
                    return Ok(());
                }
                SpamDetectorMessages::NewBatch { video_id, actions } => {
                    let detector_instance = self
                        .streams
                        .entry(video_id.clone())
                        .or_insert_with(SpamDetector::init);
                    let decisions = detector_instance.process_new_messages(actions, &self.params);
                    if !decisions.is_empty() {
                        let result = DetectorResults::ProcessingResult {
                            video_id,
                            decisions,
                        };

                        self.result_tx.send(result).await?;
                    }
                }
                SpamDetectorMessages::StreamEnded { video_id } => {
                    self.streams.remove(&video_id);
                    self.result_tx
                        .send(DetectorResults::StreamEnded { video_id })
                        .await?;
                }
            }
        }
    }
}
