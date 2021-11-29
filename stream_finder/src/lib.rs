#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use core::{
    http_client::{HttpClient, RequestSettings},
    youtube_regexes::YoutubeRegexes,
    ActorWrapper,
    messages::stream_finder::{IncMessage, OutMessage}
};
use std::io::Write;
use std::{
    collections::HashMap,
    fs::File,
    sync::Arc,
    time::Duration,
};

use error::StreamFinderError;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::{timeout_at, Instant},
};
use video_list::VideoList;

mod error;
mod video_list;

pub struct StreamFinder {
    rx: Receiver<IncMessage>,
    result_tx: Sender<OutMessage>,
    next_poll_time: Instant,
    poll_interval: Duration,
    channels: HashMap<String, String>,
    request_settings: RequestSettings,
    http_client: Arc<HttpClient>,
}

impl StreamFinder {
    pub fn init(
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        result_tx: Sender<OutMessage>,
        poll_interval: Duration,
    ) -> ActorWrapper<IncMessage> {
        let (tx, rx) = mpsc::channel(32);

        let stream_finder = Self {
            rx,
            result_tx,
            next_poll_time: Instant::now(),
            poll_interval,
            channels: HashMap::new(),
            request_settings,
            http_client,
        };

        let join_handle = tokio::spawn(async move {
            stream_finder.run().await;
        });

        ActorWrapper { join_handle, tx }
    }

    async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {
                // StreamFinder finished it's work due to incoming `Close` message
            }
            Err(e) => {
                println!("StreamFinder: Error, while looking for new and airing streams and premiers: {}", &e);
            }
        }

        // We can do some cleaup work here before closing StreamFinder

        println!("StreamFinder has been closed");
    }

    async fn do_run(&mut self) -> Result<(), StreamFinderError> {
        loop {
            // timeout_at will return Err(Elapsed) after the timeout has been reached,
            // but that is expected and not an error, just a way to communicate, that we hit the timeout
            while let Ok(recv_result) = timeout_at(self.next_poll_time, self.rx.recv()).await {
                match recv_result {
                    Some(message) => match message {
                        IncMessage::Close => return Ok(()),
                        IncMessage::AddChannel(channel_id) => {
                            let url = format!(
                                "https://www.youtube.com/channel/{}/videos?view=57",
                                &channel_id
                            );
                            self.channels.insert(channel_id, url);
                        }
                        IncMessage::RemoveChannel(channel_id) => {
                            self.channels.remove(&channel_id);
                        }
                        IncMessage::UpdatePollInterval(interval_ms) => {
                            self.poll_interval = Duration::from_millis(interval_ms);
                        }
                        IncMessage::UpdateUserAgent(user_agent) => {
                            self.request_settings.user_agent = user_agent.clone();
                        }
                        IncMessage::UpdateBrowserVersion(version) => {
                            self.request_settings.browser_version = version.clone();
                        }
                        IncMessage::UpdateBrowserNameAndVersion { name, version } => {
                            self.request_settings.browser_name = name.clone();
                            self.request_settings.browser_version = version.clone();
                        }
                    },
                    None => {
                        // Incoming channel was closed. That should never happen,
                        // as the ChatFinder should be closed first, after receiveng the `Close` message
                        return Err(StreamFinderError::IncomingChannelClosed);
                    }
                }
            }

            self.poll_channels().await;
            self.next_poll_time = Instant::now() + self.poll_interval;
        }
    }

    async fn poll_channels(&self) {
        let poll_results: FuturesUnordered<_> = self
            .channels
            .iter()
            .map(|(channel_id, url)| {
                let channel_id = channel_id.clone();
                let channel_url = url.clone();
                let result_tx = self.result_tx.clone();

                async { self.load_streams_from_channel(channel_id, channel_url, result_tx).await }
            })
            .collect();

        for _ in poll_results.collect::<Vec<_>>().await {
            // Just waiting for all futures to complete
        }
    }

    async fn load_streams_from_channel(
        &self,
        channel_id: String,
        channel_url: String,
        result_tx: Sender<OutMessage>
    ) {
        let load_result = self
            .http_client
            .get_request(&channel_url, &self.request_settings.user_agent)
            .await;

        let channel_page = match load_result {
            Ok(data) => data,
            Err(e) => {
                println!("StreamFinder: couldn't load content from {} channel, reason: {}", channel_id, e);
                return;
            }
        };

        let video_list = match YoutubeRegexes::extract_video_list(&channel_page) {
            Some(video_list) => video_list,
            None => {
                // No scheduled or airing streams or premiers
                return;
            }
        };

        let video_list = match serde_json::from_str::<VideoList>(video_list) {
            Ok(list) => list,
            Err(e) => {
                // Dumping stream page to the logs for further investigation
                let mut request_output = match File::create(format!("{}.channel", &channel_id)) {
                    Ok(file) => file,
                    Err(io_e) => {
                        println!(
                            "StreamFinder: couldn't dump an error {} from the channel {}, reason: {}",
                            e, channel_id, io_e
                        );
                        return;
                    }
                };

                match write!(request_output, "{}", channel_page) {
                    Ok(_r) => {}
                    Err(io_e) => {
                        println!(
                            "StreamFinder: couldn't dump an error {} from the channel {}, reason: {}",
                            e, channel_id, io_e
                        );
                        return;
                    }
                }

                println!("StreamFinder: couldn't extract video list from the channel {}, reason: {}", channel_id, e);
                return;
            }
        };

        match result_tx
            .send(OutMessage { channel: channel_id.clone(), streams: video_list.streams })
            .await 
            {
                Ok(_r) => { },
                Err(e) => {
                    println!("StreamFinder: couldn't send polling results from channel {}, reason: {}", channel_id, e);
                },
            }
    }
}