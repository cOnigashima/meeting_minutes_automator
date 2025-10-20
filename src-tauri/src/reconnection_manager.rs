//! ReconnectionManager - Audio Device Reconnection Logic
//!
//! Phase 0.2 Design (2025-10-20)
//! ===============================
//!
//! ## Purpose
//! Extract device reconnection logic from commands.rs (932 lines) to dedicated module
//! for testability and maintainability.
//!
//! ## Requirements
//! - STT-REQ-004.11: Auto-reconnect (max 3 attempts, 5s intervals)
//!
//! ## Design Rationale
//!
//! ### Why New Module (vs audio_device_adapter.rs Extension)?
//! **Rejected**: Extending audio_device_adapter.rs
//! - **Reason**: Violates Single Responsibility Principle
//! - Device adapter layer should handle hardware abstraction only
//! - Reconnection policy is application-level logic, not device-level
//!
//! **Accepted**: New reconnection_manager.rs module
//! - **Benefits**:
//!   - Isolates retry policy from device abstraction
//!   - Enables comprehensive unit testing
//!   - Future-proof for advanced policies (exponential backoff, device health tracking)
//!   - Reduces commands.rs bloat (currently 932 lines)
//!
//! ## Module Interface
//!
//! ```rust
//! use crate::audio_device_adapter::{AudioDeviceAdapter, AudioDeviceEvent};
//! use anyhow::Result;
//! use std::time::Duration;
//!
//! /// Reconnection attempt result
//! #[derive(Debug, Clone, PartialEq)]
//! pub enum ReconnectionResult {
//!     /// Reconnection successful
//!     Success { device_id: String, attempts: u32 },
//!
//!     /// All retries exhausted
//!     Failed { device_id: String, attempts: u32, last_error: String },
//!
//!     /// Reconnection in progress
//!     InProgress { device_id: String, attempt: u32, max_attempts: u32 },
//! }
//!
//! /// Reconnection manager for audio device recovery
//! ///
//! /// Implements STT-REQ-004.11 retry policy:
//! /// - Max 3 attempts
//! /// - 5 second delay between attempts
//! /// - Preserves last device_id for reconnection
//! pub struct ReconnectionManager {
//!     /// Maximum reconnection attempts (STT-REQ-004.11)
//!     max_retries: u32,
//!
//!     /// Delay between retry attempts (STT-REQ-004.11)
//!     retry_delay: Duration,
//!
//!     /// Current attempt count (0-indexed)
//!     attempt_count: u32,
//!
//!     /// Last disconnected device ID
//!     last_device_id: Option<String>,
//!
//!     /// Reconnection state
//!     is_reconnecting: bool,
//! }
//!
//! impl ReconnectionManager {
//!     /// Create new ReconnectionManager with default policy (STT-REQ-004.11)
//!     pub fn new() -> Self {
//!         Self {
//!             max_retries: 3,                      // STT-REQ-004.11
//!             retry_delay: Duration::from_secs(5), // STT-REQ-004.11
//!             attempt_count: 0,
//!             last_device_id: None,
//!             is_reconnecting: false,
//!         }
//!     }
//!
//!     /// Handle device disconnect event
//!     ///
//!     /// Initiates reconnection sequence by storing device_id and resetting attempt count.
//!     ///
//!     /// # Arguments
//!     /// * `device_id` - Disconnected device identifier
//!     pub fn handle_disconnect(&mut self, device_id: String) {
//!         self.last_device_id = Some(device_id);
//!         self.attempt_count = 0;
//!         self.is_reconnecting = true;
//!     }
//!
//!     /// Attempt reconnection to last disconnected device
//!     ///
//!     /// Implements retry loop with delay. Returns immediately if max retries exceeded.
//!     ///
//!     /// # Arguments
//!     /// * `adapter` - Mutable reference to audio device adapter
//!     ///
//!     /// # Returns
//!     /// * `Ok(ReconnectionResult::Success)` - Reconnection successful
//!     /// * `Ok(ReconnectionResult::Failed)` - All retries exhausted
//!     /// * `Err` - Unexpected error during reconnection
//!     ///
//!     /// # Example
//!     /// ```no_run
//!     /// use meeting_minutes_automator_lib::reconnection_manager::ReconnectionManager;
//!     /// use meeting_minutes_automator_lib::audio_device_adapter::create_audio_adapter;
//!     ///
//!     /// #[tokio::main]
//!     /// async fn main() {
//!     ///     let mut manager = ReconnectionManager::new();
//!     ///     let mut adapter = create_audio_adapter().unwrap();
//!     ///
//!     ///     manager.handle_disconnect("device-123".to_string());
//!     ///
//!     ///     match manager.attempt_reconnect(&mut *adapter).await {
//!     ///         Ok(result) => println!("Result: {:?}", result),
//!     ///         Err(e) => eprintln!("Error: {}", e),
//!     ///     }
//!     /// }
//!     /// ```
//!     pub async fn attempt_reconnect(
//!         &mut self,
//!         adapter: &mut dyn AudioDeviceAdapter,
//!     ) -> Result<ReconnectionResult> {
//!         // Implementation placeholder (Phase 1)
//!         todo!("Implement in Phase 1: P13-PREP-001")
//!     }
//!
//!     /// Reset reconnection state
//!     ///
//!     /// Called after successful reconnection or manual intervention.
//!     pub fn reset(&mut self) {
//!         self.attempt_count = 0;
//!         self.last_device_id = None;
//!         self.is_reconnecting = false;
//!     }
//!
//!     /// Check if currently in reconnection process
//!     pub fn is_reconnecting(&self) -> bool {
//!         self.is_reconnecting
//!     }
//!
//!     /// Get current attempt count (0-indexed)
//!     pub fn current_attempt(&self) -> u32 {
//!         self.attempt_count
//!     }
//! }
//!
//! impl Default for ReconnectionManager {
//!     fn default() -> Self {
//!         Self::new()
//!     }
//! }
//! ```
//!
//! ## Integration with commands.rs
//!
//! ### Before (commands.rs:193)
//! ```rust
//! // TODO: Implement auto-reconnect (STT-REQ-004.11)
//! {
//!     let state = app.state::<AppState>();
//!     let is_recording = state.is_recording.lock().unwrap();
//!     if *is_recording {
//!         drop(is_recording);
//!         log_warn_details!(/* ... */);
//!         // Note: Actual stop will be triggered by frontend or timeout
//!     }
//! }
//! ```
//!
//! ### After (commands.rs:193)
//! ```rust
//! // Auto-reconnect (STT-REQ-004.11)
//! {
//!     let state = app.state::<AppState>();
//!     let mut reconnection_mgr = state.reconnection_manager.lock().unwrap();
//!
//!     reconnection_mgr.handle_disconnect(device_id.clone());
//!     drop(reconnection_mgr);
//!
//!     // Spawn reconnection task
//!     let app_handle = app.clone();
//!     tokio::spawn(async move {
//!         let state = app_handle.state::<AppState>();
//!         let mut reconnection_mgr = state.reconnection_manager.lock().unwrap();
//!         let mut adapter = state.audio_adapter.lock().unwrap();
//!
//!         match reconnection_mgr.attempt_reconnect(&mut *adapter).await {
//!             Ok(ReconnectionResult::Success { device_id, attempts }) => {
//!                 log_info_details!(/* success log */);
//!                 app_handle.emit_all("device_reconnected", /* ... */).ok();
//!             }
//!             Ok(ReconnectionResult::Failed { device_id, attempts, last_error }) => {
//!                 log_error_details!(/* failure log */);
//!                 app_handle.emit_all("device_reconnect_failed", /* ... */).ok();
//!             }
//!             Err(e) => {
//!                 log_error_details!(/* unexpected error */);
//!             }
//!         }
//!     });
//! }
//! ```
//!
//! ## AppState Changes Required
//!
//! ### Add Field
//! ```rust
//! pub struct AppState {
//!     // ... existing fields ...
//!
//!     /// Reconnection manager for device recovery (STT-REQ-004.11)
//!     pub reconnection_manager: Arc<Mutex<ReconnectionManager>>,
//! }
//! ```
//!
//! ### Initialization
//! ```rust
//! let app_state = AppState {
//!     // ... existing fields ...
//!     reconnection_manager: Arc::new(Mutex::new(ReconnectionManager::new())),
//! };
//! ```
//!
//! ## Test Strategy
//!
//! ### Unit Tests (src/reconnection_manager.rs)
//! ```rust
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     #[test]
//!     fn test_handle_disconnect() {
//!         let mut mgr = ReconnectionManager::new();
//!         mgr.handle_disconnect("device-123".to_string());
//!
//!         assert_eq!(mgr.last_device_id, Some("device-123".to_string()));
//!         assert_eq!(mgr.attempt_count, 0);
//!         assert!(mgr.is_reconnecting());
//!     }
//!
//!     #[test]
//!     fn test_reset() {
//!         let mut mgr = ReconnectionManager::new();
//!         mgr.handle_disconnect("device-123".to_string());
//!         mgr.reset();
//!
//!         assert_eq!(mgr.last_device_id, None);
//!         assert!(!mgr.is_reconnecting());
//!     }
//! }
//! ```
//!
//! ### Integration Tests (tests/reconnection_integration.rs)
//! **Scenario 1**: Successful reconnection on 2nd attempt
//! ```rust
//! #[tokio::test]
//! async fn test_reconnection_success_on_second_attempt() {
//!     let mut mgr = ReconnectionManager::new();
//!     let mut mock_adapter = MockAudioAdapter::new();
//!
//!     // Configure mock: fail 1st, succeed 2nd
//!     mock_adapter.set_reconnect_behavior(vec![
//!         ReconnectBehavior::Fail("Device not ready".to_string()),
//!         ReconnectBehavior::Success,
//!     ]);
//!
//!     mgr.handle_disconnect("device-123".to_string());
//!
//!     let result = mgr.attempt_reconnect(&mut mock_adapter).await.unwrap();
//!
//!     assert_eq!(result, ReconnectionResult::Success {
//!         device_id: "device-123".to_string(),
//!         attempts: 2,
//!     });
//! }
//! ```
//!
//! **Scenario 2**: All retries exhausted
//! ```rust
//! #[tokio::test]
//! async fn test_reconnection_all_retries_exhausted() {
//!     let mut mgr = ReconnectionManager::new();
//!     let mut mock_adapter = MockAudioAdapter::new();
//!
//!     // Configure mock: fail all 3 attempts
//!     mock_adapter.set_reconnect_behavior(vec![
//!         ReconnectBehavior::Fail("Error 1".to_string()),
//!         ReconnectBehavior::Fail("Error 2".to_string()),
//!         ReconnectBehavior::Fail("Error 3".to_string()),
//!     ]);
//!
//!     mgr.handle_disconnect("device-123".to_string());
//!
//!     let result = mgr.attempt_reconnect(&mut mock_adapter).await.unwrap();
//!
//!     assert_eq!(result, ReconnectionResult::Failed {
//!         device_id: "device-123".to_string(),
//!         attempts: 3,
//!         last_error: "Error 3".to_string(),
//!     });
//! }
//! ```
//!
//! **Scenario 3**: AppState Integration (tests/device_reconnect_e2e.rs)
//! ```rust
//! #[tokio::test]
//! async fn test_appstate_reconnection_flow() {
//!     // 1. Initialize AppState with ReconnectionManager
//!     // 2. Start recording
//!     // 3. Inject DeviceGone event
//!     // 4. Verify reconnection task spawned
//!     // 5. Wait for completion (with timeout)
//!     // 6. Verify UI events emitted (device_reconnected or device_reconnect_failed)
//! }
//! ```
//!
//! ## Future Enhancements (Post-MVP1)
//! - Exponential backoff strategy
//! - Device health tracking (consecutive failure count)
//! - User-configurable retry policy
//! - Metrics collection (reconnection success rate)
