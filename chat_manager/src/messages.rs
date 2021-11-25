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