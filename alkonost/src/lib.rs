#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use shared::{
    http_client::HttpClient,
    messages::{self, alkonost::IncMessage},
    ActorWrapper,
};
use std::{sync::Arc, time::Duration};

use ::chat_manager::ChatManager;
use ::detector::DetectorManager;
use error::{AlkonostError, AlkonostInitError};
use stream_finder::StreamFinder;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

pub mod error;

pub type DetectorParams = shared::detector_params::DetectorParams;
pub type RequestSettings = shared::http_client::RequestSettings;
pub type AlkonostInMessage = shared::messages::alkonost::IncMessage;
pub type AlkonostOutMessage = shared::messages::detector::OutMessage;

pub struct Alkonost {
    rx: Receiver<IncMessage>,
    stream_finder: JoinHandle<()>,
    chat_manager: JoinHandle<()>,
    detector: JoinHandle<()>,
    finder_to_chat_handle: JoinHandle<()>,
    chat_to_detector_handle: JoinHandle<()>,
    stream_finder_tx: Sender<messages::stream_finder::IncMessage>,
    chat_manager_tx: Sender<messages::chat_manager::IncMessage>,
    detector_tx: Sender<messages::detector::IncMessage>,
}

impl Alkonost {
    pub fn init(
        detector_params: DetectorParams,
        request_settings: RequestSettings,
        chat_poll_interval: Duration,
    ) -> Result<
        (
            ActorWrapper<IncMessage>,
            Receiver<messages::detector::OutMessage>,
        ),
        AlkonostInitError,
    > {
        let (detector_result_tx, detector_result_rx) = mpsc::channel(32);
        let ActorWrapper {
            join_handle: detector,
            tx: detector_tx,
        } = DetectorManager::init(detector_params, detector_result_tx);
        let detector_tx_clone = detector_tx.clone();

        let http_client = HttpClient::init()?;
        let http_client = Arc::new(http_client);

        let (chat_manager_result_tx, mut chat_manager_result_rx) = mpsc::channel(32);
        let ActorWrapper {
            join_handle: chat_manager,
            tx: chat_manager_tx,
        } = ChatManager::init(
            http_client.clone(),
            request_settings.clone(),
            chat_manager_result_tx,
        );
        let chat_manager_tx_clone = chat_manager_tx.clone();

        let (stream_finder_result_tx, mut stream_finder_result_rx) = mpsc::channel(32);
        let ActorWrapper {
            join_handle: stream_finder,
            tx: stream_finder_tx,
        } = StreamFinder::init(
            http_client,
            request_settings,
            stream_finder_result_tx,
            chat_poll_interval,
        );

        let chat_to_detector_handle = tokio::spawn(async move {
            while let Some(out_message) = chat_manager_result_rx.recv().await {
                let inc_message = messages::detector::IncMessage::ChatPoller(out_message);
                match detector_tx_clone.send(inc_message).await {
                    Ok(_r) => {}
                    Err(e) => {
                        println!(
                            "ChatToDetector: Detector's end of the channel has closed: {}",
                            &e
                        );
                        return;
                    }
                }
            }
        });

        let finder_to_chat_handle = tokio::spawn(async move {
            while let Some(out_message) = stream_finder_result_rx.recv().await {
                let inc_message = messages::chat_manager::IncMessage::FoundStreams {
                    channel: out_message.channel,
                    streams: out_message.streams,
                };
                match chat_manager_tx_clone.send(inc_message).await {
                    Ok(_r) => {}
                    Err(e) => {
                        println!(
                            "FinderToChat: Chat Manager's end of the channel has closed: {}",
                            &e
                        );
                        return;
                    }
                }
            }
        });

        let (tx, rx) = mpsc::channel(32);

        let alkonost = Self {
            rx,
            stream_finder,
            chat_manager,
            detector,
            finder_to_chat_handle,
            chat_to_detector_handle,
            stream_finder_tx,
            chat_manager_tx,
            detector_tx,
        };

        let join_handle = tokio::spawn(async move {
            alkonost.run().await;
        });

        let actor = ActorWrapper { join_handle, tx };

        Ok((actor, detector_result_rx))
    }

    async fn run(self) {
        match self.do_run().await {
            Ok(_r) => {
                // Alkonost finished it's work due to incoming `Close` message
            }
            Err(e) => {
                println!("Alkonost: Error: {}", &e);
            }
        }

        // We can do some cleaup work here before closing Alkonost
        println!("Alkonost has been closed");
    }

    async fn do_run(mut self) -> Result<(), AlkonostError> {
        loop {
            let message = match self.rx.recv().await {
                Some(message) => message,
                None => {
                    return Err(AlkonostError::IncomingChannelClosed);
                }
            };

            match message {
                messages::alkonost::IncMessage::Close => {
                    self.stream_finder_tx
                        .send(messages::stream_finder::IncMessage::Close)
                        .await?;
                    self.chat_manager_tx
                        .send(messages::chat_manager::IncMessage::Close)
                        .await?;
                    self.detector_tx
                        .send(messages::detector::IncMessage::Close)
                        .await?;
                    self.stream_finder.await?;
                    self.finder_to_chat_handle.await?;
                    self.chat_manager.await?;
                    self.chat_to_detector_handle.await?;
                    self.detector.await?;
                    return Ok(());
                }
                messages::alkonost::IncMessage::AddChannel(channel) => {
                    let module_message = messages::stream_finder::IncMessage::AddChannel(channel);
                    self.stream_finder_tx.send(module_message).await?;
                }
                messages::alkonost::IncMessage::RemoveChannel(channel) => {
                    let module_message =
                        messages::stream_finder::IncMessage::RemoveChannel(channel);
                    self.stream_finder_tx.send(module_message).await?;
                }
                messages::alkonost::IncMessage::UpdateStreamPollInterval(interval) => {
                    let module_message =
                        messages::stream_finder::IncMessage::UpdatePollInterval(interval);
                    self.stream_finder_tx.send(module_message).await?;
                }
                messages::alkonost::IncMessage::UpdateUserAgent(user_agent) => {
                    let module_message_1 =
                        messages::stream_finder::IncMessage::UpdateUserAgent(user_agent.clone());
                    let module_message_2 =
                        messages::chat_manager::IncMessage::UpdateUserAgent(user_agent);
                    self.stream_finder_tx.send(module_message_1).await?;
                    self.chat_manager_tx.send(module_message_2).await?;
                }
                messages::alkonost::IncMessage::UpdateBrowserVersion(browser_version) => {
                    let module_message_1 =
                        messages::stream_finder::IncMessage::UpdateBrowserVersion(
                            browser_version.clone(),
                        );
                    let module_message_2 =
                        messages::chat_manager::IncMessage::UpdateBrowserVersion(browser_version);
                    self.stream_finder_tx.send(module_message_1).await?;
                    self.chat_manager_tx.send(module_message_2).await?;
                }
                messages::alkonost::IncMessage::UpdateBrowserNameAndVersion { name, version } => {
                    let module_message_1 =
                        messages::stream_finder::IncMessage::UpdateBrowserNameAndVersion {
                            name: name.clone(),
                            version: version.clone(),
                        };

                    let module_message_2 =
                        messages::chat_manager::IncMessage::UpdateBrowserNameAndVersion {
                            name,
                            version,
                        };

                    self.stream_finder_tx.send(module_message_1).await?;
                    self.chat_manager_tx.send(module_message_2).await?;
                }
            }
        }
    }
}
