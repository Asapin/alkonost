use super::actions::Action;

pub struct ChatJson {
    pub continuation: Option<Continuation>,
    pub actions: Option<Vec<Action>>,
}

pub struct Continuation {
    pub timeout_ms: u16,
    pub continuation: String,
}

mod custom_deser_impls {
    use serde::Deserialize;
    use super::*;

    impl<'de> Deserialize<'de> for ChatJson {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> 
        {
            #[derive(Deserialize)]
            #[serde(rename_all(deserialize = "camelCase"))]
            struct Outer {
                continuation_contents: Option<Middle>,
            }
    
            #[derive(Deserialize)]
            #[serde(rename_all(deserialize = "camelCase"))]
            struct Middle {
                live_chat_continuation: Inner,
            }
    
            #[derive(Deserialize)]
            struct Inner {
                continuations: Vec<InnerContinuation>,
                actions: Option<Vec<ActionWrapper>>,
            }
    
            #[derive(Deserialize)]
            #[serde(rename_all(deserialize = "camelCase"))]
            enum InnerContinuation {
                #[serde(rename_all(deserialize = "camelCase"))]
                TimedContinuationData {
                    timeout_ms: u16,
                    continuation: String,
                },
                #[serde(rename_all(deserialize = "camelCase"))]
                InvalidationContinuationData {
                    timeout_ms: u16,
                    continuation: String,
                },
                #[serde(rename_all(deserialize = "camelCase"))]
                ReloadContinuationData {
                    continuation: String,
                }
            }
    
            #[derive(Deserialize)]
            #[serde(rename_all(deserialize = "camelCase"))]
            struct ActionWrapper {
                #[serde(flatten)]
                action: Action    
            }
    
            let outer = Outer::deserialize(deserializer)?;
            let chat_json = match outer.continuation_contents {
                Some(middle) => {
                    let inner_continuation = middle
                        .live_chat_continuation
                        .continuations
                        .into_iter()
                        .next()
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &"at least 1 continuation should exist"))?;
    
                    let continuation = match inner_continuation {
                        InnerContinuation::TimedContinuationData { 
                            timeout_ms, 
                            continuation 
                        } => Continuation { timeout_ms, continuation},
                        InnerContinuation::InvalidationContinuationData { 
                            timeout_ms, 
                            continuation 
                        } => Continuation { timeout_ms, continuation},
                        InnerContinuation::ReloadContinuationData { 
                            continuation 
                        } => Continuation { timeout_ms: 0, continuation},
                    };
    
                    let actions = middle
                        .live_chat_continuation
                        .actions
                        .map(|action_wrappers| {
                            action_wrappers
                                .into_iter()
                                .map(|action_wrapper| action_wrapper.action)
                                .collect()
                        });
    
                    ChatJson {
                        continuation: Some(continuation),
                        actions
                    }
                },
                None => {
                    ChatJson {
                        continuation: None,
                        actions: None
                    }
                },
            };
    
            Ok(chat_json)
        }
    }
}