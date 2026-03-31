use bevy_ecs::schedule::{ExecutorKind, Schedule, ScheduleLabel};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Startup;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedUpdate;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreRender;

pub struct EngineSchedules {
    pub startup: Schedule,
    pub fixed_update: Schedule,
    pub update: Schedule,
    pub pre_render: Schedule,
}

impl Default for EngineSchedules {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineSchedules {
    pub fn new() -> Self {
        let mut startup = Schedule::new(Startup);
        startup.set_executor_kind(ExecutorKind::MultiThreaded);

        let mut fixed_update = Schedule::new(FixedUpdate);
        fixed_update.set_executor_kind(ExecutorKind::MultiThreaded);

        let mut update = Schedule::new(Update);
        update.set_executor_kind(ExecutorKind::MultiThreaded);

        let mut pre_render = Schedule::new(PreRender);
        pre_render.set_executor_kind(ExecutorKind::MultiThreaded);

        Self {
            startup,
            fixed_update,
            update,
            pre_render,
        }
    }
}
