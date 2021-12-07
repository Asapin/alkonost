use std::mem;

use shared::{detector_params::DetectorParams, messages::detector::Decision};

pub enum UserMessage {
    Regular {
        message: String,
        timestamp: u64,
        author_has_badges: bool,
    },
    Support,
    Delete,
    Blocked,
}

enum UserStatus {
    Immune, // Members, moderators, verified users and users who sent superchat or sticker
    Blocked {
        // User was blocked by moderators or by the streamer
        // But there's still a chance, that the ban was a mistake, and they can be unblocked
        history: Vec<(u64, String)>,
        delete_messages_count: usize,
    },
    Suspicious {
        // Collects data just as a regular user, but doesn't analyze it
        history: Vec<(u64, String)>,
        delete_messages_count: usize,
    },
    Regular {
        // Regular user. Collect and analyze their messages
        history: Vec<(u64, String)>,
        delete_messages_count: usize,
    },
}

pub struct UserData {
    status: UserStatus,
}

impl UserData {
    pub fn new() -> Self {
        Self {
            status: UserStatus::Regular {
                history: Vec::new(),
                delete_messages_count: 0,
            },
        }
    }

    pub fn analyze_new_message(
        &mut self,
        message: UserMessage,
        detector_params: &DetectorParams,
    ) -> Option<Decision> {
        let old_status = mem::replace(&mut self.status, UserStatus::Immune);
        let (new_status, decision) = UserData::do_analysis(old_status, message, detector_params);
        self.status = new_status;

        decision
    }

    fn do_analysis(
        status: UserStatus,
        message: UserMessage,
        detector_params: &DetectorParams,
    ) -> (UserStatus, Option<Decision>) {
        match status {
            UserStatus::Immune => (UserStatus::Immune, None),
            UserStatus::Blocked {
                history,
                delete_messages_count,
            } => {
                let temp_status = UserStatus::Regular {
                    history,
                    delete_messages_count,
                };

                let (new_status, decision) =
                    UserData::do_analysis(temp_status, message, detector_params);
                (new_status, decision.or(Some(Decision::Clear)))
            }
            UserStatus::Suspicious {
                mut history,
                mut delete_messages_count,
            } => match message {
                UserMessage::Support => (UserStatus::Immune, Some(Decision::Clear)),
                UserMessage::Blocked => (
                    UserStatus::Blocked {
                        history,
                        delete_messages_count,
                    },
                    Some(Decision::Blocked),
                ),
                UserMessage::Delete => {
                    delete_messages_count += 1;
                    (
                        UserStatus::Suspicious {
                            history,
                            delete_messages_count,
                        },
                        None,
                    )
                }
                UserMessage::Regular {
                    message,
                    timestamp,
                    author_has_badges,
                } => match author_has_badges {
                    true => (UserStatus::Immune, Some(Decision::Clear)),
                    false => {
                        history.push((timestamp, message));
                        (
                            UserStatus::Suspicious {
                                history,
                                delete_messages_count,
                            },
                            None,
                        )
                    }
                },
            },
            UserStatus::Regular {
                mut history,
                mut delete_messages_count,
            } => match message {
                UserMessage::Support => (UserStatus::Immune, None),
                UserMessage::Blocked => (
                    UserStatus::Blocked {
                        history,
                        delete_messages_count,
                    },
                    Some(Decision::Blocked),
                ),
                UserMessage::Delete => {
                    delete_messages_count += 1;
                    if detector_params.is_too_many_deleted_messages(delete_messages_count) {
                        return (
                            UserStatus::Suspicious {
                                history,
                                delete_messages_count,
                            },
                            Some(Decision::TooManyDeleted),
                        );
                    }

                    (
                        UserStatus::Regular {
                            history,
                            delete_messages_count,
                        },
                        None,
                    )
                }
                UserMessage::Regular {
                    message,
                    timestamp,
                    author_has_badges,
                } => {
                    if author_has_badges {
                        return (UserStatus::Immune, None);
                    }

                    history.push((timestamp, message));

                    struct Acc {
                        last_timestamp: u64,
                        sum_of_delays: u64,
                        sum_of_lengths: usize,
                    }

                    let init = Acc {
                        last_timestamp: 0,
                        sum_of_delays: 0,
                        sum_of_lengths: 0,
                    };

                    let result = history.iter().fold(init, |mut acc, (timestamp, message)| {
                        if acc.last_timestamp != 0 {
                            acc.sum_of_delays += timestamp - acc.last_timestamp
                        }
                        acc.last_timestamp = *timestamp;
                        acc.sum_of_lengths += message.len();
                        acc
                    });

                    let current_avg_length = result.sum_of_lengths as f32 / history.len() as f32;
                    let current_avg_delay = result.sum_of_delays as f32 / history.len() as f32;

                    if detector_params.are_messages_too_long(current_avg_length, history.len()) {
                        let new_status = UserStatus::Suspicious {
                            history,
                            delete_messages_count,
                        };
                        let decision = Decision::TooLong(current_avg_length);
                        return (new_status, Some(decision));
                    }

                    if detector_params.is_too_fast(current_avg_delay, history.len()) {
                        let new_status = UserStatus::Suspicious {
                            history,
                            delete_messages_count,
                        };
                        let decision = Decision::TooFast(current_avg_length);
                        return (new_status, Some(decision));
                    }

                    if detector_params.should_check_similarity(history.len()) {
                        let mut similar_count = 0;
                        for (index, (_, message_1)) in history.iter().enumerate() {
                            for (_, message_2) in history.iter().skip(index + 1) {
                                let similarity = strsim::jaro(message_1, message_2) as f32;
                                if detector_params.are_messages_similar(similarity) {
                                    similar_count += 1;
                                    break;
                                }
                            }

                            if detector_params.too_many_similar_messages(similar_count) {
                                let new_status = UserStatus::Suspicious {
                                    history,
                                    delete_messages_count,
                                };
                                let decision = Decision::Similar;
                                return (new_status, Some(decision));
                            }
                        }
                    }

                    (
                        UserStatus::Regular {
                            history,
                            delete_messages_count,
                        },
                        None,
                    )
                }
            },
        }
    }
}
