use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

use frame_core::{error::ConversionError, types::DEFAULT_MAX_CONCURRENCY};
use sysinfo::{Pid, ProcessesToUpdate, System};

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
    start_time: u64,
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
    /// Updates the maximum number of conversion processes allowed to run at once.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is zero or the controller state mutex is
    /// poisoned.
    pub fn update_max_concurrency(&self, value: usize) -> Result<(), ConversionError> {
        if value == 0 {
            return Err(ConversionError::InvalidInput(
                "Max concurrency must be at least 1".to_string(),
            ));
        }

        {
            let mut state = self.lock_state()?;
            state.max_concurrency = value;
        }
        Ok(())
    }

    /// Returns the current conversion concurrency limit.
    ///
    /// # Errors
    ///
    /// Returns an error when the controller state mutex is poisoned.
    pub fn current_max_concurrency(&self) -> Result<usize, ConversionError> {
        Ok(self.lock_state()?.max_concurrency.max(1))
    }

    #[must_use]
    pub fn active_pid(&self, id: &str) -> Option<u32> {
        self.active_process(id).map(|process| process.pid)
    }

    #[cfg(test)]
    #[must_use]
    pub(super) fn active_start_time(&self, id: &str) -> Option<u64> {
        self.active_process(id).map(|process| process.start_time)
    }

    #[must_use]
    pub fn is_cancelled(&self, id: &str) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.cancelled_tasks.contains(id))
    }

    /// Records a started worker process for a conversion task.
    ///
    /// # Errors
    ///
    /// Returns an error when the controller state mutex is poisoned or when a
    /// pre-cancelled process cannot be terminated.
    pub fn register_started_process(&self, id: &str, pid: u32) -> Result<bool, ConversionError> {
        let process = ActiveConversionProcess {
            pid,
            start_time: process_start_time(pid).unwrap_or(0),
        };
        let was_cancelled = {
            let mut state = self.lock_state()?;
            state.active_processes.insert(id.to_string(), process);
            state.cancelled_tasks.contains(id)
        };

        if was_cancelled && pid > 0 {
            terminate_process(pid)?;
        }

        Ok(was_cancelled)
    }

    /// Removes a finished task from process tracking.
    ///
    /// # Errors
    ///
    /// Returns an error when the controller state mutex is poisoned.
    pub fn finish_task(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        state.active_processes.remove(id);
        Ok(state.cancelled_tasks.remove(id))
    }

    /// Marks a task as cancelled and terminates its active process when present.
    ///
    /// # Errors
    ///
    /// Returns an error when the controller state mutex is poisoned, the active
    /// process no longer matches the tracked process, or termination fails.
    pub fn cancel_task(&self, id: &str) -> Result<(), ConversionError> {
        let process = {
            let mut state = self.lock_state()?;
            state.cancelled_tasks.insert(id.to_string());
            state.active_processes.get(id).copied()
        };

        if let Some(process) = process
            && process.pid > 0
        {
            ensure_same_process(id, process)?;
            terminate_process(process.pid)?;
        }

        Ok(())
    }

    /// Pauses the process associated with a task.
    ///
    /// # Errors
    ///
    /// Returns an error when the task is not active, the tracked process has
    /// changed, or pausing the process fails.
    pub fn pause_task(&self, id: &str) -> Result<(), ConversionError> {
        let process = self
            .active_process(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        ensure_same_process(id, process)?;
        pause_process(process.pid)
    }

    /// Resumes the process associated with a task.
    ///
    /// # Errors
    ///
    /// Returns an error when the task is not active, the tracked process has
    /// changed, or resuming the process fails.
    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let process = self
            .active_process(id)
            .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;
        ensure_same_process(id, process)?;
        resume_process(process.pid)
    }

    /// Removes and returns the cancellation marker for a task.
    ///
    /// # Errors
    ///
    /// Returns an error when the controller state mutex is poisoned.
    pub fn take_cancelled(&self, id: &str) -> Result<bool, ConversionError> {
        let mut state = self.lock_state()?;
        Ok(state.cancelled_tasks.remove(id))
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ConversionProcessState>, ConversionError> {
        self.state.lock().map_err(|error| {
            ConversionError::Worker(format!("process controller poisoned: {error}"))
        })
    }

    fn active_process(&self, id: &str) -> Option<ActiveConversionProcess> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.active_processes.get(id).copied())
    }
}

fn process_start_time(pid: u32) -> Option<u64> {
    if pid == 0 {
        return None;
    }

    let target = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[target]));
    system.process(target).map(sysinfo::Process::start_time)
}

fn ensure_same_process(id: &str, process: ActiveConversionProcess) -> Result<(), ConversionError> {
    if process.start_time == 0 {
        return Ok(());
    }

    let current_start = process_start_time(process.pid)
        .ok_or_else(|| ConversionError::TaskNotFound(id.to_string()))?;

    if current_start != process.start_time {
        return Err(ConversionError::TaskNotFound(id.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_same_process_accepts_current_process_identity() {
        let pid = std::process::id();
        let start_time =
            process_start_time(pid).expect("current process start time should be readable");

        let result = ensure_same_process("self", ActiveConversionProcess { pid, start_time });

        assert!(result.is_ok());
    }

    #[test]
    fn ensure_same_process_rejects_mismatched_start_time() {
        let pid = std::process::id();
        let start_time =
            process_start_time(pid).expect("current process start time should be readable");

        let error = ensure_same_process(
            "self",
            ActiveConversionProcess {
                pid,
                start_time: start_time.saturating_add(1),
            },
        )
        .expect_err("mismatched process identity should fail");

        assert!(
            error.to_string().contains("Task not found"),
            "unexpected error: {error}"
        );
    }
}
