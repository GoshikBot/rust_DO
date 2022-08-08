use base::entities::order::{BasicOrderMainProperties, BasicOrderPrices};

use crate::step::utils::entities::working_levels::WLId;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepOrderProperties {
    pub main_props: StepOrderMainProperties,
    pub prices: BasicOrderPrices,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepOrderMainProperties {
    pub base: BasicOrderMainProperties,
    pub working_level_id: WLId,
}

impl Default for StepOrderMainProperties {
    fn default() -> Self {
        Self {
            base: Default::default(),
            working_level_id: String::from("1"),
        }
    }
}
