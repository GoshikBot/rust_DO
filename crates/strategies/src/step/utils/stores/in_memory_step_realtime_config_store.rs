use crate::step::utils::entities::Diff;
use crate::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use crate::step::utils::stores::StepDiffs;
use anyhow::Result;
use base::entities::MovementType;

#[derive(Default)]
pub struct InMemoryStepRealtimeConfigStore {
    tendency: MovementType,
    tendency_changed_on_crossing_bargaining_corridor: bool,
    second_level_after_bargaining_tendency_change_is_created: bool,
    skip_creating_new_working_level: bool,
    diffs: StepDiffs,
}

impl InMemoryStepRealtimeConfigStore {
    pub fn new() -> Self {
        Default::default()
    }
}

impl StepRealtimeConfigStore for InMemoryStepRealtimeConfigStore {
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

    fn get_current_diff(&self) -> Result<Option<Diff>> {
        Ok(self.diffs.current)
    }

    fn update_current_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.diffs.current = Some(new_diff);

        Ok(())
    }

    fn get_previous_diff(&self) -> Result<Option<Diff>> {
        Ok(self.diffs.previous)
    }

    fn update_previous_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.diffs.previous = Some(new_diff);

        Ok(())
    }
}
