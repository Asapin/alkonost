use std::convert::TryFrom;

use crate::youtube_types::actions::Action;
use thiserror::Error;

type CoreAction = shared::types::Action;

mod extractors;

#[derive(Error, Debug)]
pub enum ConverterError {
    #[error("The field for user badges is present, but has 0 elements")]
    EmptyUserBadges,
    #[error("Couldn't determine the membership type")]
    MembershipType,
}

pub struct Converter;

impl Converter {
    pub fn convert(actions: Vec<Action>) -> Result<Vec<CoreAction>, ConverterError> {
        let mut result: Vec<CoreAction> = Vec::new();
        for action in actions {
            if let Some(core_action) = Option::<CoreAction>::try_from(action)? {
                result.push(core_action)
            }
        }

        Ok(result)
    }
}
