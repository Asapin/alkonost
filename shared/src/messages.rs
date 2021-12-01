use crate::SuspicionReason;

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
    use super::{chat_poller, DetectorDecision};

    #[derive(Debug, Clone)]
    pub enum IncMessage {
        Close,
        ChatPoller(chat_poller::OutMessage),
    }

    #[derive(Debug)]
    pub enum OutMessage {
        NewChat {
            channel: String,
            video_id: String,
        },
        ChatClosed {
            channel: String,
            video_id: String
        },
        DetectorResult {
            video_id: String,
            processed_messages: usize,
            decisions: Vec<DetectorDecision>,
        },
    }
}

pub mod alkonost {
    pub enum IncMessage {
        Close,
        AddChannel(String),
        RemoveChannel(String),
        UpdateStreamPollInterval(u64),
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion { name: String, version: String },
    }
}

#[derive(Debug)]
pub struct DetectorDecision {
    pub user: String,
    pub timestamp: u64,
    pub action: DecisionAction,
}

impl DetectorDecision {
    pub fn remove_user(user: String) -> Self {
        DetectorDecision {
            user,
            timestamp: chrono::Utc::now().timestamp() as u64,
            action: DecisionAction::Remove,
        }
    }

    pub fn add_user(user: String, reason: SuspicionReason) -> Self {
        DetectorDecision {
            user,
            timestamp: chrono::Utc::now().timestamp() as u64,
            action: DecisionAction::Add(reason),
        }
    }
}

#[derive(Debug)]
pub enum DecisionAction {
    Add(SuspicionReason),
    Remove,
}
