use core::SuspicionReason;

use crate::{detector_params::DetectorParams, message_data::MessageData};

pub struct UserData {
    history: Vec<MessageData>,
    delete_messages_count: u64,
    last_message_timestamp: u64,
    total_messages: u64,
    avg_delay: f32,
    avg_message_length: f32,
}

impl UserData {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            delete_messages_count: 0,
            last_message_timestamp: 0,
            total_messages: 0,
            avg_delay: 0.0,
            avg_message_length: 0.0,
        }
    }

    /// Returns `true` if the user deleted more than `detector_params.deleted_messages_threshold` messages
    pub fn deleted_message(&mut self, detector_params: &DetectorParams) -> bool {
        self.delete_messages_count += 1;
        detector_params.is_too_many_deleted_messages(self.delete_messages_count)
    }

    /// Add a new message from the user and check the user is suspicious.
    /// Returns `true` if the user is suspicious under current detector params
    pub fn new_message(
        &mut self,
        message: &str,
        timestamp: u64,
        detector_params: &DetectorParams,
    ) -> Option<SuspicionReason> {
        self.total_messages += 1;

        let message_length = message.chars().count();
        self.avg_message_length = (message_length as f32
            + (self.total_messages - 1) as f32 * self.avg_message_length)
            / self.total_messages as f32;

        if self.last_message_timestamp == 0 {
            self.last_message_timestamp = timestamp;
            return None;
        }

        if detector_params.are_messages_too_long(self.avg_message_length, self.total_messages) {
            return Some(SuspicionReason::TooLong(self.avg_message_length));
        }

        let time_diff = timestamp - self.last_message_timestamp;
        self.last_message_timestamp = timestamp;
        self.avg_delay = (time_diff + (self.total_messages - 1) * self.avg_delay as u64) as f32
            / self.total_messages as f32;

        if detector_params.is_too_fast(self.avg_delay, self.total_messages) {
            return Some(SuspicionReason::TooFast(self.avg_delay));
        }

        if !detector_params.should_check_message(message_length) {
            return None;
        }

        let mut found_similar_message = false;
        for history in self.history.iter_mut() {
            if detector_params.are_messages_similar(history.content(), message) {
                found_similar_message = true;
                history.reconstruct_message(message);
                if detector_params.too_many_similar_messages(history.count()) {
                    return Some(SuspicionReason::Similar);
                }

                break;
            }
        }

        if !found_similar_message {
            let new_message_data = MessageData::new(message.to_string());
            self.history.push(new_message_data);
        }

        None
    }
}
