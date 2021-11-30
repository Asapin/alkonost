use std::convert::{TryFrom, TryInto};

use shared::types::{MembershipType, UserBadges};
use vec1::Vec1;

use crate::youtube_types::{
    actions::{
        Action, BannerItem, BannerItemContent, ChatModeIconType, EngagementMessageIconType,
        MessageItem, PanelItem, PollToUpdateItem, PollType,
    },
    generic_types::{
        AuthorBadge, AuthorBadgeIcon, AuthorBadgeIconType, AuthorInfo, LinkUrl, Message,
        MessageContent,
    },
};

use super::ConverterError;

type YouTubeAction = crate::youtube_types::actions::Action;
type CoreAction = shared::types::Action;

impl From<LinkUrl> for String {
    fn from(value: LinkUrl) -> Self {
        match value.endpoint {
            crate::youtube_types::generic_types::LinkEndpoint::UrlEndpoint { url } => url,
            crate::youtube_types::generic_types::LinkEndpoint::WatchEndpoint { video_id } => {
                format!("https://youtu.be/{}", video_id)
            }
        }
    }
}

impl From<Message> for shared::types::RichText {
    fn from(value: Message) -> Self {
        match value {
            Message::SimpleText(text) => html_escape::encode_text(&text).into_owned(),
            Message::Runs(runs) => runs
                .into_iter()
                .map(|content| match content {
                    MessageContent::Link { text, url } => {
                        let url: String = url.into();
                        format!(
                            r#"<a href="{}" target="_blank">{}</a>"#,
                            url,
                            html_escape::encode_text(&text)
                        )
                    }
                    MessageContent::Text {
                        text,
                        bold,
                        italics,
                    } => {
                        let mut result = html_escape::encode_text(&text).into_owned();
                        if bold {
                            result = format!("<strong>{}</strong>", result);
                        }
                        if italics {
                            result = format!("<em>{}</em>", result);
                        }
                        result
                    }
                    MessageContent::Emoji(emoji) => emoji.emoji_id,
                })
                .collect::<Vec<String>>()
                .join(""),
        }
    }
}

impl From<PollType> for shared::types::PollType {
    fn from(value: PollType) -> Self {
        match value {
            PollType::LiveChatPollTypeCreator => shared::types::PollType::PollTypeCreator,
        }
    }
}

impl From<PanelItem> for CoreAction {
    fn from(value: PanelItem) -> Self {
        CoreAction::StartPoll {
            id: value
                .live_chat_action_panel_renderer
                .contents
                .poll_renderer
                .live_chat_poll_id,
            question: value
                .live_chat_action_panel_renderer
                .contents
                .poll_renderer
                .header
                .poll_header_renderer
                .poll_question
                .into(),
            choices: value
                .live_chat_action_panel_renderer
                .contents
                .poll_renderer
                .choices
                .into_iter()
                .map(|choice| choice.text.into())
                .collect(),
            poll_type: value
                .live_chat_action_panel_renderer
                .contents
                .poll_renderer
                .header
                .poll_header_renderer
                .live_chat_poll_type
                .into(),
        }
    }
}

impl From<PollToUpdateItem> for CoreAction {
    fn from(value: PollToUpdateItem) -> Self {
        CoreAction::FinishPoll {
            id: value.poll_renderer.live_chat_poll_id,
            choices: value
                .poll_renderer
                .choices
                .into_iter()
                .map(|choise| shared::types::PollResult {
                    choise: choise.text.into(),
                    ratio: choise.vote_ratio,
                })
                .collect(),
        }
    }
}

impl From<AuthorBadge> for UserBadges {
    fn from(value: AuthorBadge) -> Self {
        match value.live_chat_author_badge_renderer.icon {
            AuthorBadgeIcon::Icon { icon_type } => match icon_type {
                AuthorBadgeIconType::Verified => shared::types::UserBadges::Verified,
                AuthorBadgeIconType::Owner => shared::types::UserBadges::Owner,
                AuthorBadgeIconType::Moderator => shared::types::UserBadges::Moderator,
            },
            AuthorBadgeIcon::CustomThumbnail {} => shared::types::UserBadges::Member,
        }
    }
}

impl TryFrom<AuthorInfo> for shared::types::User {
    type Error = ConverterError;

    fn try_from(value: AuthorInfo) -> Result<Self, Self::Error> {
        let name = value.author_name.map(|text| text.into());
        let channel_id = value.author_external_channel_id;
        let badges = value
            .author_badges
            .map(|badges| {
                let badges = badges
                    .into_iter()
                    .map(|badge| badge.into())
                    .collect::<Vec<_>>();

                Vec1::try_from(badges).map_err(|_e| ConverterError::EmptyUserBadges)
            })
            .transpose()?;

        Ok(shared::types::User {
            name,
            channel_id,
            badges,
        })
    }
}

impl TryFrom<BannerItem> for CoreAction {
    type Error = ConverterError;

    fn try_from(value: BannerItem) -> Result<Self, Self::Error> {
        let action = match value.live_chat_banner_renderer.contents {
            BannerItemContent::LiveChatTextMessageRenderer {
                message,
                author_info,
                timestamp_usec,
                id,
            } => CoreAction::ChannelNotice {
                id: shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                },
                author: author_info.try_into()?,
                message: message.into(),
            },
            BannerItemContent::PollRenderer { poll_renderer } => CoreAction::StartPoll {
                id: poll_renderer.live_chat_poll_id,
                question: poll_renderer
                    .header
                    .poll_header_renderer
                    .poll_question
                    .into(),
                choices: poll_renderer
                    .choices
                    .into_iter()
                    .map(|choice| choice.text.into())
                    .collect(),
                poll_type: poll_renderer
                    .header
                    .poll_header_renderer
                    .live_chat_poll_type
                    .into(),
            },
            BannerItemContent::DonationsProgressBarRenderer {
                raised,
                campaign_title,
                goal_reached_label,
            } => CoreAction::FundraiserProgress {
                title: campaign_title,
                goal_label: goal_reached_label,
                raised: raised.into(),
            },
        };

        Ok(action)
    }
}

impl TryFrom<MessageItem> for Option<(shared::types::IdEntry, shared::types::MessageContent)> {
    type Error = ConverterError;

    fn try_from(value: MessageItem) -> Result<Self, Self::Error> {
        let result = match value {
            MessageItem::LiveChatTextMessageRenderer {
                id,
                timestamp_usec,
                message,
                author_info,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let content = shared::types::MessageContent::SimpleMessage {
                    author: author_info.try_into()?,
                    message: message.into(),
                };
                Some((id_entry, content))
            }
            MessageItem::LiveChatMembershipItemRenderer {
                id,
                timestamp_usec,
                author_info,
                header_primary_text,
                header_subtext,
                message,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let author = author_info.try_into()?;

                let membership_type = match (header_primary_text, header_subtext, message) {
                    (None, Some(subtext), None) => MembershipType::NewMember {
                        greeting: subtext.into(),
                    },
                    (Some(primary), subtext, message) => MembershipType::Member {
                        period: primary.into(),
                        membership_name: subtext.map(|text| text.into()),
                        message: message.map(|text| text.into()),
                    },
                    _ => return Err(ConverterError::MembershipType),
                };

                let content = shared::types::MessageContent::Membership {
                    author,
                    membership_type,
                };
                Some((id_entry, content))
            }
            MessageItem::LiveChatPaidMessageRenderer {
                id,
                timestamp_usec,
                message,
                author_info,
                purchase_amount_text,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let content = shared::types::MessageContent::Superchat {
                    author: author_info.try_into()?,
                    message: message.map(|m| m.into()),
                    amount: purchase_amount_text.into(),
                };
                Some((id_entry, content))
            }
            MessageItem::LiveChatPaidStickerRenderer {
                id,
                timestamp_usec,
                author_info,
                sticker,
                purchase_amount_text,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let content = shared::types::MessageContent::Sticker {
                    author: author_info.try_into()?,
                    sticker_name: sticker.accessibility.accessibility_data.label,
                    purchase_amount: purchase_amount_text.into(),
                };
                Some((id_entry, content))
            }
            MessageItem::LiveChatViewerEngagementMessageRenderer {
                id,
                timestamp_usec,
                message,
                icon,
            } => match icon.icon_type {
                EngagementMessageIconType::YoutubeRound => None,
                EngagementMessageIconType::Poll => {
                    let id_entry = shared::types::IdEntry {
                        id,
                        timepstamp: timestamp_usec.map(|wrapper| wrapper.0).unwrap_or_default(),
                    };

                    let content = shared::types::MessageContent::PollResult {
                        message: message.into(),
                    };
                    Some((id_entry, content))
                }
            },
            MessageItem::LiveChatPlaceholderItemRenderer {
                id: _,
                timestamp_usec: _,
            } => None,
            MessageItem::LiveChatModeChangeMessageRenderer {
                id,
                timestamp_usec,
                text,
                subtext,
                icon,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let content = shared::types::MessageContent::ChatMode {
                    text: text.into(),
                    subtext: subtext.into(),
                    mode: match icon.icon_type {
                        ChatModeIconType::TabSubscriptions => {
                            shared::types::ChatMode::SubscribersOnly
                        }
                        ChatModeIconType::SlowMode => shared::types::ChatMode::SlowMode,
                        ChatModeIconType::QuestionAnswer => shared::types::ChatMode::QuestionAnswer,
                        ChatModeIconType::Memberships => shared::types::ChatMode::MembersOnly,
                    },
                };
                Some((id_entry, content))
            }
            MessageItem::LiveChatDonationAnnouncementRenderer {
                id,
                timestamp_usec,
                text,
                subtext,
                author_info,
            } => {
                let id_entry = shared::types::IdEntry {
                    id,
                    timepstamp: timestamp_usec,
                };
                let author = author_info.map(|info| info.try_into()).transpose()?;
                let content = shared::types::MessageContent::Fundraiser {
                    author,
                    text: text.into(),
                    subtext: subtext.into(),
                };
                Some((id_entry, content))
            }
        };

        Ok(result)
    }
}

impl TryFrom<YouTubeAction> for Option<CoreAction> {
    type Error = ConverterError;

    fn try_from(value: YouTubeAction) -> Result<Self, Self::Error> {
        let action = match value {
            Action::AddChatItemAction { item } => {
                let new_item: Option<(shared::types::IdEntry, shared::types::MessageContent)> =
                    item.try_into()?;

                new_item.map(|(id, content)| CoreAction::NewMessage {
                    id,
                    message: content,
                })
            }
            Action::MarkChatItemAsDeletedAction { target_item_id } => {
                Some(CoreAction::DeleteMessage {
                    target_id: target_item_id,
                })
            }
            Action::MarkChatItemsByAuthorAsDeletedAction {
                external_channel_id,
            } => Some(CoreAction::BlockUser {
                channel_id: external_channel_id,
            }),
            Action::ReplaceChatItemAction {
                target_item_id,
                replacement_item,
            } => {
                let replacement: Option<(shared::types::IdEntry, shared::types::MessageContent)> =
                    replacement_item.try_into()?;

                replacement.map(|(id, content)| CoreAction::ReplaceMessage {
                    target_id: target_item_id,
                    new_id: id,
                    message: content,
                })
            }
            Action::AddBannerToLiveChatCommand { banner_renderer } => {
                Some(banner_renderer.try_into()?)
            }
            Action::RemoveBannerForLiveChatCommand { target_action_id } => {
                Some(CoreAction::CloseBanner {
                    banner_id: target_action_id,
                })
            }
            Action::ShowLiveChatActionPanelAction { panel_to_show } => Some(panel_to_show.into()),
            Action::UpdateLiveChatPollAction { poll_to_update } => Some(poll_to_update.into()),
            Action::CloseLiveChatActionPanelAction { target_panel_id } => {
                Some(CoreAction::ClosePanel {
                    target_id: target_panel_id,
                })
            }
            Action::ShowLiveChatTooltipCommand {} => None,
            Action::AddLiveChatTickerItemAction {} => None,
        };

        Ok(action)
    }
}
