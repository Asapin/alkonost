#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use std::{fs::File, io::Write, sync::Arc, time::Duration};

use chat_params::ChatParams;
use error::{ActionExtractorError, InitError, PollerError};
use params_extractor::{ExtractingResult, ParamsExtractor};
use shared::{
    http_client::{HttpClient, RequestSettings},
    messages::chat_poller::{IncMessage, OutMessage},
    types::Action,
    ActorWrapper,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::{timeout_at, Instant},
};
use type_converter::Converter;
use youtube_types::root::{ChatJson, Continuation};

mod chat_params;
pub mod error;
mod params_extractor;
mod type_converter;
mod youtube_types;

pub enum InitResult {
    Started(ActorWrapper<IncMessage>),
    ChatDisabled,
}

pub struct ChatPoller {
    channel: String,
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
}

impl ChatPoller {
    pub async fn init(
        video_id: String,
        channel: String,
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
        let endpoint_url = format!(
            "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
            &chat_key
        );

        let poller = Self {
            channel: channel.clone(),
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
        };

        poller
            .result_tx
            .send(OutMessage::ChatInit { channel, video_id })
            .await?;

        let join_handle = tokio::spawn(async move {
            poller.run().await;
        });

        let tx = shared::AlkSender::new(tx, "ChatPoller_tx".to_string());
        let wraper = ActorWrapper { join_handle, tx };
        Ok(InitResult::Started(wraper))
    }

    async fn run(mut self) {
        let result = self.do_run().await;
        match result {
            Ok(_r) => {
                // Chat poller finished its work because the stream has ended and the chat room has been closed
                // or because the poller received `Close` message
            }
            Err(e) => {
                shared::tracing_error!(
                    "{}: Error, while processing messages: {}",
                    &self.video_id,
                    &e
                );
            }
        }

        shared::tracing_info!("{}: Sending `StreamEnded` message...", &self.video_id);
        let closing_message = OutMessage::StreamEnded {
            channel: self.channel.clone(),
            video_id: self.video_id.clone(),
        };
        match self.result_tx.send(closing_message).await {
            Ok(_r) => {
                // Nothing else to do
            }
            Err(e) => {
                shared::tracing_error!(
                    "{}: Couldn't send `StreamEnded` message: {}",
                    &self.video_id,
                    &e
                );
            }
        }

        shared::tracing_info!("{}: Chat poller has been closed", self.video_id);
    }

    async fn do_run(&mut self) -> Result<(), PollerError> {
        loop {
            while let Ok(recv_result) = timeout_at(self.next_poll_time, self.rx.recv()).await {
                match recv_result {
                    Some(message) => match message {
                        IncMessage::Close => {
                            return Ok(());
                        }
                        IncMessage::Ping => {
                            // Do nothing
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
                    channel: self.channel.clone(),
                    video_id: self.video_id.clone(),
                    actions,
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
                    shared::tracing_warn!(
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
