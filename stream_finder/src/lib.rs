#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

use core::{
    http_client::{HttpClient, RequestSettings},
    messages::{ChatManagerMessages, StreamFinderMessages},
    youtube_regexes::YoutubeRegexes,
    ActorWrapper,
};
use std::io::Write;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
    time::Duration,
};

use error::{LoadError, StreamFinderError};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::{timeout_at, Instant},
};
use video_list::VideoList;

mod error;
mod video_list;

pub struct StreamFinder {
    rx: Receiver<StreamFinderMessages>,
    result_tx: Sender<ChatManagerMessages>,
    next_poll_time: Instant,
    poll_interval: Duration,
    channels: HashMap<String, String>,
    request_settings: RequestSettings,
    http_client: Arc<HttpClient>,
}

struct PollChannelsResult {
    streams: HashSet<String>,
    encountered_errors: Vec<LoadError>,
}

impl StreamFinder {
    pub fn init(
        http_client: Arc<HttpClient>,
        request_settings: RequestSettings,
        result_tx: Sender<ChatManagerMessages>,
        poll_interval: Duration,
    ) -> ActorWrapper<StreamFinderMessages> {
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
                        StreamFinderMessages::Close => return Ok(()),
                        StreamFinderMessages::AddChannel(channel_id) => {
                            let url = format!(
                                "https://www.youtube.com/channel/{}/videos?view=57",
                                &channel_id
                            );
                            self.channels.insert(channel_id, url);
                        }
                        StreamFinderMessages::RemoveChannel(channel_id) => {
                            self.channels.remove(&channel_id);
                        }
                        StreamFinderMessages::UpdatePollInterval(interval_ms) => {
                            self.poll_interval = Duration::from_millis(interval_ms);
                        }
                        StreamFinderMessages::UpdateUserAgent(user_agent) => {
                            self.request_settings.user_agent = user_agent.clone();
                        }
                        StreamFinderMessages::UpdateBrowserVersion(version) => {
                            self.request_settings.browser_version = version.clone();
                        }
                        StreamFinderMessages::UpdateBrowserNameAndVersion { name, version } => {
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

            let poll_result = self.poll_channels().await;
            for encountered_error in poll_result.encountered_errors {
                println!("StreamFinder: {}", &encountered_error);
            }
            self.report_poll_results(poll_result.streams).await?;
            self.next_poll_time = Instant::now() + self.poll_interval;
        }
    }

    async fn poll_channels(&self) -> PollChannelsResult {
        let poll_results: FuturesUnordered<_> = self
            .channels
            .iter()
            .map(|(channel_id, url)| {
                let channel_id = channel_id.clone();
                let url = url.clone();

                async { self.load_streams(channel_id, url).await }
            })
            .collect();

        let mut streams = HashSet::new();
        let mut encountered_errors = Vec::new();
        for poll_result in poll_results.collect::<Vec<_>>().await {
            match poll_result {
                Ok(video_ids) => {
                    if let Some(ids) = video_ids {
                        streams.extend(ids.into_iter())
                    }
                }
                Err(e) => {
                    encountered_errors.push(e);
                }
            }
        }

        PollChannelsResult {
            streams,
            encountered_errors,
        }
    }

    async fn load_streams(
        &self,
        channel_id: String,
        channel_url: String,
    ) -> Result<Option<HashSet<String>>, LoadError> {
        let load_result = self
            .http_client
            .get_request(&channel_url, &self.request_settings.user_agent)
            .await;

        let channel_page = match load_result {
            Ok(data) => data,
            Err(e) => {
                return Err(LoadError::LoadContent(channel_id, e));
            }
        };

        let video_list = match YoutubeRegexes::extract_video_list(&channel_page) {
            Some(video_list) => video_list,
            None => {
                // No scheduled or airing streams or premiers
                return Ok(None);
            }
        };

        let video_list = match serde_json::from_str::<VideoList>(video_list) {
            Ok(list) => list,
            Err(e) => {
                // Dumping stream page to the logs for further investigation
                let mut request_output = match File::create(format!("{}.channel", &channel_id)) {
                    Ok(file) => file,
                    Err(io_e) => {
                        return Err(LoadError::DumpError(channel_id, e, io_e));
                    }
                };

                match write!(request_output, "{}", channel_page) {
                    Ok(_r) => {}
                    Err(io_e) => {
                        return Err(LoadError::DumpError(channel_id, e, io_e));
                    }
                }

                return Err(LoadError::VideoList(channel_id, e));
            }
        };

        Ok(Some(video_list.streams))
    }

    async fn report_poll_results(
        &self,
        airing_streams: HashSet<String>,
    ) -> Result<(), StreamFinderError> {
        self.result_tx
            .send(ChatManagerMessages::FoundStreamIds(airing_streams))
            .await
            .map_err(|e| e.into())
    }
}
