use std::collections::{HashMap, HashSet};

use shared::{
    detector_params::DetectorParams, messages::DetectorDecision, types::Action, SuspicionReason,
};

use crate::user_data::UserData;

pub struct ProcessingResult {
    pub decisions: Vec<DetectorDecision>,
    pub processed_messages: usize
}

pub struct SpamDetector {
    history: HashMap<String, UserData>,
    suspicious: HashMap<String, SuspicionReason>,
    supporters: HashSet<String>,
    message_to_user: HashMap<String, String>,
}

impl SpamDetector {
    pub fn init() -> Self {
        Self {
            history: HashMap::new(),
            suspicious: HashMap::new(),
            supporters: HashSet::new(),
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
            processed_messages: 0
        };

        for action in actions {
            match action {
                Action::NewMessage { id, message }
                | Action::ReplaceMessage {
                    new_id: id,
                    message,
                    ..
                } => {
                    result.processed_messages += 1;

                    match message {
                        shared::types::MessageContent::SimpleMessage { author, message } => {
                            if self.should_skip_analyzing(&author.channel_id) {
                                continue;
                            }

                            // Don't analyze messages from mods/verified users/members/owner
                            if author.badges.is_some() {
                                continue;
                            }

                            self.message_to_user
                                .insert(id.id, author.channel_id.clone());

                            let user_data = self.get_user_data(author.channel_id.clone());
                            if let Some(reason) =
                                user_data.new_message(&message, id.timepstamp, detector_params)
                            {
                                self.mark_as_suspicious(author.channel_id, reason, &mut result.decisions);
                            }
                        }
                        shared::types::MessageContent::Membership { author, .. }
                        | shared::types::MessageContent::Superchat { author, .. }
                        | shared::types::MessageContent::Sticker { author, .. } => {
                            self.mark_as_supporter(author.channel_id, &mut result.decisions);
                        }
                        shared::types::MessageContent::Fundraiser { author, .. } => {
                            author.into_iter().for_each(|author| {
                                self.mark_as_supporter(author.channel_id, &mut result.decisions)
                            });
                        }
                        shared::types::MessageContent::ChatMode { .. } => {
                            // Only streamer can generate this type of messages
                        }
                        shared::types::MessageContent::PollResult { .. } => {
                            // Poll results are generated after the poll has ended
                        }
                    }
                }
                Action::DeleteMessage { target_id } => {
                    result.processed_messages += 1;

                    let message_author = self.message_to_user.get(&target_id);
                    let author = match message_author {
                        Some(author) => author.clone(),
                        None => {
                            // It was probably a message from either a supporter or a suspicious user,
                            // that's why it wasn't registered
                            continue;
                        }
                    };

                    if self.should_skip_analyzing(&author) {
                        continue;
                    }

                    let user_data = self.get_user_data(author.clone());
                    if user_data.deleted_message(detector_params) {
                        self.mark_as_suspicious(
                            author,
                            SuspicionReason::TooManyDeletedMessages,
                            &mut result.decisions,
                        );
                    }
                }
                Action::BlockUser { channel_id } => {
                    result.processed_messages += 1;

                    // There's a possibility of a human error,
                    // but most of the times users get blocked either for spamming
                    // or inapropriate behaviour, which this app tries to detect.
                    // However, chances that the harasser will donate money
                    // or join channel's membership are also very slim.
                    // Which means, that if a member or person who supported channel monetarily
                    // gets blocked, it's most likely a human error
                    if !self.supporters.contains(&channel_id) {
                        self.mark_as_suspicious(
                            channel_id,
                            SuspicionReason::Blocked,
                            &mut result.decisions,
                        );
                    }
                }
                Action::CloseBanner { .. }
                | Action::StartPoll { .. }
                | Action::FinishPoll { .. }
                | Action::ChannelNotice { .. }
                | Action::FundraiserProgress { .. }
                | Action::ClosePanel { .. } => {
                    // These are not user generated actions, so we ignore them
                }
            }
        }

        result
    }

    /// No need to check if the user is already marked as a potential spammer
    /// or if they are a supporter of the channel
    fn should_skip_analyzing(&self, channel_id: &str) -> bool {
        self.supporters.contains(channel_id) || self.suspicious.contains_key(channel_id)
    }

    fn get_user_data(&mut self, channel_id: String) -> &mut UserData {
        self.history.entry(channel_id).or_insert_with(UserData::new)
    }

    fn mark_as_supporter(&mut self, channel_id: String, decisions: &mut Vec<DetectorDecision>) {
        self.history.remove(&channel_id);
        if let Some(_prev) = self.suspicious.remove(&channel_id) {
            decisions.push(DetectorDecision::remove_user(channel_id.clone()));
        }
        self.supporters.insert(channel_id);
    }

    fn mark_as_suspicious(
        &mut self,
        channel_id: String,
        reason: SuspicionReason,
        decisions: &mut Vec<DetectorDecision>,
    ) {
        self.history.remove(&channel_id);
        self.suspicious.insert(channel_id.clone(), reason.clone());
        decisions.push(DetectorDecision::add_user(channel_id, reason));
    }
}
