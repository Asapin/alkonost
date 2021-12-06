#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration, mem::replace,
};

use chat_poller::{ChatPoller, InitResult};
use error::ChatManagerError;
use shared::{
    http_client::{HttpClient, RequestSettings},
    messages::{self, chat_manager::IncMessage},
    ActorWrapper,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::timeout,
};

mod error;

pub struct ChatManager {
    rx: Receiver<IncMessage>,
    check_children_period: Duration,
    http_client: Arc<HttpClient>,
    request_settings: RequestSettings,
    inprogress_chats: HashMap<String, ActorWrapper<messages::chat_poller::IncMessage>>,
    result_tx: Sender<messages::chat_poller::OutMessage>,
}

impl ChatManager {
    pub fn init(
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        result_tx: Sender<messages::chat_poller::OutMessage>,
    ) -> ActorWrapper<IncMessage> {
        let (tx, rx) = mpsc::channel(32);
        let check_children_period = Duration::from_secs(60);

        let manager = Self {
            rx,
            check_children_period,
            http_client,
            request_settings,
            inprogress_chats: HashMap::with_capacity(20),
            result_tx,
        };

        let join_handle = tokio::spawn(async move {
            let _result = manager.run().await;
        });

        let tx = shared::AlkSender::new(tx, "ChatManager_tx".to_string());
        ActorWrapper { join_handle, tx }
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // Chat manager was closed due to received `Close` message
            }
            Err(e) => {
                shared::tracing_error!("Encountered an error: {}", &e);
            }
        }

        self.close_gracefully().await;
        shared::tracing_info!("Closed");
    }

    async fn do_run(&mut self) -> Result<(), ChatManagerError> {
        loop {
            while let Ok(recv_result) = timeout(self.check_children_period, self.rx.recv()).await {
                match recv_result {
                    Some(message) => match message {
                        IncMessage::Close => return Ok(()),
                        IncMessage::FoundStreams { channel, streams } => {
                            let new_streams = streams
                                .into_iter()
                                .filter(|video_id| !self.inprogress_chats.contains_key(video_id))
                                .collect::<Vec<_>>();

                            for video_id in new_streams {
                                match ChatPoller::init(
                                    video_id.clone(),
                                    channel.clone(),
                                    self.http_client.clone(),
                                    self.request_settings.clone(),
                                    self.result_tx.clone(),
                                )
                                .await
                                {
                                    Ok(r) => match r {
                                        InitResult::ChatDisabled => {}
                                        InitResult::Started(inprogress_chat) => {
                                            self.inprogress_chats.insert(video_id, inprogress_chat);
                                        }
                                    },
                                    Err(e) => {
                                        // Not a hard error
                                        shared::tracing_warn!(
                                            "Couldn't initialize chat poller {}: {}",
                                            &video_id, &e
                                        );
                                    }
                                }
                            }
                        }
                        IncMessage::UpdateUserAgent(user_agent) => {
                            self.request_settings.user_agent = user_agent.clone();
                            let poller_message =
                                messages::chat_poller::IncMessage::UpdateUserAgent(user_agent);
                            self.send_message_to_pollers(poller_message).await;
                        }
                        IncMessage::UpdateBrowserVersion(version) => {
                            self.request_settings.browser_version = version.clone();
                            let poller_message =
                                messages::chat_poller::IncMessage::UpdateBrowserVersion(version);
                            self.send_message_to_pollers(poller_message).await;
                        }
                        IncMessage::UpdateBrowserNameAndVersion { name, version } => {
                            self.request_settings.browser_name = name.clone();
                            self.request_settings.browser_version = name.clone();
                            let poller_message =
                                messages::chat_poller::IncMessage::UpdateBrowserNameAndVersion {
                                    name,
                                    version,
                                };
                            self.send_message_to_pollers(poller_message).await;
                        }
                    },
                    None => {
                        // Incoming channel was closed. That should never happen,
                        // as the ChatFinder should be closed first, after receiveng the `Close` message
                        return Err(ChatManagerError::IncomingChannelClosed);
                    }
                }
            }

            self.send_message_to_pollers(messages::chat_poller::IncMessage::Ping).await;
        }
    }

    async fn send_message_to_pollers(&mut self, message: messages::chat_poller::IncMessage) {
        let buffer = HashMap::with_capacity(self.inprogress_chats.len());
        let old = replace(&mut self.inprogress_chats, buffer);

        for (video_id, mut inprogress_chat) in old.into_iter() {
            match inprogress_chat.tx.send(message.clone()).await {
                Ok(_r) => {
                    self.inprogress_chats.insert(video_id, inprogress_chat);
                },
                Err(_) => {
                    shared::tracing_info!("Chat poller {} has closed its channel, waiting for the task to finish...", &video_id);
                    match inprogress_chat.join_handle.await {
                        Ok(_r) => {
                            shared::tracing_info!("Chat poller {} finished successfully", &video_id);
                        },
                        Err(e) => {
                            shared::tracing_info!("Chat poller {} panicked: {}", &video_id, &e);
                        },
                    }
                },
            }
        }
    }

    async fn close_gracefully(&mut self) {
        shared::tracing_info!("Sending `Close` message to currently active chat pollers...");
        let close_message = messages::chat_poller::IncMessage::Close;
        self.send_message_to_pollers(close_message).await;
        for (video_id, inprogress_chat) in self.inprogress_chats.drain() {
            shared::tracing_info!("Waiting for the chat poller {} to finish...", &video_id);
            match inprogress_chat.join_handle.await {
                Ok(_r) => {
                    shared::tracing_info!("Chat poller {} finished successfully", &video_id);
                },
                Err(e) => {
                    shared::tracing_info!("Chat poller {} panicked: {}", &video_id, &e);
                },
            }
        }
    }
}
