#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::collections::HashMap;

use error::DetectorError;
use shared::{
    detector_params::DetectorParams,
    messages::detector::{IncMessage, OutMessage},
    ActorWrapper,
};
use spam_detector::SpamDetector;
use tokio::sync::mpsc::{self, Receiver, Sender};

mod error;
mod spam_detector;
mod user_data;

pub struct DetectorManager {
    streams: HashMap<String, SpamDetector>,
    rx: Receiver<IncMessage>,
    result_tx: Sender<OutMessage>,
    params: DetectorParams,
}

impl DetectorManager {
    pub fn init(
        detector_params: DetectorParams,
        result_tx: Sender<OutMessage>,
    ) -> ActorWrapper<IncMessage> {
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

        let tx = shared::AlkSender::new(tx, "DetectorManager_tx".to_string());
        ActorWrapper { join_handle, tx }
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // Manager finished it's work due to incoming `Close` message
            }
            Err(e) => {
                shared::tracing_warn!("Error, while processing messages: {}", &e);
            }
        }

        shared::tracing_info!("Closed");
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
                IncMessage::Close => return Ok(()),
                IncMessage::ChatPoller(poller_message) => {
                    match poller_message {
                        shared::messages::chat_poller::OutMessage::ChatInit {
                            channel,
                            video_id,
                        } => {
                            // TODO: load channel specific detector params
                            self.streams.insert(video_id.clone(), SpamDetector::init());

                            let message = OutMessage::NewChat { channel, video_id };
                            self.result_tx.send(message).await?;
                        }
                        shared::messages::chat_poller::OutMessage::NewBatch {
                            video_id,
                            actions,
                        } => {
                            let detector_instance = self.streams.get_mut(&video_id);

                            let detector_instance = match detector_instance {
                                Some(instance) => instance,
                                None => {
                                    shared::tracing_warn!(
                                        "{} has sent `NewBatch` before `ChatInit`",
                                        &video_id
                                    );
                                    continue;
                                }
                            };

                            let result =
                                detector_instance.process_new_messages(actions, &self.params);
                            self.result_tx
                                .send(OutMessage::DetectorResult {
                                    video_id,
                                    decisions: result.decisions,
                                    processed_messages: result.processed_messages,
                                })
                                .await?;
                        }
                        shared::messages::chat_poller::OutMessage::StreamEnded {
                            channel,
                            video_id,
                        } => {
                            self.streams.remove(&video_id);
                            self.result_tx
                                .send(OutMessage::ChatClosed { channel, video_id })
                                .await?;
                        }
                    }
                }
            }
        }
    }
}
