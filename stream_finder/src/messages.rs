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