use core::http_client::HttpClientInitError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlkonostInitError {
    #[error("Couldn't initialize http client: {0}")]
    HttpClientInit(#[source] HttpClientInitError),
}

impl From<HttpClientInitError> for AlkonostInitError {
    fn from(e: HttpClientInitError) -> Self {
        Self::HttpClientInit(e)
    }
}