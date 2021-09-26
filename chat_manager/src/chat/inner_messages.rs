use core::{messages::ChatManagerMessages, types::Action};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ManagerMessages {
    Close,
    FoundStreamIds(HashSet<String>),
    UpdateUserAgent(String),
    UpdateBrowserVersion(String),
    UpdateBrowserNameAndVersion {
        name: String,
        version: String
    },
    NewMessages {
        video_id: String,
        actions: Vec<Action>
    },
    StreamEnded {
        video_id: String
    }
}

impl From<ChatManagerMessages> for ManagerMessages {
    fn from(inc_message: ChatManagerMessages) -> Self {
        match inc_message {
            ChatManagerMessages::Close => ManagerMessages::Close,
            ChatManagerMessages::FoundStreamIds(stream_ids) => ManagerMessages::FoundStreamIds(stream_ids),
            ChatManagerMessages::UpdateUserAgent(user_agent) => ManagerMessages::UpdateUserAgent(user_agent),
            ChatManagerMessages::UpdateBrowserVersion(version) => ManagerMessages::UpdateBrowserVersion(version),
            ChatManagerMessages::UpdateBrowserNameAndVersion { 
                name, 
                version 
            } => ManagerMessages::UpdateBrowserNameAndVersion { name, version },
        }
    }
}

impl From<PollingResultMessages> for ManagerMessages {
    fn from(inc_message: PollingResultMessages) -> Self {
        match inc_message {
            PollingResultMessages::NewBatch { 
                video_id, 
                actions 
            } => ManagerMessages::NewMessages { video_id, actions },
            PollingResultMessages::StreamEnded { video_id } => ManagerMessages::StreamEnded { video_id },
        }
    }
}

#[derive(Clone)]
pub enum PollerMessages {
    Close,
    UpdateUserAgent(String),
    UpdateBrowserVersion(String),
    UpdateBrowserNameAndVersion {
        name: String,
        version: String
    }
}

#[derive(Debug)]
pub enum PollingResultMessages {
    NewBatch {
        video_id: String,
        actions: Vec<Action>
    },
    StreamEnded {
        video_id: String
    }
}