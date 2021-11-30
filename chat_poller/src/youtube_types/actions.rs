use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use super::generic_types::{AuthorInfo, Image, Message};

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum Action {
    #[serde(rename_all(deserialize = "camelCase"))]
    AddChatItemAction { item: MessageItem },
    #[serde(rename_all(deserialize = "camelCase"))]
    MarkChatItemAsDeletedAction { target_item_id: String },
    #[serde(rename_all(deserialize = "camelCase"))]
    MarkChatItemsByAuthorAsDeletedAction { external_channel_id: String },
    #[serde(rename_all(deserialize = "camelCase"))]
    ReplaceChatItemAction {
        target_item_id: String,
        replacement_item: MessageItem,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    AddLiveChatTickerItemAction {
        // Ticker is a small message about a superchat or a new membership,
        // that shows up on top of the chat window, and usually is duplicated by a regular AddChatItemAction,
        // that shows inside the chat window.
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    AddBannerToLiveChatCommand { banner_renderer: BannerItem },
    #[serde(rename_all(deserialize = "camelCase"))]
    RemoveBannerForLiveChatCommand { target_action_id: String },
    #[serde(rename_all(deserialize = "camelCase"))]
    ShowLiveChatTooltipCommand {},
    #[serde(rename_all(deserialize = "camelCase"))]
    ShowLiveChatActionPanelAction { panel_to_show: PanelItem },
    #[serde(rename_all(deserialize = "camelCase"))]
    UpdateLiveChatPollAction { poll_to_update: PollToUpdateItem },
    #[serde(rename_all(deserialize = "camelCase"))]
    CloseLiveChatActionPanelAction { target_panel_id: String },
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum MessageItem {
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatTextMessageRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        message: Message,
        #[serde(flatten)]
        author_info: AuthorInfo,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatMembershipItemRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        #[serde(flatten)]
        author_info: AuthorInfo,
        header_primary_text: Option<Message>, // Showed only for members who renewed their membership
        header_subtext: Option<Message>, // Showed either for new members or if the channel set custom name for their membership
        message: Option<Message>,        // Optional message
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatPaidMessageRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        message: Option<Message>,
        #[serde(flatten)]
        author_info: AuthorInfo,
        purchase_amount_text: Message,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatPaidStickerRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        #[serde(flatten)]
        author_info: AuthorInfo,
        sticker: Image,
        purchase_amount_text: Message,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatViewerEngagementMessageRenderer {
        // Standard YouTube message about protecting you privacy
        // and following community guidelines,
        // that appears as the last message in chat when you load it
        id: String,
        timestamp_usec: Option<WrappedU64>,
        message: Message,
        icon: EngagementMessageIcon,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatPlaceholderItemRenderer {
        // No idea what this message type means, because
        // messages of this type don't appear in chat.
        // Have nothing other than an id and a timestamp.
        // Parsing it for a potential future investigation.
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatModeChangeMessageRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        text: Message,
        subtext: Message,
        icon: ChatModeIcon,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatDonationAnnouncementRenderer {
        id: String,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        text: Message,
        subtext: Message,
        #[serde(flatten)]
        author_info: Option<AuthorInfo>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct BannerItem {
    pub live_chat_banner_renderer: BannerItemRenderer,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct BannerItemRenderer {
    pub contents: BannerItemContent,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum BannerItemContent {
    #[serde(rename_all(deserialize = "camelCase"))]
    LiveChatTextMessageRenderer {
        message: Message,
        #[serde(flatten)]
        author_info: AuthorInfo,
        #[serde(deserialize_with = "from_str")]
        timestamp_usec: u64,
        id: String,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    PollRenderer {
        #[serde(flatten)]
        poll_renderer: Poll,
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    DonationsProgressBarRenderer {
        raised: Message,
        campaign_title: String,
        goal_reached_label: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PanelItem {
    pub live_chat_action_panel_renderer: PanelItemRenderer,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PanelItemRenderer {
    pub contents: PanelItemItemContent,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PanelItemItemContent {
    pub poll_renderer: Poll,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Poll {
    pub choices: Vec<PollChoices>,
    pub live_chat_poll_id: String,
    pub header: PollHeader,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PollChoices {
    pub text: Message,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PollHeader {
    pub poll_header_renderer: PollHeaderRenderer,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PollHeaderRenderer {
    pub poll_question: Message,
    pub live_chat_poll_type: PollType,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum PollType {
    LiveChatPollTypeCreator,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PollToUpdateItem {
    pub poll_renderer: PollToUpdateItemRenderer,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PollToUpdateItemRenderer {
    pub choices: Vec<ResultingPollChoices>,
    pub live_chat_poll_id: String,
    pub header: PollHeader,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ResultingPollChoices {
    pub text: Message,
    pub vote_ratio: f64,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ChatModeIcon {
    pub icon_type: ChatModeIconType,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum ChatModeIconType {
    TabSubscriptions,
    SlowMode,
    QuestionAnswer,
    Memberships,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct EngagementMessageIcon {
    pub icon_type: EngagementMessageIconType,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum EngagementMessageIconType {
    YoutubeRound, // standard YouTube greeting message
    Poll,
}

#[derive(Deserialize)]
pub struct WrappedU64(#[serde(deserialize_with = "from_str")] pub u64);

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}
