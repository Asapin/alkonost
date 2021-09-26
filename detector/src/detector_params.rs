pub struct DetectorParams {
    deleted_messages_threshold: u64,
    avg_delay_threshold: f32,
    avg_delay_min_message_count: u64,
    avg_length_threshold: f32,
    avg_length_min_message_count: u64,
    similarity_threshold: f64,
    similarity_count_threshold: u16,
    similarity_min_message_length: usize
}

impl DetectorParams {
    pub fn new(
        deleted_messages_threshold: u64,
        avg_delay_threshold: f32,
        avg_delay_min_message_count: u64,
        avg_length_threshold: f32,
        avg_length_min_message_count: u64,
        similarity_threshold: f64,
        similarity_count_threshold: u16,
        similarity_min_message_length: usize
    ) -> Self {
        Self {
            deleted_messages_threshold,
            avg_delay_threshold,
            avg_delay_min_message_count,
            avg_length_threshold,
            avg_length_min_message_count,
            similarity_threshold,
            similarity_count_threshold,
            similarity_min_message_length
        }
    }

    pub fn is_too_many_deleted_messages(&self, delete_messages_count: u64) -> bool {
        delete_messages_count >= self.deleted_messages_threshold
    }

    pub fn is_too_fast(&self, current_delay: f32, sent_messages_count: u64) -> bool {
        sent_messages_count >= self.avg_delay_min_message_count &&
        current_delay < self.avg_delay_threshold
    }

    pub fn are_messages_too_long(&self, current_avg_length: f32, sent_messages_count: u64) -> bool {
        sent_messages_count >= self.avg_length_min_message_count &&
        current_avg_length >= self.avg_length_threshold
    }

    pub fn should_check_message(&self, message_length: usize) -> bool {
        message_length >= self.similarity_min_message_length
    }

    pub fn are_messages_similar(&self, message1: &str, message2: &str) -> bool {
        strsim::jaro(message1, message2) > self.similarity_threshold
    }

    pub fn too_many_similar_messages(&self, similar_messages_count: u16) -> bool {
        similar_messages_count >= self.similarity_count_threshold
    }
}