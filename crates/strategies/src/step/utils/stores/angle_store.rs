use anyhow::Result;
use base::entities::{candle::CandleId, Item};

use crate::step::utils::entities::angle::{AngleId, FullAngleProperties};

pub trait StepAngleStore {
    type AngleProperties;
    type CandleProperties;

    fn create_angle(
        &mut self,
        properties: Self::AngleProperties,
        candle_id: CandleId,
    ) -> Result<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>;

    fn get_angle_by_id(
        &self,
        id: &str,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn get_angle_of_second_level_after_bargaining_tendency_change(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_angle_of_second_level_after_bargaining_tendency_change(
        &mut self,
        new_angle: AngleId,
    ) -> Result<()>;

    fn get_tendency_change_angle(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_min_angle(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_max_angle(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle_before_bargaining_corridor(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle_before_bargaining_corridor(
        &self,
    ) -> Result<
        Option<Item<AngleId, FullAngleProperties<Self::AngleProperties, Self::CandleProperties>>>,
    >;

    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;
}
