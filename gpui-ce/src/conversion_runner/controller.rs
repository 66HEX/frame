use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

use frame_core::{error::ConversionError, types::DEFAULT_MAX_CONCURRENCY};

use super::process::{pause_process, resume_process, terminate_process};

#[derive(Clone, Debug, Default)]
pub struct ConversionProcessController {
    state: Arc<Mutex<ConversionProcessState>>,
}

#[derive(Debug)]
struct ConversionProcessState {
    active_processes: HashMap<String, ActiveConversionProcess>,
    cancelled_tasks: HashSet<String>,
    max_concurrency: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveConversionProcess {
    pid: u32,
}

impl Default for ConversionProcessState {
    fn default() -> Self {
        Self {
            active_processes: HashMap::new(),
            cancelled_tasks: HashSet::new(),
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
        }
    }
}

impl ConversionProcessController {
    pub fn update_max_concurrency(&self, value: usize) -> Result<(), ConversionError> {
        if value == 0 {
            return Err(ConversionError::InvalidInput(
                "Max concurrency must be at least 1".to_string(),
            ));
        }

        let mut state = self.lock_state()?;
        state.max_concurrency = value;
        Ok(())
    }

    pub fn current_max_concurrency(&self) -> Result<usize, ConversionError> {
        Ok(self.lock_state()?.max_concurrency.max(1))
    }

    #[must_use]
    pub fn active_pid(&self, id: &str) -> Option<u32> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.active_processes.get(id).map(|process| process.pid))
    }

    #[must_use]
    pub fn is_cancelled(&self, id: &str) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.cancelled_tasks.contains(id))
    }

    pub fn register_started_process(&self, id: &str, pid: u32) -> Result<bool, ConversionError> {
        let was_cancelled = {
            let mut state = self.lock_state()?;
            state
                .active_processes
                .insert(id.to_string(), ActiveConversionProcess { pid });
            state.cancelled_tasks.contains(id)
        };

        if was_cancelled && pid > 0 {
            terminate_process(pid)?;
        }

        Ok(was_cancelled)
    }

    pub fn finish_task(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        state.active_processes.remove(id);
        Ok(state.cancelled_tasks.remove(id))
    }

    pub fn cancel_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = {
            let mut state = self.lock_state()?;
            state.cancelled_tasks.insert(id.to_string());
            state.active_processes.get(id).map(|process| process.pid)
        };

        if let Some(pid) = pid
            && pid > 0
        {
            terminate_process(pid)?;
        }

        Ok(())
    }

    pub fn pause_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = self
            .active_pid(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        pause_process(pid)
    }

    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let pid = self
            .active_pid(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        resume_process(pid)
    }

    pub fn take_cancelled(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        Ok(state.cancelled_tasks.remove(id))
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ConversionProcessState>, ConversionError> {
        self.state.lock().map_err(|error| {
            ConversionError::Worker(format!("process controller poisoned: {error}"))
        })
    }
}
