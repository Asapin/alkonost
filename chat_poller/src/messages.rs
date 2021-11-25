use core::types::Action;

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