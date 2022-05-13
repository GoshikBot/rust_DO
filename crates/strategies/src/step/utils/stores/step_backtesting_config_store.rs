use base::entities::MovementType;

use crate::step::utils::entities::Diff;
use anyhow::Result;

pub trait StepBacktestingConfigStore {
    fn get_tendency(&self) -> Result<MovementType>;
    fn update_tendency(&mut self, new_tendency: MovementType) -> Result<()>;

    fn tendency_changed_on_crossing_bargaining_corridor(&self) -> Result<bool>;
    fn update_tendency_changed_on_crossing_bargaining_corridor(
        &mut self,
        new_value: bool,
    ) -> Result<()>;

    fn second_level_after_bargaining_tendency_change_is_created(&self) -> Result<bool>;
    fn update_second_level_after_bargaining_tendency_change_is_created(
        &mut self,
        new_value: bool,
    ) -> Result<()>;

    fn skip_creating_new_working_level(&self) -> Result<bool>;
    fn update_skip_creating_new_working_level(&mut self, new_value: bool) -> Result<()>;

    fn no_trading_mode(&self) -> Result<bool>;
    fn update_no_trading_mode(&mut self, new_value: bool) -> Result<()>;

    fn get_current_diff(&self) -> Result<Option<Diff>>;
    fn update_current_diff(&mut self, new_diff: Diff) -> Result<()>;

    fn get_previous_diff(&self) -> Result<Option<Diff>>;
    fn update_previous_diff(&mut self, new_diff: Diff) -> Result<()>;
}
