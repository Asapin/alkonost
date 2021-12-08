#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::{fmt::Debug, sync::Arc, time::Duration};

use chat_manager::ChatManager;
use detector::DetectorManager;
use error::{AlkonostError, AlkonostInitError};
use shared::{
    http_client::HttpClient,
    messages::{self, alkonost::IncMessage},
    ActorWrapper, AlkSender,
};
use stream_finder::StreamFinder;
use tokio::{
    sync::mpsc::{self, Receiver},
    task::JoinHandle,
};

pub mod error;

pub type DetectorParams = shared::detector_params::DetectorParams;
pub type RequestSettings = shared::http_client::RequestSettings;
pub type AlkonostInMessage = shared::messages::alkonost::IncMessage;
pub type AlkonostOutMessage = shared::messages::detector::OutMessage;
pub type DetectorDecision = shared::messages::detector::DetectorDecision;
pub type DecisionAction = shared::messages::detector::Decision;

pub struct Alkonost {
    rx: Receiver<IncMessage>,
    stream_finder: JoinHandle<()>,
    chat_manager: JoinHandle<()>,
    detector: JoinHandle<()>,
    finder_to_chat_handle: JoinHandle<()>,
    chat_to_detector_handle: JoinHandle<()>,
    stream_finder_tx: AlkSender<messages::stream_finder::IncMessage>,
    chat_manager_tx: AlkSender<messages::chat_manager::IncMessage>,
    detector_tx: AlkSender<messages::detector::IncMessage>,
}

impl Alkonost {
    pub fn init(
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
        } = DetectorManager::init(detector_result_tx);
        let mut detector_tx_clone = detector_tx.clone();

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
        let mut chat_manager_tx_clone = chat_manager_tx.clone();

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
                        shared::tracing_error!("Detector's end of the channel has closed: {}", &e);
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
                        shared::tracing_error!(
                            "Chat Manager's end of the channel has closed: {}",
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

        let tx = AlkSender::new(tx, "Alkonost_tx".to_string());
        let actor = ActorWrapper { join_handle, tx };

        Ok((actor, detector_result_rx))
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // Alkonost finished it's work due to incoming `Close` message
            }
            Err(e) => {
                shared::tracing_error!("Error: {}", &e);
            }
        }

        Alkonost::close_task(
            self.stream_finder,
            &mut self.stream_finder_tx,
            messages::stream_finder::IncMessage::Close,
            "stream_finder",
        )
        .await;

        Alkonost::await_task(self.finder_to_chat_handle, "finder_to_chat").await;

        Alkonost::close_task(
            self.chat_manager,
            &mut self.chat_manager_tx,
            messages::chat_manager::IncMessage::Close,
            "chat_manager",
        )
        .await;

        Alkonost::await_task(self.chat_to_detector_handle, "chat_to_detector").await;

        Alkonost::close_task(
            self.detector,
            &mut self.detector_tx,
            messages::detector::IncMessage::Close,
            "detector",
        )
        .await;

        // We can do some cleaup work here before closing Alkonost
        shared::tracing_info!("Closed");
    }

    async fn do_run(&mut self) -> Result<(), AlkonostError> {
        loop {
            let message = match self.rx.recv().await {
                Some(message) => message,
                None => {
                    return Err(AlkonostError::IncomingChannelClosed);
                }
            };

            match message {
                messages::alkonost::IncMessage::Close => return Ok(()),
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
                messages::alkonost::IncMessage::UpdateDetectorParams {
                    channel,
                    new_params,
                } => {
                    let module_message = messages::detector::IncMessage::UpdateParams {
                        channel,
                        params: new_params,
                    };

                    self.detector_tx.send(module_message).await?;
                }
            }
        }
    }

    async fn close_task<T: Debug>(
        task: JoinHandle<()>,
        tx: &mut AlkSender<T>,
        close_message: T,
        task_name: &str,
    ) {
        match tx.send(close_message).await {
            Ok(_r) => Alkonost::await_task(task, task_name).await,
            Err(e) => {
                shared::tracing_error!(
                    "Aborting {} because couldn't send `Close` message {}",
                    task_name,
                    &e
                );
                task.abort();
                Alkonost::await_task(task, task_name).await;
            }
        }
    }

    async fn await_task(task: JoinHandle<()>, task_name: &str) {
        match task.await {
            Ok(_r) => shared::tracing_info!("Task {} closed", &task_name),
            Err(e) => shared::tracing_error!("Task {} panicked: {}", &task_name, &e),
        }
    }
}
