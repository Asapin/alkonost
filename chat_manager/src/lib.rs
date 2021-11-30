#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use chat_poller::{ChatPoller, InitResult};
use error::ChatManagerError;
use shared::{
    http_client::{HttpClient, RequestSettings},
    messages::{self, chat_manager::IncMessage},
    ActorWrapper,
};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    time::timeout,
};

mod error;

struct InprogressChat {
    actor: ActorWrapper<messages::chat_poller::IncMessage>,
    notify_close_rx: oneshot::Receiver<()>,
}

pub struct ChatManager {
    rx: Receiver<IncMessage>,
    check_children_period: Duration,
    http_client: Arc<HttpClient>,
    request_settings: RequestSettings,
    inprogress_chats: HashMap<String, InprogressChat>,
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

        ActorWrapper { join_handle, tx }
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // Chat manager was closed due to received `Close` message
            }
            Err(e) => {
                println!("ChatManager: encountered an error: {}", &e);
            }
        }

        self.close_gracefully().await;
        println!("ChatManager has been closed");
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
                                        InitResult::Started {
                                            actor,
                                            notify_close_rx,
                                        } => {
                                            let inprogress_chat = InprogressChat {
                                                actor,
                                                notify_close_rx,
                                            };
                                            self.inprogress_chats.insert(video_id, inprogress_chat);
                                        }
                                    },
                                    Err(e) => {
                                        // Not a hard error
                                        println!(
                                            "ChatManager: Couldn't initialize chat poller for {}: {}",
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

            self.remove_closed_chat_pollers().await;
        }
    }

    async fn remove_closed_chat_pollers(&mut self) {
        // Check closed chat pollers
        let mut closed_chats = HashMap::new();
        for (video_id, inprogress_chat) in self.inprogress_chats.iter_mut() {
            match inprogress_chat.notify_close_rx.try_recv() {
                Ok(_r) => {
                    closed_chats.insert(video_id.clone(), true);
                }
                Err(e) => match e {
                    oneshot::error::TryRecvError::Empty => {}
                    oneshot::error::TryRecvError::Closed => {
                        closed_chats.insert(video_id.clone(), false);
                    }
                },
            }
        }

        for (video_id, closed_safely) in closed_chats {
            if let Some(chat) = self.inprogress_chats.remove(&video_id) {
                if closed_safely {
                    println!(
                        "ChatManager: waiting for {} chat poller to finish its work",
                        &video_id
                    );
                    match chat.actor.join_handle.await {
                        Ok(_r) => {
                            println!(
                                "ChatManager: chat poller {} has finished its work",
                                &video_id
                            );
                        }
                        Err(e) => {
                            println!(
                                "ChatManager: chat poller for {} has panicked: {}",
                                &video_id, e
                            );
                        }
                    };
                } else {
                    println!(
                        "ChatManager: chat poller for {} closed before sending the notification",
                        &video_id
                    );
                    chat.actor.join_handle.abort()
                }
            }
        }
    }

    async fn send_message_to_pollers(&mut self, message: messages::chat_poller::IncMessage) {
        let mut already_closed_pollers = HashSet::new();

        for (video_id, inprogress_chat) in self.inprogress_chats.iter_mut() {
            match inprogress_chat.actor.tx.send(message.clone()).await {
                Ok(_r) => {}
                Err(_e) => {
                    // It's possible for a poller to be closed but still present in the hashmap,
                    // because the chat manager is yet to check associated notify_close_rx channel
                    already_closed_pollers.insert(video_id.clone());
                }
            }
        }

        for video_id in already_closed_pollers {
            if let Some(chat) = self.inprogress_chats.remove(&video_id) {
                ChatManager::close_chat_poller(video_id, chat).await;
            }
        }
    }

    async fn close_gracefully(&mut self) {
        println!("ChatManager: Sending `Close` message to currently active chat pollers...");
        let close_message = messages::chat_poller::IncMessage::Close;
        self.send_message_to_pollers(close_message).await;
        for (video_id, chat) in self.inprogress_chats.drain() {
            ChatManager::close_chat_poller(video_id, chat).await;
        }
    }

    async fn close_chat_poller(video_id: String, mut chat: InprogressChat) {
        match chat.notify_close_rx.try_recv() {
            Ok(_r) => {
                println!(
                    "ChatManager: waiting for {} chat poller to finish its work",
                    &video_id
                );
                match chat.actor.join_handle.await {
                    Ok(_r) => {}
                    Err(e) => {
                        println!(
                            "ChatManager: chat poller for {} has panicked: {}",
                            &video_id, e
                        );
                    }
                };
            }
            Err(e) => {
                match e {
                    oneshot::error::TryRecvError::Empty => {
                        println!(
                            "ChatManager: chat poller for {} closed it's rx without closing itself which shouldn't be possible", 
                            &video_id
                        );
                    }
                    oneshot::error::TryRecvError::Closed => {
                        println!(
                            "ChatManager: chat poller for {} closed before sending the notification", 
                            &video_id
                        );
                    }
                };
                chat.actor.join_handle.abort();
            }
        }
    }
}
