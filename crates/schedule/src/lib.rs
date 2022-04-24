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
use std::{collections::HashMap, sync::Arc};
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
        let mut sched = JobScheduler::new();

        println!("{:?}", self.subscriptions);

        for (schedule, idx) in self.subscriptions.iter() {
            let name = &self.engine.config.components[*idx].id;
            log::info!(
                "Subscribed component #{} ({}) to schedule: {}",
                idx,
                name,
                schedule
            );
            sched.add(
                Job::new_async(name, |uuid, l| {
                    Box::pin(async move {
                        drop(
                            self.handle(schedule.to_string())
                                .await
                                .expect("Error scheduling task"),
                        );
                    })
                })
                .unwrap(),
            );
        }

        loop {
            sched.tick();
            std::thread::sleep(sched.time_till_next_job().map_err(|_| anyhow!("eerrr"))?);
        }
    }

    // Handle the message.
    async fn handle(&self, msg: String) -> Result<()> {
        log::info!("Received message on schedule: {:?}", msg);

        if let Some(idx) = self.subscriptions.get(&msg).copied() {
            let component = &self.engine.config.components[idx];
            let executor = self
                .component_triggers
                .get(component)
                .and_then(|t| t.executor.clone())
                .unwrap_or_default();

            match executor {
                spin_manifest::ScheduleExecutor::Spin => {
                    log::trace!("Executing Spin Schedule component {}", component.id);
                    let executor = SpinScheduleExecutor;
                    executor
                        .execute(&self.engine, &component.id, &msg.as_bytes())
                        .await?
                }
            };
        } else {
            log::debug!("No subscription found for {:?}", msg);
        }

        Ok(())
    }
}

/// The Schedule executor trait.
/// All Schedule executors must implement this trait.
#[async_trait]
pub(crate) trait ScheduleExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        payload: &[u8],
    ) -> Result<()>;
}

// #[cfg(test)]
// mod tests;
