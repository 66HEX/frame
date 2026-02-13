use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::conversion::types::{ErrorPayload, LogPayload};

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
    active_tasks: Arc<Mutex<HashMap<String, u32>>>,
    cancelled_tasks: Arc<Mutex<HashSet<String>>>,
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
                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        )
                        .await;
                    }
                    ManagerMessage::ConcurrencyUpdated => {
                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        )
                        .await;
                    }
                    ManagerMessage::TaskStarted(id, pid) => {
                        let is_cancelled = {
                            let cancelled = cancelled_tasks_loop.lock().unwrap();
                            cancelled.contains(&id)
                        };

                        if is_cancelled {
                            if pid > 0 {
                                let _ = ConversionManager::terminate_process(pid);
                            }
                            running_tasks.remove(&id);
                            {
                                let mut tasks = active_tasks_loop.lock().unwrap();
                                tasks.remove(&id);
                            }
                            ConversionManager::process_queue(
                                &app,
                                &tx_clone,
                                &mut queue,
                                &mut queued_ids,
                                &mut running_tasks,
                                Arc::clone(&limiter),
                                Arc::clone(&cancelled_tasks_loop),
                            )
                            .await;
                            continue;
                        }

                        let mut tasks = active_tasks_loop.lock().unwrap();
                        tasks.insert(id, pid);
                    }
                    ManagerMessage::TaskCompleted(id) => {
                        running_tasks.remove(&id);
                        {
                            let mut cancelled = cancelled_tasks_loop.lock().unwrap();
                            cancelled.remove(&id);
                        }
                        {
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            tasks.remove(&id);
                        }

                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        )
                        .await;
                    }
                    ManagerMessage::TaskError(id, err) => {
                        eprintln!("Task {} failed: {}", id, err);

                        let _ = app.emit(
                            "conversion-log",
                            LogPayload {
                                id: id.clone(),
                                line: format!("[ERROR] {}", err),
                            },
                        );

                        let _ = app.emit(
                            "conversion-error",
                            ErrorPayload {
                                id: id.clone(),
                                error: err.to_string(),
                            },
                        );

                        running_tasks.remove(&id);
                        {
                            let mut cancelled = cancelled_tasks_loop.lock().unwrap();
                            cancelled.remove(&id);
                        }
                        {
                            let mut tasks = active_tasks_loop.lock().unwrap();
                            tasks.remove(&id);
                        }

                        ConversionManager::process_queue(
                            &app,
                            &tx_clone,
                            &mut queue,
                            &mut queued_ids,
                            &mut running_tasks,
                            Arc::clone(&limiter),
                            Arc::clone(&cancelled_tasks_loop),
                        )
                        .await;
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

    async fn process_queue(
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
        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            if pid == 0 {
                return Err(ConversionError::TaskNotFound(id.to_string()));
            }

            #[cfg(unix)]
            unsafe {
                if libc::kill(pid as libc::pid_t, libc::SIGSTOP) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGSTOP".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(pid, true)?;
            }

            Ok(())
        } else {
            Err(ConversionError::TaskNotFound(id.to_string()))
        }
    }

    pub fn resume_task(&self, id: &str) -> Result<(), ConversionError> {
        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            if pid == 0 {
                return Err(ConversionError::TaskNotFound(id.to_string()));
            }

            #[cfg(unix)]
            unsafe {
                if libc::kill(pid as libc::pid_t, libc::SIGCONT) != 0 {
                    return Err(ConversionError::Shell("Failed to send SIGCONT".to_string()));
                }
            }

            #[cfg(windows)]
            unsafe {
                windows_suspend_resume(pid, false)?;
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

        let tasks = self.active_tasks.lock().unwrap();
        if let Some(&pid) = tasks.get(id) {
            if pid > 0 {
                ConversionManager::terminate_process(pid)?;
            }
            ConversionManager::cleanup_temp_upscale_dir(id);
            Ok(())
        } else {
            ConversionManager::cleanup_temp_upscale_dir(id);
            Ok(())
        }
    }

    fn cleanup_temp_upscale_dir(id: &str) {
        let temp_dir = std::env::temp_dir().join(format!("frame_upscale_{}", id));
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
