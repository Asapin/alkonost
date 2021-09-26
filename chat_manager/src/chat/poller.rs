use core::http_client::{HttpClient, RequestSettings};
use core::types::Action;
use core::ActorWrapper;
use std::io::Write;
use std::time::Duration;
use std::{fs::File, sync::Arc};

use tokio::sync::mpsc::{self, Sender};
use tokio::{
    sync::mpsc::Receiver,
    time::{timeout_at, Instant},
};

use crate::chat::error::ParamsExtractingError;
use crate::chat::params_extractor::{ExtractingResult, ParamsExtractor};
use crate::type_converter::Converter;
use crate::youtube_types::root::{ChatJson, Continuation};

use super::chat_params::ChatParams;
use super::error::{ActionExtractorError, PollerError};
use super::inner_messages::{PollerMessages, PollingResultMessages};

pub enum InitResult {
    Started(ActorWrapper<PollerMessages>),
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
    rx: Receiver<PollerMessages>,
    chat_manager_tx: Sender<PollingResultMessages>,
    poll_errors_count: u8,
}

impl ChatPoller {
    pub async fn init(
        video_id: String,
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        chat_manager_tx: Sender<PollingResultMessages>,
    ) -> Result<InitResult, ParamsExtractingError> {
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
            video_id,
            http_client,
            request_settings,
            referer_url: chat_url,
            endpoint_url,
            next_poll_time: Instant::now(),
            chat_params,
            rx,
            chat_manager_tx,
            poll_errors_count: 0,
        };

        let join_handle = tokio::spawn(async move {
            poller.run().await;
        });

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
                println!(
                    "{}: Error, while processing messages: {}",
                    &self.video_id, &e
                );
            }
        }

        println!(
            "{}: Sending `StreamEnded` message back to the chat manager...",
            &self.video_id
        );
        let closing_message = PollingResultMessages::StreamEnded {
            video_id: self.video_id.clone(),
        };
        match self.chat_manager_tx.send(closing_message).await {
            Ok(_r) => {
                // Successfully reported back to the chat manager.
                // Nothing else to do
            }
            Err(e) => {
                println!(
                    "{}: Couldn't send message to the chat manager: {}",
                    &self.video_id, &e
                );
            }
        }

        println!("{}: Chat poller has been closed", self.video_id);
    }

    async fn do_run(&mut self) -> Result<(), PollerError> {
        loop {
            while let Ok(recv_result) = timeout_at(self.next_poll_time, self.rx.recv()).await {
                match recv_result {
                    Some(message) => match message {
                        PollerMessages::Close => {
                            return Ok(());
                        }
                        PollerMessages::UpdateUserAgent(user_agent) => {
                            self.request_settings.user_agent = user_agent;
                        }
                        PollerMessages::UpdateBrowserVersion(version) => {
                            self.request_settings.browser_version = version;
                        }
                        PollerMessages::UpdateBrowserNameAndVersion { name, version } => {
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
                let polling_results = PollingResultMessages::NewBatch {
                    actions,
                    video_id: self.video_id.clone(),
                };
                self.chat_manager_tx.send(polling_results).await?;
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
