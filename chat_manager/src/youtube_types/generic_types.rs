use serde::{Deserialize, de::Visitor};
use vec1::Vec1;

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Emoji {
    pub emoji_id: String,
    pub shortcuts: Option<Vec1<String>>,
    #[serde(default)]
    pub is_custom_emoji: bool
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct AuthorBadge {
    pub live_chat_author_badge_renderer: AuthorBadgeRenderer
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct AuthorBadgeRenderer {
    #[serde(flatten)]
    pub icon: AuthorBadgeIcon
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum AuthorBadgeIcon {
    #[serde(rename_all(deserialize = "camelCase"))]
    Icon {
        icon_type: AuthorBadgeIconType
    },
    CustomThumbnail { }
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub enum AuthorBadgeIconType {
    Verified,
    Owner,
    Moderator
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct AuthorInfo {
    // It's somehow possible to have a user without name.
    // Example: UCAX4_YlLpeVKF8L3fIUSDRA
    pub author_name: Option<Message>,
    pub author_external_channel_id: String,
    pub author_badges: Option<Vec<AuthorBadge>>,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Image {
    pub thumbnails: Vec1<Thumbnail>,
    pub accessibility: Accessibility
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Thumbnail {
    pub url: String,
    pub width: u16,
    pub height: u16
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Accessibility {
    pub accessibility_data: AccessibilityData
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct AccessibilityData {
    pub label: String
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct LinkUrl {
    #[serde(flatten)]
    pub endpoint: LinkEndpoint
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum LinkEndpoint {
    UrlEndpoint {
        url: String
    },
    #[serde(rename_all(deserialize = "camelCase"))]
    WatchEndpoint {
        video_id: String
    }
}

pub enum MessageContent {
    Link {
        text: String,
        url: LinkUrl
    },
    Text {
        text: String,
        bold: bool,
        italics: bool
    },
    Emoji(Emoji)
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum Message {
    SimpleText(String),
    Runs(Vec<MessageContent>)
}

mod custom_deser_impls {
    use serde::Deserialize;
    use super::*;

    impl <'de> Deserialize<'de> for MessageContent {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> 
        {
            #[derive(Deserialize)]
            #[serde(field_identifier, rename_all = "camelCase")]
            enum Field { Text, Emoji, NavigationEndpoint, Italics, Bold }

            struct ContentVisitor;
    
            impl<'de> Visitor<'de> for ContentVisitor {
                type Value = MessageContent;
    
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("enum MessageContent")
                }
    
                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>, 
                {
                    let mut text: Option<String> = None;
                    let mut url: Option<LinkUrl> = None;
                    let mut emoji: Option<Emoji> = None;
                    let mut bold: Option<bool> = None;
                    let mut italics: Option<bool> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Text => {
                                if emoji.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a text and an emoji"
                                    ));
                                }
                                if text.is_some() {
                                    return Err(serde::de::Error::duplicate_field("text"));
                                }
                                text = Some(map.next_value()?);
                            },
                            Field::Emoji => {
                                if text.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a text and an emoji"
                                    ));
                                }
                                if url.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and an emoji"
                                    ));
                                }
                                if italics.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both an emoji and an italics modifier"
                                    ));
                                }
                                if bold.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both an emoji and a bold modifier"
                                    ));
                                }
                                if emoji.is_some() {
                                    return Err(serde::de::Error::duplicate_field("emoji"));
                                }
                                emoji = Some(map.next_value()?);
                            },
                            Field::NavigationEndpoint => {
                                if emoji.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and an emoji"
                                    ));
                                }
                                if italics.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and an italics modifier"
                                    ));
                                }
                                if bold.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and a bold modifier"
                                    ));
                                }
                                if url.is_some() {
                                    return Err(serde::de::Error::duplicate_field("navigationEndpoint"));
                                }
                                url = Some(map.next_value()?);
                            },
                            Field::Italics => {
                                if emoji.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both an emoji and an italics modifier"
                                    ));
                                }
                                if url.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and an italics modifier"
                                    ));
                                }
                                if italics.is_some() {
                                    return Err(serde::de::Error::duplicate_field("italics"));
                                }
                                italics = Some(map.next_value()?);
                            }
                            Field::Bold => {
                                if emoji.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both an emoji and a bold modifier"
                                    ));
                                }
                                if url.is_some() {
                                    return Err(serde::de::Error::invalid_value(
                                        serde::de::Unexpected::StructVariant, 
                                        &"can't have both a url and a bold modifier"
                                    ));
                                }
                                if bold.is_some() {
                                    return Err(serde::de::Error::duplicate_field("bold"));
                                }
                                bold = Some(map.next_value()?);
                            }
                        }
                    }
    
                    if let Some(emoji) = emoji {
                        return Ok(MessageContent::Emoji(emoji));
                    }
    
                    let text = text
                        .ok_or_else(|| serde::de::Error::missing_field("text"))?;
                    if let Some(link) = url {
                        return Ok(MessageContent::Link {
                            text,
                            url: link
                        });
                    } else {
                        return Ok(MessageContent::Text { 
                            text,
                            bold: bold.unwrap_or_default(),
                            italics: italics.unwrap_or_default()
                        })
                    }
                }
            }
    
            const FIELDS: &'static [&'static str] = &["text", "emoji", "url"];
            deserializer.deserialize_struct("MessageContent", FIELDS, ContentVisitor)
        }
    }
}