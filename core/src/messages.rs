use std::collections::HashSet;

use crate::{types::Action, SuspicionReason};

#[derive(Debug)]
pub enum StreamFinderMessages {
    Close,
    AddChannel(String),
    RemoveChannel(String),
    UpdatePollInterval(u64),
    UpdateUserAgent(String),
    UpdateBrowserVersion(String),
    UpdateBrowserNameAndVersion { name: String, version: String },
}

#[derive(Debug)]
pub enum ChatManagerMessages {
    Close,
    FoundStreamIds(HashSet<String>),
    UpdateUserAgent(String),
    UpdateBrowserVersion(String),
    UpdateBrowserNameAndVersion { name: String, version: String },
}

#[derive(Debug)]
pub enum SpamDetectorMessages {
    Close,
    NewBatch {
        video_id: String,
        actions: Vec<Action>,
    },
    StreamEnded {
        video_id: String,
    },
}

#[derive(Debug)]
pub enum DetectorResults {
    Close,
    ProcessingResult {
        video_id: String,
        decisions: Vec<DetectorDecision>,
    },
    StreamEnded {
        video_id: String,
    },
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
