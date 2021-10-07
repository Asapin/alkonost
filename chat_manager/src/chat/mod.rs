use core::{
    http_client::{HttpClient, RequestSettings},
    messages::{ChatManagerMessages, SpamDetectorMessages},
    ActorWrapper,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
    time::timeout,
};

use crate::chat::inner_messages::ManagerMessages;

use self::{
    error::ChatManagerError,
    inner_messages::{PollerMessages, PollingResultMessages},
    poller::{ChatPoller, InitResult},
    retranslator::Retranslator,
};

mod chat_params;
mod error;
mod inner_messages;
mod params_extractor;
mod poller;
mod retranslator;

pub struct ChatManager {
    poller_tx: Sender<PollingResultMessages>,
    manager_rx: Receiver<ManagerMessages>,
    incoming_proxy_join_handler: JoinHandle<()>,
    poller_proxy_join_handler: JoinHandle<()>,

    http_client: Arc<HttpClient>,
    request_settings: RequestSettings,
    inprogress_chats: HashMap<String, ActorWrapper<PollerMessages>>,
    detector_tx: Sender<SpamDetectorMessages>,
}

impl ChatManager {
    pub fn init(
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        detector_tx: Sender<SpamDetectorMessages>,
    ) -> ActorWrapper<ChatManagerMessages> {
        let (outside_tx, outside_rx) = mpsc::channel(32);
        let (manager_tx, manager_rx) = mpsc::channel(32);
        let (poller_tx, poller_rx) = mpsc::channel(32);

        let mut incoming_proxy = Retranslator::init(
            outside_rx,
            manager_tx.clone(),
            Box::new(|message| matches!(message, ManagerMessages::Close)),
        );

        let incoming_proxy_join_handler = tokio::spawn(async move {
            let _result = incoming_proxy.run().await;
        });

        let mut poller_proxy = Retranslator::init(poller_rx, manager_tx, Box::new(|_| false));

        let poller_proxy_join_handler = tokio::spawn(async move {
            let _result = poller_proxy.run().await;
        });

        let manager = Self {
            poller_tx,
            manager_rx,
            incoming_proxy_join_handler,
            poller_proxy_join_handler,
            http_client,
            request_settings,
            inprogress_chats: HashMap::with_capacity(20),
            detector_tx,
        };

        let join_handle = tokio::spawn(async move {
            let _result = manager.run().await;
        });

        ActorWrapper {
            join_handle,
            tx: outside_tx,
        }
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

        match self.close_gracefully().await {
            Ok(_r) => {
                // All active chat pollers were closed successfully
            }
            Err(e) => {
                println!("ChatManager: encountered an error while closing: {}", &e);
            }
        }

        // Closing the rest of resources, including those, that weren't previously closed due to an error
        self.shutdown().await;
        println!("ChatManager has been closed");
    }

    async fn do_run(&mut self) -> Result<(), ChatManagerError> {
        loop {
            let message = match self.manager_rx.recv().await {
                Some(message) => message,
                None => return Err(ChatManagerError::ReceiverClosed),
            };

            match timeout(
                Duration::from_millis(10_000),
                self.process_message(message.clone()),
            )
            .await
            {
                Ok(r) => match r? {
                    true => return Ok(()),
                    false => {}
                },
                Err(_e) => return Err(ChatManagerError::SlowProcessing(message)),
            }
        }
    }

    async fn process_message(
        &mut self,
        message: ManagerMessages,
    ) -> Result<bool, ChatManagerError> {
        match message {
            ManagerMessages::Close => {
                return Ok(true);
            }
            ManagerMessages::FoundStreamIds(stream_ids) => {
                let new_streams = stream_ids
                    .into_iter()
                    .filter(|video_id| !self.inprogress_chats.contains_key(video_id))
                    .collect::<Vec<_>>();

                for video_id in new_streams {
                    match ChatPoller::init(
                        video_id.clone(),
                        self.http_client.clone(),
                        self.request_settings.clone(),
                        self.poller_tx.clone(),
                    )
                    .await
                    {
                        Ok(r) => match r {
                            InitResult::ChatDisabled => {}
                            InitResult::Started(actor) => {
                                self.inprogress_chats.insert(video_id, actor);
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
            ManagerMessages::UpdateUserAgent(user_agent) => {
                self.request_settings.user_agent = user_agent.clone();
                let poller_message = PollerMessages::UpdateUserAgent(user_agent);
                self.send_message_to_pollers(poller_message).await?;
            }
            ManagerMessages::UpdateBrowserVersion(version) => {
                self.request_settings.browser_version = version.clone();
                let poller_message = PollerMessages::UpdateBrowserVersion(version);
                self.send_message_to_pollers(poller_message).await?;
            }
            ManagerMessages::UpdateBrowserNameAndVersion { name, version } => {
                self.request_settings.browser_name = name.clone();
                self.request_settings.browser_version = name.clone();
                let poller_message = PollerMessages::UpdateBrowserNameAndVersion { name, version };
                self.send_message_to_pollers(poller_message).await?;
            }
            ManagerMessages::NewMessages { video_id, actions } => {
                let detector_message = SpamDetectorMessages::NewBatch { video_id, actions };
                self.detector_tx.send(detector_message).await?;
            }
            ManagerMessages::StreamEnded { video_id } => {
                if let Some(actor) = self.inprogress_chats.remove(&video_id) {
                    actor.join_handle.await?;
                    let detector_message = SpamDetectorMessages::StreamEnded { video_id };
                    self.detector_tx.send(detector_message).await?;
                } else {
                    println!("ChatManager: Received `Close` from the {} chat, but it already has been closed", &video_id);
                }
            }
        };

        Ok(false)
    }

    async fn send_message_to_pollers(
        &mut self,
        message: PollerMessages,
    ) -> Result<(), ChatManagerError> {
        let mut already_closed_pollers = HashSet::new();

        for (video_id, actor) in self.inprogress_chats.iter_mut() {
            match actor.tx.send(message.clone()).await {
                Ok(_r) => {}
                Err(_e) => {
                    // It's possible for a poller to be closed but still present in the hashmap,
                    // because the chat manager is yet to process the StreamEnded message from the poller.
                    already_closed_pollers.insert(video_id.clone());
                }
            }
        }

        for video_id in already_closed_pollers {
            match self.inprogress_chats.remove(&video_id) {
                Some(chat_poller) => {
                    // Chat poller should've already sent the StreamEnded message, meaning it's safe for us
                    // to await on its join handle.
                    chat_poller.join_handle.await?;
                    let detector_message = SpamDetectorMessages::StreamEnded { video_id };
                    self.detector_tx.send(detector_message).await?;
                }
                None => {
                    return Err(ChatManagerError::ChatPollerDisappeared);
                }
            }
        }

        Ok(())
    }

    async fn close_gracefully(&mut self) -> Result<(), ChatManagerError> {
        println!("ChatManager: Sending `Close` message to currently active chat pollers...");
        let poller_message = PollerMessages::Close;
        self.send_message_to_pollers(poller_message).await?;
        for (_, actor) in self.inprogress_chats.drain() {
            let _r = actor.join_handle.await?;
        }

        while let Ok(message) = timeout(Duration::from_millis(5000), self.manager_rx.recv()).await {
            if let Some(content) = message {
                match content {
                    ManagerMessages::Close
                    | ManagerMessages::FoundStreamIds(_)
                    | ManagerMessages::UpdateUserAgent(_)
                    | ManagerMessages::UpdateBrowserVersion(_)
                    | ManagerMessages::UpdateBrowserNameAndVersion { .. } => {
                        return Err(ChatManagerError::UseAfterClosing);
                    }
                    ManagerMessages::NewMessages { video_id, actions } => {
                        let detector_message = SpamDetectorMessages::NewBatch { video_id, actions };
                        self.detector_tx.send(detector_message).await?;
                    }
                    ManagerMessages::StreamEnded { video_id } => {
                        let detector_message = SpamDetectorMessages::StreamEnded { video_id };
                        self.detector_tx.send(detector_message).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn shutdown(mut self) {
        println!("ChatManager: Shutting down incoming proxy...");
        match self.incoming_proxy_join_handler.await {
            Ok(_r) => {}
            Err(e) => {
                println!(
                    "ChatManager: Error, while waiting for an incoming proxy to close: {}",
                    ChatManagerError::from(e)
                );
            }
        }

        println!("ChatManager: Aborting ChatPollers that weren't closed before...");
        for (_, actor) in self.inprogress_chats.drain() {
            actor.join_handle.abort();
        }

        println!("ChatManager: Shutting down chat poller proxy...");
        self.poller_proxy_join_handler.abort();

        println!("ChatManager: Sending `Close` message to the spam detector...");
        let detector_message = SpamDetectorMessages::Close;
        match self.detector_tx.send(detector_message).await {
            Ok(_r) => {}
            Err(e) => {
                println!("ChatManager: Error, while trying to send `Close` message to the spam detector: {}", &e);
            }
        }
    }
}
