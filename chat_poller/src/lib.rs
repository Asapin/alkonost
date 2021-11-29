#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use core::http_client::{HttpClient, RequestSettings};
use core::types::Action;
use core::ActorWrapper;
use core::messages::chat_poller::{IncMessage, OutMessage};
use std::io::Write;
use std::time::Duration;
use std::{fs::File, sync::Arc};

use chat_params::ChatParams;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;
use tokio::{
    sync::mpsc::Receiver,
    time::{timeout_at, Instant},
};
use type_converter::Converter;
use youtube_types::root::{ChatJson, Continuation};

use crate::params_extractor::{ExtractingResult, ParamsExtractor};
use crate::error::{InitError, PollerError, ActionExtractorError};

mod chat_params;
mod params_extractor;
mod type_converter;
mod youtube_types;
pub mod error;

pub enum InitResult {
    Started { 
        actor: ActorWrapper<IncMessage>,
        notify_close_rx: oneshot::Receiver<()>
    },
    ChatDisabled,
}

pub struct ChatPoller {
    video_id: String,
    http_client: Arc<HttpClient>,
    request_settings: RequestSettings,
    referer_url: String,
    endpoint_url: String,
    next_poll_time: Instant,
    chat_params: ChatParams,
    rx: Receiver<IncMessage>,
    result_tx: Sender<OutMessage>,
    poll_errors_count: u8,
    notify_close_tx: oneshot::Sender<()>
}

impl ChatPoller {
    pub async fn init(
        video_id: String,
        channel_id: String,
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        result_tx: Sender<OutMessage>,
    ) -> Result<InitResult, InitError> {
        let chat_url = format!(
            "https://www.youtube.com/live_chat?is_popout=1&v={}",
            &video_id
        );
        let extract_result = ParamsExtractor::extract_chat_params(
            &video_id,
            &chat_url,
            &http_client,
            &request_settings,
        )
        .await?;

        let (chat_params, chat_key) = match extract_result {
            ExtractingResult::ChatDisabled => {
                return Ok(InitResult::ChatDisabled);
            }
            ExtractingResult::Extracted {
                chat_params,
                chat_key,
            } => (chat_params, chat_key),
        };

        let (tx, rx) = mpsc::channel(32);
        let (notify_close_tx, notify_close_rx) = oneshot::channel();
        let endpoint_url = format!(
            "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
            &chat_key
        );

        let poller = Self {
            video_id: video_id.clone(),
            http_client,
            request_settings,
            referer_url: chat_url,
            endpoint_url,
            next_poll_time: Instant::now(),
            chat_params,
            rx,
            result_tx,
            poll_errors_count: 0,
            notify_close_tx
        };

        poller.result_tx.send(OutMessage::ChatInit { channel: channel_id, video_id }).await?;

        let join_handle = tokio::spawn(async move {
            poller.run().await;
        });

        let wraper = ActorWrapper { join_handle, tx };
        Ok(InitResult::Started { actor: wraper, notify_close_rx })
    }

    async fn run(mut self) {
        let result = self.do_run().await;
        match result {
            Ok(_r) => {
                // Chat poller finished its work because the stream has ended and the chat room has been closed
                // or because the poller received `Close` message
            }
            Err(e) => {
                println!(
                    "{}: Error, while processing messages: {}",
                    &self.video_id, &e
                );
            }
        }

        println!(
            "{}: Sending `StreamEnded` message...",
            &self.video_id
        );
        let closing_message = OutMessage::StreamEnded {
            video_id: self.video_id.clone(),
        };
        match self.result_tx.send(closing_message).await {
            Ok(_r) => {
                // Nothing else to do
            }
            Err(e) => {
                println!(
                    "{}: Couldn't send `StreamEnded` message: {}",
                    &self.video_id, &e
                );
            }
        }

        println!("{}: Notifying that the module has stopped...", self.video_id);
        match self.notify_close_tx.send(()) {
            Ok(_r) => {
                // Nothing else to do
            },
            Err(_e) => {
                println!("{}: Couldn't notify that module has stopped", self.video_id);
            },
        }

        println!("{}: Chat poller has been closed", self.video_id);
    }

    async fn do_run(&mut self) -> Result<(), PollerError> {
        loop {
            while let Ok(recv_result) = timeout_at(self.next_poll_time, self.rx.recv()).await {
                match recv_result {
                    Some(message) => match message {
                        IncMessage::Close => {
                            return Ok(());
                        }
                        IncMessage::UpdateUserAgent(user_agent) => {
                            self.request_settings.user_agent = user_agent;
                        }
                        IncMessage::UpdateBrowserVersion(version) => {
                            self.request_settings.browser_version = version;
                        }
                        IncMessage::UpdateBrowserNameAndVersion { name, version } => {
                            self.request_settings.browser_name = name;
                            self.request_settings.browser_version = version;
                        }
                    },
                    None => {
                        return Err(PollerError::ChannelClosed);
                    }
                }
            }

            let chat_json = self.load_new_messages().await?;
            let (actions, continuation) = match Self::extract_messages_from_json(&chat_json) {
                Ok((actions, continuation)) => (actions, continuation),
                Err(e) => {
                    let mut response_output = match File::create(format!("{}.rsp", &self.video_id))
                    {
                        Ok(file) => file,
                        Err(io_e) => return Err(PollerError::DumpError(e, io_e)),
                    };

                    match write!(response_output, "{}", &chat_json) {
                        Ok(_r) => {}
                        Err(io_e) => return Err(PollerError::DumpError(e, io_e)),
                    };

                    return Err(e.into());
                }
            };

            match continuation {
                Some(continuation) => {
                    self.next_poll_time =
                        Instant::now() + Duration::from_millis(continuation.timeout_ms as u64);
                    self.chat_params
                        .update_continuation(continuation.continuation);
                }
                None => {
                    return Ok(());
                }
            }

            if let Some(actions) = actions {
                let polling_results = OutMessage::NewBatch {
                    actions,
                    video_id: self.video_id.clone(),
                };
                self.result_tx.send(polling_results).await?;
            }
        }
    }

    async fn load_new_messages(&mut self) -> Result<String, PollerError> {
        let body = serde_json::to_string(&self.chat_params)?;

        loop {
            let chat_response = self
                .http_client
                .post_request(
                    &self.endpoint_url,
                    &self.request_settings.user_agent,
                    &self.referer_url,
                    body.clone(),
                )
                .await;
            match chat_response {
                Ok(response) => {
                    self.poll_errors_count = 0;
                    break Ok(response);
                }
                Err(e) => {
                    self.poll_errors_count += 1;
                    if self.poll_errors_count > 2 {
                        return Err(e.into());
                    }
                    println!(
                        "{}: Encountered an error while polling for new messages. Retrying in 100ms. Attempt #: {}. Error: {}",
                        &self.video_id,
                        self.poll_errors_count,
                        &e
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    fn extract_messages_from_json(
        json: &str,
    ) -> Result<(Option<Vec<Action>>, Option<Continuation>), ActionExtractorError> {
        let chat_json = serde_json::from_str::<ChatJson>(json)?;

        let continuation = chat_json.continuation;
        let actions = chat_json.actions.map(Converter::convert).transpose()?;

        Ok((actions, continuation))
    }
}
