//! Implementation for the Spin Scheduler engine.

mod spin;

use crate::spin::SpinScheduleExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use spin_engine::Builder;
use spin_manifest::{
    Application, ComponentMap, CoreComponent, ScheduleConfig, ScheduleTriggerConfiguration,
};
use spin_schedule::SpinScheduleData;
use std::str::FromStr;
use std::{collections::HashMap, sync::Arc};
use tokio::task::spawn;
use tokio_cron_scheduler::{Job, JobScheduler, JobToRun};

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-schedule.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinScheduleData>;
type RuntimeContext = spin_engine::RuntimeContext<SpinScheduleData>;

/// The Spin Schedule trigger.
#[derive(Clone)]
pub struct ScheduleTrigger {
    /// Trigger configuration.
    trigger_config: ScheduleTriggerConfiguration,
    /// Component trigger configurations.
    component_triggers: ComponentMap<ScheduleConfig>,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
    /// Map from channel name to tuple of cron-like syntax & index.
    subscriptions: HashMap<String, usize>,
}

impl ScheduleTrigger {
    /// Create a new Spin Schedule trigger.
    pub async fn new(
        mut builder: Builder<SpinScheduleData>,
        app: Application<CoreComponent>,
    ) -> Result<Self> {
        let trigger_config = app
            .info
            .trigger
            .as_schedule()
            .ok_or_else(|| anyhow!("Application trigger is not a schedule trigger"))?
            .clone();

        let component_triggers = app.component_triggers.try_map_values(|id, trigger| {
            trigger
                .as_schedule()
                .cloned()
                .ok_or_else(|| anyhow!("Expected Schedule configuration for component {}", id))
        })?;

        let subscriptions = app
            .components
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| component_triggers.get(c).map(|c| (c.cron.clone(), idx)))
            .collect();

        let engine = Arc::new(builder.build().await?);

        log::trace!("Created new Schedule trigger.");

        Ok(Self {
            trigger_config,
            component_triggers,
            engine,
            subscriptions,
        })
    }

    /// Run the Schedule trigger indefinitely.
    pub async fn run(&self) -> Result<()> {
        let mut sched = JobScheduler::new().map_err(|_| anyhow!("Failed to create scheduler"))?;

        println!("{:?}", self.subscriptions);

        for (schedule, idx) in self.subscriptions.iter() {
            let component = &self.engine.config.components[*idx];
            log::info!(
                "Subscribed component #{} ({}) to schedule: {}",
                idx,
                component.id,
                schedule
            );
            let executor = self
                .component_triggers
                .get(component)
                .and_then(|t| t.executor.clone());
            let engine = self.engine.clone();
            sched.add(
                Job::new_async(schedule.as_ref(), |uuid, l| {
                    Box::pin(async move {
                        let executor = executor.unwrap_or_default();
                        log::info!("Received message on schedule: {:?}", schedule);

                        match executor {
                            spin_manifest::ScheduleExecutor::Spin => {
                                log::trace!("Executing Spin Schedule component {}", component.id);
                                let executor = SpinScheduleExecutor;
                                executor
                                    .execute(&engine, &component.id, &schedule.as_bytes())
                                    .await
                                    .expect("Failed to execute schedule");
                            }
                        };
                    })
                })
                .unwrap(),
            );
        }

        loop {
            sched.tick();
            if let Some(duration) = sched.time_till_next_job().map_err(|_| anyhow!("eerrr"))? {
                std::thread::sleep(duration);
            }
        }
    }
}

/// The Schedule executor trait.
/// All Schedule executors must implement this trait.
#[async_trait]
pub(crate) trait ScheduleExecutor: Clone + Send + 'static {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        payload: &[u8],
    ) -> Result<()>;
}

// #[cfg(test)]
// mod tests;
