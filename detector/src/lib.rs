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

struct ChannelData {
    streams: HashMap<String, SpamDetector>,
    params: DetectorParams
}

pub struct DetectorManager {
    active_channels: HashMap<String, ChannelData>,
    rx: Receiver<IncMessage>,
    result_tx: Sender<OutMessage>,
}

impl DetectorManager {
    pub fn init(result_tx: Sender<OutMessage>) -> ActorWrapper<IncMessage> {
        let (tx, rx) = mpsc::channel(32);
        let manager = Self {
            active_channels: HashMap::new(),
            rx,
            result_tx,
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
                            self.load_detector_and_params(channel.clone(), video_id.clone()).await;
                            let message = OutMessage::NewChat { channel, video_id };
                            self.result_tx.send(message).await?;
                        }
                        shared::messages::chat_poller::OutMessage::NewBatch {
                            channel,
                            video_id,
                            actions,
                        } => {
                            let channel_data = match self.active_channels.get_mut(&channel) {
                                Some(data) => data,
                                None => {
                                    shared::tracing_warn!("Channel data {} wasn't initialized", &channel);
                                    continue;
                                },
                            };

                            let detector = match channel_data.streams.get_mut(&video_id) {
                                Some(detector) => detector,
                                None => {
                                    shared::tracing_warn!("Stream data {} for channel {} wasn't initialized", &video_id, &channel);
                                    continue;
                                },
                            };

                            let result = detector.process_new_messages(&video_id, actions, &channel_data.params);

                            let message = OutMessage::DetectorResult {
                                video_id,
                                decisions: result.decisions,
                                processed_messages: result.processed_messages,
                            };
                            self.result_tx.send(message).await?;
                        }
                        shared::messages::chat_poller::OutMessage::StreamEnded {
                            channel,
                            video_id,
                        } => {
                            let channel_data = match self.active_channels.get_mut(&channel) {
                                Some(data) => data,
                                None => {
                                    shared::tracing_warn!("Can't remove uninitialized channel {}", &channel);
                                    continue;
                                },
                            };

                            let _ = channel_data.streams.remove(&video_id);

                            if channel_data.streams.is_empty() {
                                // TODO: Save detector params for channel
                                self.active_channels.remove(&channel);
                            }

                            self.result_tx
                                .send(OutMessage::ChatClosed { channel, video_id })
                                .await?;
                        }
                    }
                }
                IncMessage::UpdateParams { 
                    channel, 
                    params 
                } => {
                    let channel_data = match self.active_channels.get_mut(&channel) {
                        Some(data) => data,
                        None => {
                            shared::tracing_warn!("Can't remove uninitialized channel {}", &channel);
                            continue;
                        },
                    };

                    let messages = channel_data
                        .streams
                        .iter_mut()
                        .filter_map(|(video_id, detector)| {
                            let result = detector.reanalyze(&params)?;
                            let message = OutMessage::DetectorResult {
                                video_id: video_id.clone(),
                                decisions: result.decisions,
                                processed_messages: result.processed_messages,
                            };
                            Some(message)
                        });
                    
                    for message in messages {
                        self.result_tx.send(message).await?;
                    }
                }
            }
        }
    }

    async fn load_detector_and_params(&mut self, channel: String, video_id: String) {
        let channel_data = self
            .active_channels
            .entry(channel)
            .or_insert_with(|| {
                // Loading detector params for the channel
                ChannelData { 
                    streams: HashMap::new(),
                    params: DetectorParams::default()
                }
            });
        
        channel_data
            .streams
            .insert(video_id, SpamDetector::init());
    }
}
