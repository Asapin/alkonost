pub mod actions;
pub mod generic_types;
pub mod root;

#[cfg(test)]
mod tests {
    use crate::youtube_types::root::ChatJson;

    #[test]
    fn deserialize_chat_json() {
        let json = include_str!("../../unimplemented_types.json");
        let _chat_json = serde_json::from_str::<ChatJson>(json).unwrap();
    }

    #[test]
    fn deserialize_all_jsons_in_a_dir() {
        let paths = std::fs::read_dir("./test_jsons/").unwrap();

        paths
            .into_iter()
            .filter_map(|dir| dir.ok())
            .map(|entry| entry.path())
            .for_each(|path| {
                let content = std::fs::read_to_string(&path).unwrap();
                let result = serde_json::from_str::<ChatJson>(&content);
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        panic!("{:?}: {}", &path, &e);
                    }
                }
            });
    }
}
