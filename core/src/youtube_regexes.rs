use lazy_regex::{Lazy, lazy_regex};
use lazy_regex::Regex;

static VIDEO_LIST: Lazy<Regex> = lazy_regex!(r#"ytInitialData[ =]+(?P<video_list>.+});</script>"#);
static CHAT_EXISTS: Lazy<Regex> = lazy_regex!(r#"liveChatRenderer"#);
static GL: Lazy<Regex> = lazy_regex!(r#"gl\W+(?P<gl>[\w.]+)"#);
static REMOTE_HOST: Lazy<Regex> = lazy_regex!(r#"remoteHost\W+(?P<remote_host>[\d.]+)"#);
static VISITOR_DATA: Lazy<Regex> = lazy_regex!(r#"visitorData\W+(?P<visitor_data>[\w%]+)"#);
static TIME_ZONE: Lazy<Regex> = lazy_regex!(r#"timeZone\W+(?P<time_zone>[\w/]+)"#);
static RELOAD_CONTINUATION: Lazy<Regex> = lazy_regex!(r#"reloadContinuationData\W+(?P<reload_continuation>[\w: %,\-"]+)"#);
static CONTINUATION: Lazy<Regex> = lazy_regex!(r#"continuation\W+(?P<continuation>[\w%\-]+)"#);
static CLIENT_VERSION: Lazy<Regex> = lazy_regex!(r#"clientVersion\W+(?P<client_version>[\w.]+)"#);
static CHAT_KEY: Lazy<Regex> = lazy_regex!(r#"INNERTUBE_API_KEY\W+(?P<chat_key>\w+)\W"#);

pub struct YoutubeRegexes;

impl YoutubeRegexes {
    pub fn extract_video_list<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_last_pattern(data, &VIDEO_LIST, "video_list")
    }

    pub fn is_chat_enabled(text_data: &str) -> bool {
        CHAT_EXISTS.is_match(&text_data)
    }

    pub fn extract_gl<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &GL, "gl")
    }

    pub fn extract_remote_host<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &REMOTE_HOST, "remote_host")
    }

    pub fn extract_visitor_data<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &VISITOR_DATA, "visitor_data")
    }

    pub fn extract_time_zone<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &TIME_ZONE, "time_zone")
    }

    pub fn extract_client_version<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &CLIENT_VERSION, "client_version")
    }

    pub fn extract_last_continuation<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_last_pattern(data, &RELOAD_CONTINUATION, "reload_continuation")
            .map(|reload_continuation| YoutubeRegexes::capture_pattern(reload_continuation, &CONTINUATION, "continuation"))
            .flatten()
    }

    pub fn extract_chat_key<'a>(data: &'a str) -> Option<&'a str> {
        YoutubeRegexes::capture_pattern(data, &CHAT_KEY, "chat_key")
    }

    fn capture_pattern<'a>(
        text_data: &'a str,
        pattern: &Regex,
        group_name: &str,
    ) -> Option<&'a str> {
        pattern
            .captures(text_data)
            .map(|capture| capture.name(group_name))
            .flatten()
            .map(|m| m.as_str())
    }

    fn capture_last_pattern<'a>(
        text_data: &'a str,
        pattern: &Regex,
        group_name: &str,
    ) -> Option<&'a str> {
        pattern
            .captures_iter(text_data)
            .last()
            .map(|capture| capture.name(group_name))
            .flatten()
            .map(|m| m.as_str())
    }
}