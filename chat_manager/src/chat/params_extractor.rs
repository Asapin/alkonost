use core::{
    http_client::{HttpClient, RequestSettings},
    youtube_regexes::YoutubeRegexes,
};

use super::{chat_params::ChatParams, error::ParamsExtractingError};

pub struct ParamsExtractor;

pub enum ExtractingResult {
    Extracted {
        chat_params: ChatParams,
        chat_key: String,
    },
    ChatDisabled,
}

impl ParamsExtractor {
    pub async fn extract_chat_params(
        video_id: &str,
        chat_url: &str,
        http_client: &HttpClient,
        request_settings: &RequestSettings,
    ) -> Result<ExtractingResult, ParamsExtractingError> {
        let chat_page_content = http_client
            .get_request(chat_url, &request_settings.user_agent)
            .await?;

        if !YoutubeRegexes::is_chat_enabled(&chat_page_content) {
            return Ok(ExtractingResult::ChatDisabled);
        }

        let gl = match YoutubeRegexes::extract_gl(&chat_page_content) {
            Some(gl) => gl,
            None => return Err(ParamsExtractingError::ExtractGl(chat_page_content)),
        };

        let remote_host = match YoutubeRegexes::extract_remote_host(&chat_page_content) {
            Some(gl) => gl,
            None => return Err(ParamsExtractingError::RemoteHost(chat_page_content)),
        };

        let visitor_data = match YoutubeRegexes::extract_visitor_data(&chat_page_content) {
            Some(gl) => gl,
            None => return Err(ParamsExtractingError::VisitorData(chat_page_content)),
        };

        let client_version = match YoutubeRegexes::extract_client_version(&chat_page_content) {
            Some(gl) => gl,
            None => return Err(ParamsExtractingError::ClientVersion(chat_page_content)),
        };

        let continuation = match YoutubeRegexes::extract_last_continuation(&chat_page_content) {
            Some(gl) => gl,
            None => return Err(ParamsExtractingError::Continuation(chat_page_content)),
        };

        let chat_key = match YoutubeRegexes::extract_chat_key(&chat_page_content) {
            Some(chat_key) => chat_key,
            None => return Err(ParamsExtractingError::ChatKey(chat_page_content)),
        };

        let now = chrono::Local::now();
        let timestamp = now.timestamp_millis();
        let offset_min = now.offset().local_minus_utc() / 60;

        let time_zone =
            YoutubeRegexes::extract_time_zone(&chat_page_content).unwrap_or("Asia/Tokyo");

        let stream_params = ChatParams::init(
            gl.to_string(),
            remote_host.to_string(),
            visitor_data.to_string(),
            request_settings.user_agent.clone(),
            client_version.to_string(),
            video_id,
            time_zone.to_string(),
            request_settings.browser_name.clone(),
            request_settings.browser_version.clone(),
            timestamp,
            offset_min,
            continuation.to_string(),
        );

        Ok(ExtractingResult::Extracted {
            chat_params: stream_params,
            chat_key: chat_key.to_string(),
        })
    }
}
