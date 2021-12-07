pub struct DetectorParams {
    deleted_messages_threshold: usize,
    avg_delay_threshold: f32,
    avg_delay_min_message_count: usize,
    avg_length_threshold: f32,
    avg_length_min_message_count: usize,
    similarity_threshold: f32,
    similarity_count_threshold: usize,
    similarity_min_message_length: usize,
}

impl DetectorParams {
    pub fn new(
        deleted_messages_threshold: usize,
        avg_delay_threshold: f32,
        avg_delay_min_message_count: usize,
        avg_length_threshold: f32,
        avg_length_min_message_count: usize,
        similarity_threshold: f32,
        similarity_count_threshold: usize,
        similarity_min_message_length: usize,
    ) -> Self {
        Self {
            deleted_messages_threshold,
            avg_delay_threshold,
            avg_delay_min_message_count,
            avg_length_threshold,
            avg_length_min_message_count,
            similarity_threshold,
            similarity_count_threshold,
            similarity_min_message_length,
        }
    }

    pub fn is_too_many_deleted_messages(&self, delete_messages_count: usize) -> bool {
        delete_messages_count >= self.deleted_messages_threshold
    }

    pub fn is_too_fast(&self, current_avg_delay: f32, sent_messages_count: usize) -> bool {
        sent_messages_count >= self.avg_delay_min_message_count
            && current_avg_delay < self.avg_delay_threshold
    }

    pub fn are_messages_too_long(
        &self,
        current_avg_length: f32,
        sent_messages_count: usize,
    ) -> bool {
        sent_messages_count >= self.avg_length_min_message_count
            && current_avg_length >= self.avg_length_threshold
    }

    pub fn should_check_similarity(&self, message_length: usize) -> bool {
        message_length >= self.similarity_min_message_length
    }

    pub fn are_messages_similar(&self, similarity: f32) -> bool {
        similarity > self.similarity_threshold
    }

    pub fn too_many_similar_messages(&self, similar_messages_count: usize) -> bool {
        similar_messages_count >= self.similarity_count_threshold
    }
}

impl Default for DetectorParams {
    fn default() -> Self {
        Self {
            deleted_messages_threshold: 4,
            avg_delay_threshold: 5000.0,
            avg_delay_min_message_count: 5,
            avg_length_threshold: 30.0,
            avg_length_min_message_count: 5,
            similarity_threshold: 0.85,
            similarity_count_threshold: 3,
            similarity_min_message_length: 10,
        }
    }
}