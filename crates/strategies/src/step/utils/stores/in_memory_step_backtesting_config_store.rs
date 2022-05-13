use crate::step::utils::entities::strategies::StrategyDiffs;
use crate::step::utils::entities::Diff;
use crate::step::utils::stores::step_backtesting_config_store::StepBacktestingConfigStore;
use anyhow::Result;
use base::entities::MovementType;

pub struct InMemoryStepBacktestingConfigStore {
    tendency: MovementType,
    tendency_changed_on_crossing_bargaining_corridor: bool,
    second_level_after_bargaining_tendency_change_is_created: bool,
    skip_creating_new_working_level: bool,
    no_trading_mode: bool,
    diffs: StrategyDiffs,
}

impl StepBacktestingConfigStore for InMemoryStepBacktestingConfigStore {
    fn get_tendency(&self) -> Result<MovementType> {
        Ok(self.tendency)
    }

    fn update_tendency(&mut self, new_tendency: MovementType) -> Result<()> {
        self.tendency = new_tendency;

        Ok(())
    }

    fn tendency_changed_on_crossing_bargaining_corridor(&self) -> Result<bool> {
        Ok(self.tendency_changed_on_crossing_bargaining_corridor)
    }

    fn update_tendency_changed_on_crossing_bargaining_corridor(
        &mut self,
        new_value: bool,
    ) -> Result<()> {
        self.tendency_changed_on_crossing_bargaining_corridor = new_value;

        Ok(())
    }

    fn second_level_after_bargaining_tendency_change_is_created(&self) -> Result<bool> {
        Ok(self.second_level_after_bargaining_tendency_change_is_created)
    }

    fn update_second_level_after_bargaining_tendency_change_is_created(
        &mut self,
        new_value: bool,
    ) -> Result<()> {
        self.second_level_after_bargaining_tendency_change_is_created = new_value;

        Ok(())
    }

    fn skip_creating_new_working_level(&self) -> Result<bool> {
        Ok(self.skip_creating_new_working_level)
    }

    fn update_skip_creating_new_working_level(&mut self, new_value: bool) -> Result<()> {
        self.skip_creating_new_working_level = new_value;

        Ok(())
    }

    fn no_trading_mode(&self) -> Result<bool> {
        Ok(self.no_trading_mode)
    }

    fn update_no_trading_mode(&mut self, new_value: bool) -> Result<()> {
        self.no_trading_mode = new_value;

        Ok(())
    }

    fn get_current_diff(&self) -> Result<Option<Diff>> {
        Ok(self.diffs.current_diff)
    }

    fn update_current_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.diffs.current_diff = Some(new_diff);

        Ok(())
    }

    fn get_previous_diff(&self) -> Result<Option<Diff>> {
        Ok(self.diffs.previous_diff)
    }

    fn update_previous_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.diffs.previous_diff = Some(new_diff);

        Ok(())
    }
}
