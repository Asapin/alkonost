use std::time::Duration;

use reqwest::{
    header::{self, ACCEPT, ACCEPT_LANGUAGE, DNT, REFERER, UPGRADE_INSECURE_REQUESTS, USER_AGENT},
    Client,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpClientInitError {
    #[error("Reqwest init error: {0}")]
    ReqwestClientCreate(#[source] reqwest::Error),
}

impl From<reqwest::Error> for HttpClientInitError {
    fn from(e: reqwest::Error) -> Self {
        HttpClientInitError::ReqwestClientCreate(e)
    }
}

#[derive(Error, Debug)]
pub enum HttpClientLoadError {
    #[error("Couldn't perform GET request: {0}")]
    GetRequest(#[source] reqwest::Error),
    #[error("Couldn't perform POST request: {0}")]
    PostRequest(#[source] reqwest::Error),
    #[error("Couldn't load response body: {0}")]
    ResponseBody(#[source] reqwest::Error),
}

#[derive(Clone)]
pub struct RequestSettings {
    pub user_agent: String,
    pub browser_name: String,
    pub browser_version: String,
}

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn init() -> Result<HttpClient, HttpClientInitError> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            ACCEPT,
            header::HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("en-US,en;q=0.5"),
        );
        headers.insert(DNT, header::HeaderValue::from_static("1"));
        headers.insert(
            UPGRADE_INSECURE_REQUESTS,
            header::HeaderValue::from_static("1"),
        );

        let client = reqwest::ClientBuilder::new()
            .tcp_keepalive(Some(Duration::from_secs(120)))
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .referer(false)
            .default_headers(headers)
            .use_rustls_tls()
            .build()?;

        Ok(HttpClient { client })
    }

    pub async fn get_request(
        &self,
        url: &str,
        user_agent: &str,
    ) -> Result<String, HttpClientLoadError> {
        self.client
            .get(url)
            .header(USER_AGENT, user_agent)
            .send()
            .await
            .map_err(HttpClientLoadError::GetRequest)?
            .text()
            .await
            .map_err(HttpClientLoadError::ResponseBody)
    }

    pub async fn post_request(
        &self,
        url: &str,
        user_agent: &str,
        referer: &str,
        body: String,
    ) -> Result<String, HttpClientLoadError> {
        self.client
            .post(url)
            .header(USER_AGENT, user_agent)
            .header(REFERER, referer)
            .body(body)
            .send()
            .await
            .map_err(HttpClientLoadError::PostRequest)?
            .text()
            .await
            .map_err(HttpClientLoadError::ResponseBody)
    }
}
