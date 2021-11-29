use crate::SuspicionReason;

pub mod stream_finder {
    use std::collections::HashSet;

    #[derive(Debug)]
    pub enum IncMessages {
        Close,
        AddChannel(String),
        RemoveChannel(String),
        UpdatePollInterval(u64),
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion { name: String, version: String },
    }

    #[derive(Debug)]
    pub enum OutMessages {
        NewStreams { channel: String, streams: HashSet<String> }
    }
}

pub mod chat_poller {
    use crate::types::Action;

    #[derive(Debug, Clone)]
    pub enum IncMessages {
        Close,
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion { name: String, version: String },
    }

    #[derive(Debug)]
    pub enum OutMessages {
        ChatInit {
            channel: String,
            video_id: String,
        },
        NewBatch {
            video_id: String,
            actions: Vec<Action>,
        },
        StreamEnded {
            video_id: String,
        },
    }
}

pub mod chat_manager {
    use std::collections::HashSet;

    #[derive(Debug, Clone)]
    pub enum IncMessages {
        Close,
        FoundStreams {
            channel: String,
            streams: HashSet<String>
        },
        UpdateUserAgent(String),
        UpdateBrowserVersion(String),
        UpdateBrowserNameAndVersion {
            name: String,
            version: String,
        }
    }
}

pub mod detector {
    use super::{DetectorDecision, chat_poller};

    pub enum IncMessages {
        Close,
        ChatPoller(chat_poller::OutMessages)
    }

    #[derive(Debug)]
    pub enum OutMessages {
        ChatClosed(String),
        DetectorResult {
            video_id: String,
            decisions: Vec<DetectorDecision>,
        }
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
