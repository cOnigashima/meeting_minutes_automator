//! ReconnectionManager - Audio Device Reconnection Logic
//!
//! Task 10.4 Phase 2 Implementation (2025-10-21 - Final Revision)
//! ========================================================
//!
//! ## Purpose
//! Implements STT-REQ-004.11: Auto-reconnect on device disconnect
//! - Max 3 attempts
//! - 5 second intervals
//! - Job-based architecture with Supervisor pattern
//!
//! ## Design Principles (Critical Review #4 Compliance)
//! 1. ✅ JobState pattern: Minimal state management (no HashMap, no generation counter)
//! 2. ✅ Supervisor pattern: Cleanup + UI notification centralized
//! 3. ✅ Immediate cancellation: abort() for instant user feedback
//! 4. ✅ Guaranteed cleanup: Supervisor handles abort/success/failure/panic uniformly
//! 5. ✅ Race-free: job_id comparison prevents old jobs from clearing current_job

use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio_device_adapter::AudioDeviceAdapter;

/// Cancellation reason with priority control
///
/// Priority: NewJob(3) > UserRequest(2) > UserManualResume(1)
/// Higher priority reasons cannot be overwritten by lower priority ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CancelReason {
    /// User manually resumed recording (is_recording=true)
    /// Priority: Low (can be overwritten by others)
    UserManualResume = 1,

    /// User called cancel_reconnection command
    /// Priority: Medium (can be overwritten by NewJob only)
    UserRequest = 2,

    /// New disconnect event triggered new job
    /// Priority: High (cannot be overwritten)
    NewJob = 3,
}

/// Set cancel reason with priority control
///
/// Priority: NewJob(3) > UserRequest(2) > UserManualResume(1)
/// Higher priority reasons cannot be overwritten by lower priority ones.
fn set_cancel_reason_priority(
    cancel_reason: &Arc<Mutex<Option<CancelReason>>>,
    new_reason: CancelReason,
) {
    let mut reason = cancel_reason.lock().unwrap();
    match *reason {
        None => {
            *reason = Some(new_reason);
        }
        Some(existing) if new_reason > existing => {
            *reason = Some(new_reason);
        }
        _ => {
            // Lower priority - don't overwrite
        }
    }
}

/// Reconnection attempt result
#[derive(Debug, Clone, PartialEq)]
pub enum ReconnectionResult {
    /// Reconnection successful
    Success { device_id: String, attempts: u32 },

    /// All retries exhausted
    Failed {
        device_id: String,
        attempts: u32,
        last_error: String,
    },

    /// Cancelled by user, manual resume, or new job
    Cancelled {
        device_id: String,
        attempt: u32,
        reason: Option<CancelReason>,
    },
}

/// Job state encapsulating a reconnection task
///
/// Contains all necessary information for a single reconnection job:
/// - Unique job ID for race-free cleanup
/// - Cancellation flag for cooperative cancellation
/// - Cancellation reason with priority control
/// - AbortHandle for immediate task cancellation
/// - Current attempt counter for UI progress tracking
/// - Device ID for UI notifications
///
/// Note: JoinHandle is NOT stored here because the supervisor consumes it.
/// The supervisor is responsible for awaiting the task and cleanup.
struct JobState {
    /// Unique job identifier
    id: u64,

    /// Atomic flag for lock-free cancellation
    cancel_flag: Arc<AtomicBool>,

    /// Cancellation reason with priority control
    cancel_reason: Arc<Mutex<Option<CancelReason>>>,

    /// AbortHandle for immediate task cancellation
    abort_handle: tokio::task::AbortHandle,

    /// Current attempt counter (for UI progress)
    #[allow(dead_code)]
    current_attempt: Arc<AtomicU32>,

    /// Device ID being reconnected
    device_id: String,
}

/// Reconnection manager for audio device recovery
///
/// Implements STT-REQ-004.11 retry policy with Supervisor pattern:
/// - Max 3 attempts (defined in reconnect_task)
/// - 5 second delay between attempts (defined in reconnect_task)
/// - Job-based architecture with unique job IDs
/// - Supervisor ensures cleanup in all cases (success/failure/abort/panic)
pub struct ReconnectionManager {
    /// Currently active reconnection job (if any)
    current_job: Option<JobState>,

    /// Next job ID counter
    next_job_id: AtomicU64,
}

impl ReconnectionManager {
    /// Create new ReconnectionManager
    pub fn new() -> Self {
        Self {
            current_job: None,
            next_job_id: AtomicU64::new(0),
        }
    }

    /// Start a new reconnection job
    ///
    /// Cancels any existing job and spawns a new reconnection task with a supervisor.
    /// The supervisor handles cleanup and UI notifications for all termination paths.
    ///
    /// # Arguments
    /// * `device_id` - Disconnected device identifier
    /// * `app` - Tauri AppHandle for state access and event emission
    pub fn start_job(&mut self, device_id: String, app: AppHandle) {
        // Cancel existing job if any
        if let Some(old_job) = self.current_job.take() {
            // Set cancel reason with priority control
            set_cancel_reason_priority(&old_job.cancel_reason, CancelReason::NewJob);
            old_job.cancel_flag.store(true, Ordering::Relaxed);
            old_job.abort_handle.abort();
            log_info_details!(
                "reconnection::job",
                "previous_job_cancelled",
                json!({
                    "reason": "new_disconnect_event",
                    "old_device": old_job.device_id
                })
            );
        }

        // Generate unique job ID
        let job_id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_reason = Arc::new(Mutex::new(None));
        let current_attempt = Arc::new(AtomicU32::new(0));

        let cancel_clone = cancel_flag.clone();
        let cancel_reason_clone = cancel_reason.clone();
        let current_attempt_clone = current_attempt.clone();
        let device_id_task = device_id.clone();
        let app_task = app.clone();

        // Spawn reconnection task
        let handle = tokio::spawn(async move {
            reconnect_task(
                app_task,
                device_id_task,
                cancel_clone,
                cancel_reason_clone,
                current_attempt_clone,
            )
            .await
        });

        // Get abort handle
        let abort_handle = handle.abort_handle();

        // Spawn supervisor: handles cleanup + UI notifications for ALL termination paths
        let app_supervisor = app.clone();
        let device_id_supervisor = device_id.clone();
        let attempt_supervisor = current_attempt.clone();
        let reason_supervisor = cancel_reason.clone();
        tokio::spawn(async move {
            // Wait for task completion (success/failure/cancelled/panic)
            let result = match handle.await {
                Ok(result) => result,
                Err(e) if e.is_cancelled() => {
                    // Task was aborted - retrieve attempt and reason
                    let attempt = attempt_supervisor.load(Ordering::Relaxed);
                    let reason = reason_supervisor.lock().unwrap().take();

                    log_info_details!(
                        "reconnection::supervisor",
                        "task_cancelled",
                        json!({
                            "job_id": job_id,
                            "attempt": attempt,
                            "reason": format!("{:?}", reason)
                        })
                    );

                    ReconnectionResult::Cancelled {
                        device_id: device_id_supervisor.clone(),
                        attempt,
                        reason,
                    }
                }
                Err(e) => {
                    // Task panicked
                    log_error_details!(
                        "reconnection::supervisor",
                        "task_panicked",
                        json!({
                            "job_id": job_id,
                            "error": format!("{:?}", e)
                        })
                    );
                    ReconnectionResult::Failed {
                        device_id: device_id_supervisor.clone(),
                        attempts: 0,
                        last_error: format!("Task panicked: {:?}", e),
                    }
                }
            };

            // Cleanup: Remove current_job if it's still this job
            {
                use crate::state::AppState;
                let state = app_supervisor.state::<AppState>();
                let mut mgr = state.reconnection_manager.lock().await;

                if mgr.current_job.as_ref().map(|j| j.id) == Some(job_id) {
                    mgr.current_job = None;
                    log_info_details!(
                        "reconnection::supervisor",
                        "job_cleaned_up",
                        json!({ "job_id": job_id })
                    );
                } else {
                    log_info_details!(
                        "reconnection::supervisor",
                        "job_already_replaced",
                        json!({
                            "job_id": job_id,
                            "current_job_id": mgr.current_job.as_ref().map(|j| j.id)
                        })
                    );
                }
            }

            // UI notifications (centralized in supervisor)
            match result {
                ReconnectionResult::Success { device_id, attempts } => {
                    log_info_details!(
                        "reconnection::supervisor",
                        "success",
                        json!({ "job_id": job_id, "device_id": device_id, "attempts": attempts })
                    );
                    let _ = app_supervisor.emit(
                        "device_reconnect_success",
                        json!({ "device_id": device_id, "attempts": attempts }),
                    );
                }
                ReconnectionResult::Failed {
                    device_id,
                    attempts,
                    last_error,
                } => {
                    log_error_details!(
                        "reconnection::supervisor",
                        "failed",
                        json!({
                            "job_id": job_id,
                            "device_id": device_id,
                            "attempts": attempts,
                            "error": last_error
                        })
                    );
                    let _ = app_supervisor.emit(
                        "device_reconnect_failed",
                        json!({
                            "device_id": device_id,
                            "attempts": attempts,
                            "error": last_error
                        }),
                    );
                }
                ReconnectionResult::Cancelled { device_id, attempt, reason } => {
                    // Task returned Cancelled
                    let reason_str = match reason {
                        Some(CancelReason::UserRequest) => "user_cancel",
                        Some(CancelReason::UserManualResume) => "user_manual_resume",
                        Some(CancelReason::NewJob) => "new_disconnect_event",
                        None => "unknown",
                    };
                    log_info_details!(
                        "reconnection::supervisor",
                        "cancelled",
                        json!({
                            "job_id": job_id,
                            "device_id": device_id,
                            "attempt": attempt,
                            "reason": reason_str
                        })
                    );
                    let _ = app_supervisor.emit(
                        "device_reconnect_cancelled",
                        json!({
                            "device_id": device_id,
                            "attempt": attempt,
                            "reason": reason_str
                        }),
                    );
                }
            }
        });

        // Register new job (handle is consumed by supervisor)
        self.current_job = Some(JobState {
            id: job_id,
            cancel_flag,
            cancel_reason,
            abort_handle,
            current_attempt,
            device_id: device_id.clone(),
        });

        log_info_details!(
            "reconnection::job",
            "started",
            json!({ "job_id": job_id, "device_id": device_id })
        );
    }

    /// Cancel the current reconnection job
    ///
    /// Sets the cancellation flag and reason with priority control.
    /// Uses both cooperative cancellation (flag) and immediate cancellation (abort).
    /// UI notifications and cleanup are handled by the supervisor.
    /// Safe to call even if no job is running.
    pub fn cancel(&mut self) {
        if let Some(job) = self.current_job.take() {
            // Set cancel reason with priority control
            set_cancel_reason_priority(&job.cancel_reason, CancelReason::UserRequest);
            job.cancel_flag.store(true, Ordering::Relaxed);
            job.abort_handle.abort();
            log_info_details!(
                "reconnection::job",
                "cancelled_by_user",
                json!({ "job_id": job.id, "device_id": job.device_id })
            );
        }
    }

    /// Check if a reconnection job is currently running
    pub fn is_reconnecting(&self) -> bool {
        self.current_job.is_some()
    }
}

impl Default for ReconnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Independent reconnection task
///
/// Implements the retry loop with lock-free cancellation and user operation protection.
/// Runs in a separate tokio task to avoid holding locks.
///
/// NOTE: UI notifications are now handled by the supervisor.
/// This task only returns ReconnectionResult for the supervisor to process.
///
/// # Arguments
/// * `app` - Tauri AppHandle for state access
/// * `device_id` - Disconnected device identifier
/// * `cancel_flag` - Atomic flag for lock-free cancellation
/// * `cancel_reason` - Shared cancellation reason with priority control
/// * `current_attempt` - Shared current attempt counter
///
/// # Returns
/// * `ReconnectionResult` - Success, Failed, or Cancelled (with reason)
async fn reconnect_task(
    app: AppHandle,
    device_id: String,
    cancel_flag: Arc<AtomicBool>,
    cancel_reason: Arc<Mutex<Option<CancelReason>>>,
    current_attempt: Arc<AtomicU32>,
) -> ReconnectionResult {
    use crate::state::AppState;

    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    log_info_details!(
        "reconnection::task",
        "started",
        json!({
            "device_id": device_id,
            "max_retries": MAX_RETRIES
        })
    );

    for attempt in 1..=MAX_RETRIES {
        // Update current attempt counter
        current_attempt.store(attempt, Ordering::Relaxed);

        log_info_details!(
            "reconnection::task",
            "attempt_start",
            json!({
                "device_id": device_id,
                "attempt": attempt,
                "max_attempts": MAX_RETRIES
            })
        );

        // Step 1: Check cancellation flag (lock-free)
        if cancel_flag.load(Ordering::Relaxed) {
            log_warn_details!(
                "reconnection::task",
                "cancelled_by_flag",
                json!({
                    "device_id": device_id,
                    "attempt": attempt
                })
            );
            let reason = cancel_reason.lock().unwrap().take();
            return ReconnectionResult::Cancelled {
                device_id,
                attempt,
                reason,
            };
        }

        // Step 2: Check if user manually resumed recording
        {
            let state = app.state::<AppState>();
            let is_recording = state.is_recording.lock().unwrap();
            if *is_recording {
                // Set cancel reason with priority control
                set_cancel_reason_priority(&cancel_reason, CancelReason::UserManualResume);
                log_info_details!(
                    "reconnection::task",
                    "user_resumed",
                    json!({
                        "device_id": device_id,
                        "attempt": attempt
                    })
                );
                let reason = cancel_reason.lock().unwrap().take();
                return ReconnectionResult::Cancelled {
                    device_id,
                    attempt,
                    reason,
                };
            }
        }

        // Step 3: Wait 5 seconds with cancellation monitoring
        let cancelled = tokio::select! {
            _ = tokio::time::sleep(RETRY_DELAY) => false,
            _ = async {
                loop {
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // Check cancel flag
                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    // Check user resumed
                    let state = app.state::<AppState>();
                    let is_recording = state.is_recording.lock().unwrap();
                    if *is_recording {
                        break;
                    }
                }
            } => true,
        };

        if cancelled {
            // Re-check to determine cancellation reason
            if cancel_flag.load(Ordering::Relaxed) {
                log_warn_details!(
                    "reconnection::task",
                    "cancelled_during_sleep",
                    json!({
                        "device_id": device_id,
                        "attempt": attempt
                    })
                );
            } else {
                // Set cancel reason with priority control
                set_cancel_reason_priority(&cancel_reason, CancelReason::UserManualResume);
                log_info_details!(
                    "reconnection::task",
                    "user_resumed_during_sleep",
                    json!({
                        "device_id": device_id,
                        "attempt": attempt
                    })
                );
            }
            let reason = cancel_reason.lock().unwrap().take();
            return ReconnectionResult::Cancelled {
                device_id,
                attempt,
                reason,
            };
        }

        // Step 4: Verify device exists in enumeration
        let device_exists = {
            let state = app.state::<AppState>();
            let audio_device = {
                let device_lock = state.audio_device.lock().unwrap();
                device_lock.clone()
            };

            match audio_device {
                Some(device) => {
                    let device_guard = device.lock().await;
                    match device_guard.enumerate_devices() {
                        Ok(devices) => devices.iter().any(|d| d.id == device_id),
                        Err(e) => {
                            log_error_details!(
                                "reconnection::task",
                                "enumerate_failed",
                                json!({
                                    "device_id": device_id,
                                    "attempt": attempt,
                                    "error": format!("{:?}", e)
                                })
                            );
                            false
                        }
                    }
                }
                None => {
                    log_error_details!(
                        "reconnection::task",
                        "audio_device_not_initialized",
                        json!({
                            "device_id": device_id,
                            "attempt": attempt
                        })
                    );
                    false
                }
            }
        };

        if !device_exists {
            log_warn_details!(
                "reconnection::task",
                "device_not_available",
                json!({
                    "device_id": device_id,
                    "attempt": attempt
                })
            );

            if attempt < MAX_RETRIES {
                continue; // Next attempt
            } else {
                // All retries exhausted
                let error_msg = "Device not available after all retries".to_string();
                return ReconnectionResult::Failed {
                    device_id,
                    attempts: MAX_RETRIES,
                    last_error: error_msg,
                };
            }
        }

        // Step 5: Attempt to start recording
        // Note: start_recording_internal is permissive (already recording = success)
        let state = app.state::<AppState>();
        match crate::commands::start_recording_internal(&app, &state, device_id.clone()).await {
            Ok(()) => {
                log_info_details!(
                    "reconnection::task",
                    "success",
                    json!({
                        "device_id": device_id,
                        "attempt": attempt
                    })
                );
                return ReconnectionResult::Success {
                    device_id,
                    attempts: attempt,
                };
            }
            Err(e) => {
                log_error_details!(
                    "reconnection::task",
                    "start_failed",
                    json!({
                        "device_id": device_id,
                        "attempt": attempt,
                        "error": e.clone()
                    })
                );

                if attempt < MAX_RETRIES {
                    continue; // Next attempt
                } else {
                    // All retries exhausted
                    return ReconnectionResult::Failed {
                        device_id,
                        attempts: MAX_RETRIES,
                        last_error: e,
                    };
                }
            }
        }
    }

    // Safety fallback (should never reach here)
    let error_msg = "Unexpected loop exit".to_string();
    ReconnectionResult::Failed {
        device_id,
        attempts: MAX_RETRIES,
        last_error: error_msg,
    }
}
