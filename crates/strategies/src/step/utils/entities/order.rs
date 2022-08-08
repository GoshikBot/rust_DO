use base::entities::order::BasicOrderProperties;

use crate::step::utils::entities::working_levels::WLId;

#[derive(Debug, Clone, PartialEq)]
pub struct StepOrderProperties {
    pub base: BasicOrderProperties,
    pub working_level_id: WLId,
}

impl Default for StepOrderProperties {
    fn default() -> Self {
        Self {
            base: BasicOrderProperties::default(),
            working_level_id: String::from("1"),
        }
    }
}
