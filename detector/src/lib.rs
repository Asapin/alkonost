use core::{ActorWrapper, detector_params::DetectorParams, messages::detector::{IncMessages, OutMessages}};
use std::collections::HashMap;

use error::DetectorError;
use spam_detector::SpamDetector;
use tokio::sync::mpsc::{self, Receiver, Sender};

mod message_data;
mod spam_detector;
mod user_data;
mod error;

pub struct DetectorManager {
    streams: HashMap<String, SpamDetector>,
    rx: Receiver<IncMessages>,
    result_tx: Sender<OutMessages>,
    params: DetectorParams,
}

impl DetectorManager {
    pub fn init(
        detector_params: DetectorParams,
        result_tx: Sender<OutMessages>,
    ) -> ActorWrapper<IncMessages> {
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
                IncMessages::Close => return Ok(()),
                IncMessages::ChatPoller(poller_message) => {
                    match poller_message {
                        core::messages::chat_poller::OutMessages::ChatInit { 
                            channel: _, 
                            video_id 
                        } => {
                            // TODO: load channel specific detector params
                            self.streams.insert(video_id, SpamDetector::init());
                        },
                        core::messages::chat_poller::OutMessages::NewBatch { 
                            video_id, 
                            actions 
                        } => {
                            let detector_instance = self
                                .streams
                                .get_mut(&video_id);
                            
                            let detector_instance = match detector_instance {
                                Some(instance) => instance,
                                None => {
                                    println!("DetectorManager: {} has sent `NewBatch` before `ChatInit`", &video_id);
                                    continue;
                                }
                            };

                            let decisions = detector_instance.process_new_messages(actions, &self.params);
                            if !decisions.is_empty() {
                                let result = OutMessages::DetectorResult {
                                    video_id,
                                    decisions,
                                };
        
                                self.result_tx.send(result).await?;
                            }
                        },
                        core::messages::chat_poller::OutMessages::StreamEnded { 
                            video_id 
                        } => {
                            self.streams.remove(&video_id);
                            self.result_tx
                                .send(OutMessages::ChatClosed(video_id))
                                .await?;
                        },
                    }
                }
            }
        }
    }
}
