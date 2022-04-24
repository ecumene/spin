use crate::{spin_schedule::SpinSchedule, ExecutionContext, RuntimeContext, ScheduleExecutor};
use anyhow::Result;
use async_trait::async_trait;
use tokio::task::spawn_blocking;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinScheduleExecutor;

#[async_trait]
impl ScheduleExecutor for SpinScheduleExecutor {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        payload: &[u8],
    ) -> Result<()> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component
        );
        let (store, instance) = engine.prepare_component(component, None, None, None, None)?;

        match Self::execute_impl(store, instance, payload.to_vec()).await {
            Ok(()) => {
                log::trace!("Request finished OK");
                Ok(())
            }
            Err(e) => {
                log::trace!("Request finished with error {}", e);
                Err(e)
            }
        }
    }
}

impl SpinScheduleExecutor {
    pub async fn execute_impl(
        mut store: Store<RuntimeContext>,
        instance: Instance,
        payload: Vec<u8>,
    ) -> Result<()> {
        let engine = SpinSchedule::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

        let _res = spawn_blocking(move || -> Result<crate::spin_schedule::Error> {
            match engine.handle_scheduled_message(&mut store, &payload) {
                Ok(_) => Ok(crate::spin_schedule::Error::Success),
                Err(_) => Ok(crate::spin_schedule::Error::Error),
            }
        })
        .await??;

        Ok(())
    }
}
