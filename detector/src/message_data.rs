pub struct MessageData {
    pub content: String,
    count: u16
}

impl MessageData {
    pub fn new(content: String) -> Self {
        MessageData {
            content,
            count: 1
        }
    }

    pub fn reconstruct_message(&mut self, s2: &str) {
        let mut buf = String::with_capacity((self.content.len() + s2.len()) / 2 + 1);
        let mut i = self.content.chars();
        let mut j = s2.chars();
    
        let mut c1 = i.next();
        let mut c2 = j.next();
        loop {
            match (c1, c2) {
                (Some(char11), Some(char21)) => {
                    if char11 == char21 {
                        buf.push(char11);
                        c1 = i.next();
                        c2 = j.next();
                    } else {
                        c1 = i.next();
                        c2 = j.next();
                        match (c1, c2) {
                            (Some(char12), Some(char22)) => {
                                if char12 == char21 {
                                    buf.push(char11);
                                    c1 = i.next();
                                    buf.push(char12);
                                } else if char11 == char22 {
                                    buf.push(char21);
                                    buf.push(char11);
                                    c2 = j.next();
                                } else {
                                    buf.push(char11);
                                    buf.push(char21);
                                }
                            },
                            (Some(char12), None) => {
                                if char12 == char21 {
                                    buf.push(char11);
                                    buf.push(char12);
                                } else {
                                    buf.push(char11);
                                    buf.push(char21);
                                    buf.push(char12);
                                }
                                c1 = i.next();
                            },
                            (None, Some(char22)) => {
                                if char11 == char22 {
                                    buf.push(char21);
                                    buf.push(char11);
                                } else {
                                    buf.push(char11);
                                    buf.push(char21);
                                    buf.push(char22);
                                }
                                c2 = j.next();
                            },
                            (None, None) => {
                                buf.push(char11);
                                buf.push(char21);
                                break;
                            }
                        }
                    }
                },
                (Some(char1), None) => {
                    buf.push(char1);
                    c1 = i.next();
                },
                (None, Some(char2)) => {
                    buf.push(char2);
                    c2 = j.next();
                },
                (None, None) => break
            }
        }

        self.count += 1;
        self.content = buf;
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[allow(unused_imports)]
mod test {
    use super::*;

    #[test]
    pub fn test_string_recreation() {
        struct TestCase {
            input: Vec<String>,
            expected: String
        }

        let test_cases = vec![
            // Synthetic data
            TestCase {
                input: vec![
                    "стрница".to_string(), 
                    "странца".to_string(), 
                    "сраница".to_string()
                ],
                expected: "страница".to_string()
            },

            // Synthetic data that includes full message
            TestCase {
                input: vec![
                    "стрница".to_string(), 
                    "странца".to_string(), 
                    "сраница".to_string(), 
                    "страница".to_string()
                ],
                expected: "страница".to_string()
            },

            // Data from chat
            TestCase {
                input: vec![
                    "カチオチの方光るよ".to_string(), 
                    "カチオチ方は光るよ".to_string(), 
                    "カチオの方は光るよ".to_string()
                ],
                expected: "カチオチの方は光るよ".to_string()
            },
            TestCase {
                input: vec![
                    "スノムの方が良さげ".to_string(),
                    "スノイムのが良さげ".to_string(),
                    "スノイムの方良さげ".to_string(),
                    "スノイムの方が良げ".to_string(),
                    "スノイム方が良さげ".to_string()
                ],
                expected: "スノイムの方が良さげ".to_string()
            },
            TestCase {
                input: vec![
                    "シーズは光れたのか".to_string(),
                    "シラーズは光たのか".to_string(),
                ],
                expected: "シラーズは光れたのか".to_string()
            },
            TestCase {
                input: vec![
                    "カーンのカリーカッグ".to_string(),
                    "カビンのカリーカッグ".to_string()
                ],
                expected: "カービンのカリーカッグ".to_string()
            },
            TestCase {
                input: vec![
                    "シラークタファー".to_string(),
                    "シラータファーか".to_string(),
                    "シラークタファか".to_string()
                ],
                expected: "シラークタファーか".to_string()
            },
            TestCase {
                input: vec![
                    "ュルク欲しされるぺこ".to_string(),
                    "シュルク欲しされるぺ".to_string()
                ],
                expected: "シュルク欲しされるぺこ".to_string()
            },
            TestCase {
                input: vec![
                    "スタヤックス".to_string(),
                    "スタイヤック".to_string()
                ],
                expected: "スタイヤックス".to_string()
            },
            TestCase {
                input: vec![
                    "グっぽくしよくねぇ".to_string(),
                    "グルっぽくしよくね".to_string()
                ],
                expected: "グルっぽくしよくねぇ".to_string()
            },
            TestCase {
                input: vec![
                    "チュウリンイス".to_string(),
                    "チュウリガイス".to_string()
                ],
                expected: "チュウリンガイス".to_string()
            },
            TestCase {
                input: vec![
                    "ノイムの方が良さげ".to_string(),
                    "スノイムの方良さげ".to_string()
                ],
                expected: "スノイムの方が良さげ".to_string()
            }
        ];

        for test_case in test_cases {
            let mut message_data = MessageData::new("".to_string());
            for input in test_case.input {
                message_data.reconstruct_message(&input);
            }

            assert_eq!(test_case.expected, message_data.content);
        }
    }
}