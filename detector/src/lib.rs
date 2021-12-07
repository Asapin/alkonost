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
    streams: HashMap<String, (String, SpamDetector)>,
    rx: Receiver<IncMessage>,
    result_tx: Sender<OutMessage>,
    params: HashMap<String, DetectorParams>,
}

impl DetectorManager {
    pub fn init(result_tx: Sender<OutMessage>) -> ActorWrapper<IncMessage> {
        let (tx, rx) = mpsc::channel(32);
        let manager = Self {
            streams: HashMap::new(),
            rx,
            result_tx,
            params: HashMap::new()
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
                            self.streams.insert(video_id.clone(), (channel.clone(), SpamDetector::init()));
                            self.load_detector_params_for_channel(channel.clone());

                            let message = OutMessage::NewChat { channel, video_id };
                            self.result_tx.send(message).await?;
                        }
                        shared::messages::chat_poller::OutMessage::NewBatch {
                            video_id,
                            actions,
                        } => {
                            let detector_with_params = self.get_detector_and_params(&video_id);
                            let (channel, mut detector, params) = match detector_with_params {
                                Some((channel, detector, params)) => (channel, detector, params),
                                None => {
                                    shared::tracing_warn!(
                                        "{} has sent `NewBatch` before `ChatInit`",
                                        &video_id
                                    );
                                    continue;
                                },
                            };

                            let result = detector.process_new_messages(&video_id, actions, &params);

                            // Put detector and params back
                            self.params.insert(channel.clone(), params);
                            self.streams.insert(video_id.clone(), (channel, detector));

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

    fn load_detector_params_for_channel(&mut self, channel: String) {
        self
            .params
            .entry(channel)
            .or_insert(DetectorParams::default());
    }

    fn get_detector_and_params(&mut self, video_id: &str) -> Option<(String, SpamDetector, DetectorParams)> {
        let (channel, detector) = self.streams.remove(video_id)?;
        let params = self.params.remove(&channel);

        match params {
            Some(params) => Some((channel, detector, params)),
            None => {
                self.streams.insert(video_id.to_string(), (channel, detector));
                None
            },
        }
    }
}
