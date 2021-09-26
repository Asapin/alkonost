use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct MainWebAppInfo {
    graft_url: String,
    web_display_mode: String,
    is_web_native_share_available: bool,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Client {
    hl: String,
    gl: String,
    remote_host: String,
    device_make: String,
    device_model: String,
    visitor_data: String,
    user_agent: String,
    client_name: String,
    client_version: String,
    os_name: String,
    os_version: String,
    original_url: String,
    screen_pixel_density: u8,
    platform: String,
    client_form_factor: String,
    screen_density_float: f32,
    user_interface_theme: String,
    time_zone: String,
    browser_name: String,
    browser_version: String,
    screen_width_points: u16,
    screen_height_points: u16,
    utc_offset_minutes: i32,
    main_app_web_info: MainWebAppInfo,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct User {
    locked_safety_mode: bool,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Request {
    use_ssl: bool,
    internal_experiment_flags: Vec<String>,
    consistency_token_jars: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct AdParam {
    key: String,
    value: String,
}

impl AdParam {
    fn new(key: &str, value: &str) -> AdParam {
        AdParam {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct AdSignalsInfo {
    params: Vec<AdParam>,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Context {
    client: Client,
    user: User,
    request: Request,
    ad_signals_info: AdSignalsInfo,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct WebClientInfo {
    is_document_hidden: bool,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct ChatParams {
    context: Context,
    continuation: String,
    web_client_info: WebClientInfo,
}

impl ChatParams {
    pub fn init(
        gl: String,
        remote_host: String,
        visitor_data: String,
        user_agent: String,
        client_version: String,
        video_id: &str,
        time_zone: String,
        browser_name: String,
        browser_version: String,
        timestamp: i64,
        utc_offset: i32,
        continuation: String,
    ) -> ChatParams {
        let main_app_web_info = MainWebAppInfo {
            graft_url: format!(
                "https://www.youtube.com/live_chat?is_popout=1&v={}",
                &video_id
            ),
            web_display_mode: "WEB_DISPLAY_MODE_BROWSER".to_string(),
            is_web_native_share_available: false,
        };

        let client = Client {
            hl: "en".to_string(),
            gl,
            remote_host,
            device_make: "".to_string(),
            device_model: "".to_string(),
            visitor_data,
            user_agent,
            client_name: "WEB".to_string(),
            client_version,
            os_name: "Windows".to_string(),
            os_version: "10.0".to_string(),
            original_url: format!(
                "https://www.youtube.com/live_chat?is_popout=1&v={}",
                &video_id
            ),
            screen_pixel_density: 1,
            platform: "DESKTOP".to_string(),
            client_form_factor: "UNKNOWN_FORM_FACTOR".to_string(),
            screen_density_float: 1.25,
            user_interface_theme: "USER_INTERFACE_THEME_DARK".to_string(),
            time_zone,
            browser_name,
            browser_version,
            screen_width_points: 1536,
            screen_height_points: 464,
            utc_offset_minutes: utc_offset,
            main_app_web_info,
        };

        let user = User {
            locked_safety_mode: false,
        };

        let request = Request {
            use_ssl: true,
            internal_experiment_flags: vec![],
            consistency_token_jars: vec![],
        };

        let mut params = Vec::with_capacity(20);
        params.push(AdParam::new("dt", &format!("{}", timestamp)));
        params.push(AdParam::new("flash", "0"));
        params.push(AdParam::new("frm", "0"));
        params.push(AdParam::new("u_tz", &format!("{}", utc_offset)));
        params.push(AdParam::new("u_his", "3"));
        params.push(AdParam::new("u_java", "false"));
        params.push(AdParam::new("u_h", "864"));
        params.push(AdParam::new("u_w", "1536"));
        params.push(AdParam::new("u_ah", "864"));
        params.push(AdParam::new("u_aw", "1536"));
        params.push(AdParam::new("u_cd", "24"));
        params.push(AdParam::new("u_nplug", "0"));
        params.push(AdParam::new("u_nmime", "0"));
        params.push(AdParam::new("bc", "31"));
        params.push(AdParam::new("bih", "464"));
        params.push(AdParam::new("biw", "1536"));
        params.push(AdParam::new(
            "brdim",
            "1529,857,1529,857,1536,0,1536,864,1536,464",
        ));
        params.push(AdParam::new("vis", "1"));
        params.push(AdParam::new("wgl", "true"));
        params.push(AdParam::new("ca_type", "image"));

        let ad_signals_info = AdSignalsInfo { params };

        let context = Context {
            client,
            user,
            request,
            ad_signals_info,
        };

        let web_client_info = WebClientInfo {
            is_document_hidden: false,
        };

        ChatParams {
            context,
            continuation,
            web_client_info,
        }
    }

    pub fn update_continuation(&mut self, new_continuation: String) {
        self.continuation = new_continuation;
    }
}
