use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::conversion::types::{CancelledPayload, ErrorPayload, LogPayload};

#[cfg(windows)]
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            Threading::{OpenProcess, PROCESS_SUSPEND_RESUME},
        },
    },
    core::s,
};

use crate::conversion::error::ConversionError;
use crate::conversion::types::{ConversionTask, DEFAULT_MAX_CONCURRENCY};
use crate::conversion::worker::run_ffmpeg_worker;

pub enum ManagerMessage {
    Enqueue(Box<ConversionTask>),
    ConcurrencyUpdated,
    TaskStarted(String, u32),
    TaskCompleted(String),
    TaskError(String, ConversionError),
}

pub struct ConversionManager {
    pub(crate) sender: mpsc::Sender<ManagerMessage>,
    max_concurrency: Arc<AtomicUsize>,
    active_tasks: Arc<Mutex<HashMap<String, ActiveProcess>>>,
    cancelled_tasks: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone, Copy)]
struct ActiveProcess {
    pid: u32,
    start_time: u64,
}

impl ConversionManager {
    pub fn new(app: AppHandle) -> Self {
        let (tx, mut rx) = mpsc::channel(32);
        let tx_clone = tx.clone();
        let max_concurrency = Arc::new(AtomicUsize::new(DEFAULT_MAX_CONCURRENCY));
        let limiter = Arc::clone(&max_concurrency);
        let active_tasks = Arc::new(Mutex::new(HashMap::new()));
        let active_tasks_loop = Arc::clone(&active_tasks);
        let cancelled_tasks = Arc::new(Mutex::new(HashSet::new()));
        let cancelled_tasks_loop = Arc::clone(&cancelled_tasks);

        tauri::async_runtime::spawn(async move {
            let mut queue: VecDeque<ConversionTask> = VecDeque::new();
            let mut queued_ids: HashSet<String> = HashSet::new();
            let mut running_tasks: HashMap<String, ()> = HashMap::new();

            while let Some(msg) = rx.recv().await {
                match msg {
                    ManagerMessage::Enqueue(task) => {
                        let task = *task;
                        {
                            let mut cancelled = cancelled_tasks_loop.lock().unwrap();
                            cancelled.remove(&task.id);
                        }

                        if running_tasks.contains_key(&task.id) || queued_ids.contains(&task.id) {
                            continue;
                        }

                        queued_ids.insert(task.id.clone());
                        queue.push_back(task);
                        Self::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        );
                    }
                    ManagerMessage::ConcurrencyUpdated => {
                        Self::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        );
                    }
                    ManagerMessage::TaskStarted(id, pid) => {
                        let is_cancelled = {
                            let cancelled = cancelled_tasks_loop.lock().unwrap();
                            cancelled.contains(&id)
                        };

                        if is_cancelled {
                            if pid > 0 {
                                let _ = Self::terminate_process(pid);
                            }
                            running_tasks.remove(&id);
                            {
                                let mut tasks = active_tasks_loop.lock().unwrap();
                                tasks.remove(&id);
                            }
                            Self::process_queue(
                                &app,
                                &tx_clone,
                                &mut queue,
                                &mut queued_ids,
                                &mut running_tasks,
                                Arc::clone(&limiter),
                                Arc::clone(&cancelled_tasks_loop),
                            );
                            continue;
                        }

                        let mut tasks = active_tasks_loop.lock().unwrap();
                        tasks.insert(
                            id,
                            ActiveProcess {
                                pid,
                                start_time: process_start_time(pid).unwrap_or(0),
                            },
                        );
                    }
                    ManagerMessage::TaskCompleted(id) => {
                        {
                            let mut cancelled = cancelled_tasks_loop.lock().unwrap();
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            let _ = finalize_task_state(
                                &id,
                                &mut running_tasks,
                                &mut tasks,
                                &mut cancelled,
                            );
                        }

                        Self::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        );
                    }
                    ManagerMessage::TaskError(id, err) => {
                        let was_cancelled = {
                            let mut cancelled = cancelled_tasks_loop.lock().unwrap();
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            finalize_task_state(&id, &mut running_tasks, &mut tasks, &mut cancelled)
                        };

                        if was_cancelled {
                            let _ = app.emit(
                                "conversion-log",
                                LogPayload {
                                    id: id.clone(),
                                    line: "[INFO] Task cancelled".to_string(),
                                },
                            );
                            let _ = app
                                .emit("conversion-cancelled", CancelledPayload { id: id.clone() });
                        } else {
                            eprintln!("Task {id} failed: {err}");
                            let _ = app.emit(
                                "conversion-log",
                                LogPayload {
                                    id: id.clone(),
                                    line: format!("[ERROR] {err}"),
                                },
                            );
                            let _ = app.emit(
                                "conversion-error",
                                ErrorPayload {
                                    id: id.clone(),
                                    error: err.to_string(),
                                },
                            );
                        }

                        Self::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        );
                    }
                }
            }
        });

        Self {
            sender: tx,
            max_concurrency,
            active_tasks,
            cancelled_tasks,
        }
    }

    fn process_queue(
        app: &AppHandle,
        tx: &mpsc::Sender<ManagerMessage>,
        queue: &mut VecDeque<ConversionTask>,
        queued_ids: &mut HashSet<String>,
        running_tasks: &mut HashMap<String, ()>,
        max_concurrency: Arc<AtomicUsize>,
        cancelled_tasks: Arc<Mutex<HashSet<String>>>,
    ) {
        let limit = max_concurrency.load(Ordering::SeqCst).max(1);

        while running_tasks.len() < limit {
            if let Some(task) = queue.pop_front() {
                queued_ids.remove(&task.id);
                let is_cancelled = {
                    let mut cancelled = cancelled_tasks.lock().unwrap();
                    cancelled.remove(&task.id)
                };
                if is_cancelled {
                    continue;
                }

                running_tasks.insert(task.id.clone(), ());

                let app_clone = app.clone();
                let tx_worker = tx.clone();
                let task_clone = task.clone();

                tauri::async_runtime::spawn(async move {
                    if let Err(e) =
                        run_ffmpeg_worker(app_clone, tx_worker.clone(), task_clone.clone()).await
                    {
                        let _ = tx_worker
                            .send(ManagerMessage::TaskError(task_clone.id, e))
                            .await;
                    } else {
                        let _ = tx_worker
                            .send(ManagerMessage::TaskCompleted(task_clone.id))
                            .await;
                    }
                });
            } else {
                break;
            }
        }
    }

    pub fn current_max_concurrency(&self) -> usize {
        self.max_concurrency.load(Ordering::SeqCst)
    }

    pub fn update_max_concurrency(&self, value: usize) -> Result<(), ConversionError> {
        if value == 0 {
            return Err(ConversionError::InvalidInput(
                "Max concurrency must be at least 1".to_string(),
            ));
        }
        self.max_concurrency.store(value, Ordering::SeqCst);
        let tx = self.sender.clone();
        tauri::async_runtime::spawn(async move {
            let _ = tx.send(ManagerMessage::ConcurrencyUpdated).await;
        });
        Ok(())
    }

    pub fn pause_task(&self, id: &str) -> Result<(), ConversionError> {
        let process = {
            let tasks = self.active_tasks.lock().unwrap();
            tasks.get(id).copied()
        };

        if let Some(process) = process {
            if process.pid == 0 {
                return Err(ConversionError::TaskNotFound(id.to_string()));
            }
            ensure_same_process(id, process)?;

            #[cfg(unix)]
            unsafe {
                if libc::kill(process.pid as libc::pid_t, libc::SIGSTOP) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGSTOP".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(process.pid, true)?;
            }

            Ok(())
        } else {
            Err(ConversionError::TaskNotFound(id.to_string()))
        }
    }

    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let process = {
            let tasks = self.active_tasks.lock().unwrap();
            tasks.get(id).copied()
        };

        if let Some(process) = process {
            if process.pid == 0 {
                return Err(ConversionError::TaskNotFound(id.to_string()));
            }
            ensure_same_process(id, process)?;

            #[cfg(unix)]
            unsafe {
                if libc::kill(process.pid as libc::pid_t, libc::SIGCONT) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGCONT".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(process.pid, false)?;
            }

            Ok(())
        } else {
            Err(ConversionError::TaskNotFound(id.to_string()))
        }
    }

    pub fn cancel_task(&self, id: &str) -> Result<(), ConversionError> {
        {
            let mut cancelled = self.cancelled_tasks.lock().unwrap();
            cancelled.insert(id.to_string());
        }

        let process = {
            let tasks = self.active_tasks.lock().unwrap();
            tasks.get(id).copied()
        };

        if let Some(process) = process {
            if process.pid > 0 {
                ensure_same_process(id, process)?;
                Self::terminate_process(process.pid)?;
            }
            Self::cleanup_temp_upscale_dir(id);
            Ok(())
        } else {
            Self::cleanup_temp_upscale_dir(id);
            Ok(())
        }
    }

    fn cleanup_temp_upscale_dir(id: &str) {
        let temp_dir = std::env::temp_dir().join(format!("frame_upscale_{id}"));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
    }

    #[cfg(unix)]
    fn terminate_process(pid: u32) -> Result<(), ConversionError> {
        unsafe {
            let _ = libc::kill(pid as libc::pid_t, libc::SIGCONT);
            if libc::kill(pid as libc::pid_t, libc::SIGKILL) != 0 {
                return Err(ConversionError::Shell("Failed to send SIGKILL".to_string()));
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn terminate_process(pid: u32) -> Result<(), ConversionError> {
        unsafe {
            let _ = windows_suspend_resume(pid, false);

            let process_handle = OpenProcess(
                windows::Win32::System::Threading::PROCESS_TERMINATE,
                false,
                pid,
            )
            .map_err(|e| {
                ConversionError::Shell(format!("Failed to open process for termination: {}", e))
            })?;

            let _ = windows::Win32::System::Threading::TerminateProcess(process_handle, 1);
            let _ = CloseHandle(process_handle);
        }
        Ok(())
    }
}

fn process_start_time(pid: u32) -> Option<u64> {
    if pid == 0 {
        return None;
    }
    let target = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[target]), true);
    system.process(target).map(sysinfo::Process::start_time)
}

fn ensure_same_process(id: &str, process: ActiveProcess) -> Result<(), ConversionError> {
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

fn finalize_task_state(
    id: &str,
    running_tasks: &mut HashMap<String, ()>,
    active_tasks: &mut HashMap<String, ActiveProcess>,
    cancelled_tasks: &mut HashSet<String>,
) -> bool {
    running_tasks.remove(id);
    active_tasks.remove(id);
    cancelled_tasks.remove(id)
}

#[cfg(test)]
mod tests {
    use super::{ActiveProcess, ensure_same_process, finalize_task_state, process_start_time};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn finalize_task_state_cleans_all_maps_for_cancelled_task() {
        let id = "task-1";
        let mut running = HashMap::from([(id.to_string(), ())]);
        let mut active = HashMap::from([(
            id.to_string(),
            ActiveProcess {
                pid: 42,
                start_time: 7,
            },
        )]);
        let mut cancelled = HashSet::from([id.to_string()]);

        let was_cancelled = finalize_task_state(id, &mut running, &mut active, &mut cancelled);

        assert!(was_cancelled);
        assert!(!running.contains_key(id));
        assert!(!active.contains_key(id));
        assert!(!cancelled.contains(id));
    }

    #[test]
    fn finalize_task_state_cleans_all_maps_for_non_cancelled_task() {
        let id = "task-2";
        let mut running = HashMap::from([(id.to_string(), ())]);
        let mut active = HashMap::from([(
            id.to_string(),
            ActiveProcess {
                pid: 55,
                start_time: 9,
            },
        )]);
        let mut cancelled = HashSet::<String>::new();

        let was_cancelled = finalize_task_state(id, &mut running, &mut active, &mut cancelled);

        assert!(!was_cancelled);
        assert!(!running.contains_key(id));
        assert!(!active.contains_key(id));
    }

    #[test]
    fn ensure_same_process_accepts_current_process_identity() {
        let pid = std::process::id();
        let start_time =
            process_start_time(pid).expect("Current process start time should be readable");

        let result = ensure_same_process("self", ActiveProcess { pid, start_time });

        assert!(result.is_ok());
    }

    #[test]
    fn ensure_same_process_rejects_mismatched_start_time() {
        let pid = std::process::id();
        let start_time =
            process_start_time(pid).expect("Current process start time should be readable");

        let err = ensure_same_process(
            "self",
            ActiveProcess {
                pid,
                start_time: start_time.saturating_add(1),
            },
        )
        .expect_err("Mismatched process start time should fail");

        assert!(
            err.to_string().contains("Task not found"),
            "Unexpected error: {err}"
        );
    }
}

#[cfg(windows)]
unsafe fn windows_suspend_resume(pid: u32, suspend: bool) -> Result<(), ConversionError> {
    let process_handle = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)
        .map_err(|e| ConversionError::Shell(format!("Failed to open process: {}", e)))?;

    let ntdll = GetModuleHandleA(s!("ntdll.dll")).map_err(|e| {
        let _ = CloseHandle(process_handle);
        ConversionError::Shell(format!("Failed to get ntdll handle: {}", e))
    })?;

    let fn_name = if suspend {
        s!("NtSuspendProcess")
    } else {
        s!("NtResumeProcess")
    };

    let func_ptr = GetProcAddress(ntdll, fn_name);

    if let Some(func) = func_ptr {
        let func: extern "system" fn(HANDLE) -> i32 = std::mem::transmute(func);
        let status = func(process_handle);
        let _ = CloseHandle(process_handle);

        if status != 0 {
            return Err(ConversionError::Shell(format!(
                "NtSuspendProcess/NtResumeProcess failed with status: {}",
                status
            )));
        }
        Ok(())
    } else {
        let _ = CloseHandle(process_handle);
        Err(ConversionError::Shell(
            "Could not find NtSuspendProcess/NtResumeProcess in ntdll".to_string(),
        ))
    }
}
