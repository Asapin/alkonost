pub mod stream_finder {
    use std::collections::HashSet;

    #[derive(Debug, Clone)]
    pub enum IncMessage {
        Close,
        AddChannel(String),
        RemoveChannel(String),
        UpdatePollInterval(u64),
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion { name: String, version: String },
    }

    #[derive(Debug)]
    pub struct OutMessage {
        pub channel: String,
        pub streams: HashSet<String>,
    }
}

pub mod chat_poller {
    use crate::types::Action;

    #[derive(Debug, Clone)]
    pub enum IncMessage {
        Close,
        Ping,
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion { name: String, version: String },
    }

    #[derive(Debug, Clone)]
    pub enum OutMessage {
        ChatInit {
            channel: String,
            video_id: String,
        },
        NewBatch {
            channel: String,
            video_id: String,
            actions: Vec<Action>,
        },
        StreamEnded {
            channel: String,
            video_id: String,
        },
    }
}

pub mod chat_manager {
    use std::collections::HashSet;

    #[derive(Debug, Clone)]
    pub enum IncMessage {
        Close,
        FoundStreams {
            channel: String,
            streams: HashSet<String>,
        },
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion {
            name: String,
            version: String,
        },
    }
}

pub mod detector {
    use crate::detector_params::DetectorParams;

    use super::chat_poller;

    #[derive(Debug, Clone)]
    pub enum IncMessage {
        Close,
        ChatPoller(chat_poller::OutMessage),
        UpdateParams {
            channel: String,
            params: DetectorParams,
        },
    }

    #[derive(Debug)]
    pub enum OutMessage {
        NewChat {
            channel: String,
            video_id: String,
        },
        ChatClosed {
            channel: String,
            video_id: String,
        },
        DetectorResult {
            video_id: String,
            processed_messages: usize,
            decisions: Vec<DetectorDecision>,
        },
    }

    #[derive(Debug)]
    pub struct DetectorDecision {
        pub channel: String,
        pub timestamp: i64,
        pub decision: Decision,
    }

    impl DetectorDecision {
        pub fn new(channel: String, decision: Decision) -> Self {
            Self {
                channel,
                timestamp: chrono::Utc::now().timestamp(),
                decision,
            }
        }
    }

    #[derive(Debug)]
    pub enum Decision {
        TooFast(f32),
        TooLong(f32),
        TooManyDeleted,
        Similar,
        Blocked,
        Clear,
    }
}

pub mod alkonost {
    use crate::detector_params::DetectorParams;

    #[derive(Debug)]
    pub enum IncMessage {
        Close,
        AddChannel(String),
        RemoveChannel(String),
        UpdateStreamPollInterval(u64),
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion {
            name: String,
            version: String,
        },
        UpdateDetectorParams {
            channel: String,
            new_params: DetectorParams,
        },
    }
}
