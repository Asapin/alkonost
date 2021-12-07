use std::collections::HashMap;

use shared::{
    detector_params::DetectorParams, messages::detector::DetectorDecision, types::Action,
};

use crate::user_data::{UserData, UserMessage};

pub struct ProcessingResult {
    pub decisions: Vec<DetectorDecision>,
    pub processed_messages: usize,
}

pub struct SpamDetector {
    history: HashMap<String, UserData>,
    message_to_user: HashMap<String, String>,
}

impl SpamDetector {
    pub fn init() -> Self {
        Self {
            history: HashMap::new(),
            message_to_user: HashMap::new(),
        }
    }

    pub fn process_new_messages(
        &mut self,
        actions: Vec<Action>,
        detector_params: &DetectorParams,
    ) -> ProcessingResult {
        let mut result = ProcessingResult {
            decisions: Vec::new(),
            processed_messages: 0,
        };

        let user_messages = actions
            .into_iter()
            .filter_map(|action| match action {
                Action::NewMessage { id, message }
                | Action::ReplaceMessage {
                    new_id: id,
                    message,
                    ..
                } => match message {
                    shared::types::MessageContent::SimpleMessage { author, message } => {
                        self.message_to_user
                            .insert(id.id, author.channel_id.clone());

                        let message = UserMessage::Regular {
                            message,
                            timestamp: id.timepstamp,
                            author_has_badges: author.badges.is_some(),
                        };
                        Some((author.channel_id, message))
                    }
                    shared::types::MessageContent::Membership { author, .. }
                    | shared::types::MessageContent::Superchat { author, .. }
                    | shared::types::MessageContent::Sticker { author, .. } => {
                        Some((author.channel_id, UserMessage::Support))
                    }
                    shared::types::MessageContent::Fundraiser { author, .. } => match author {
                        Some(user) => Some((user.channel_id, UserMessage::Support)),
                        None => None,
                    },
                    shared::types::MessageContent::ChatMode { .. }
                    | shared::types::MessageContent::PollResult { .. } => None,
                },
                Action::DeleteMessage { target_id } => match self.message_to_user.get(&target_id) {
                    Some(author) => Some((author.clone(), UserMessage::Delete)),
                    None => {
                        shared::tracing_warn!(
                            "Couldn't find author of the deleted message {}",
                            &target_id
                        );
                        None
                    }
                },
                Action::BlockUser { channel_id } => Some((channel_id, UserMessage::Blocked)),
                Action::CloseBanner { .. }
                | Action::StartPoll { .. }
                | Action::FinishPoll { .. }
                | Action::ChannelNotice { .. }
                | Action::FundraiserProgress { .. }
                | Action::ClosePanel { .. } => None,
            })
            .collect::<Vec<_>>();

        for (channel_id, message) in user_messages {
            result.processed_messages += 1;

            let user_data = self.get_user_data(channel_id.clone());
            if let Some(decision) = user_data.analyze_new_message(message, detector_params) {
                let detector_decision = DetectorDecision::new(channel_id, decision);
                result.decisions.push(detector_decision);
            }
        }

        result
    }

    fn get_user_data(&mut self, channel_id: String) -> &mut UserData {
        self.history.entry(channel_id).or_insert_with(UserData::new)
    }
}
