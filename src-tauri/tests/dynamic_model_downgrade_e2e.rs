// Task 10.3.3: Dynamic Model Downgrade E2E Tests
// Requirements: STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9
//
// External Review Adjustments (2025-10-20):
// - Removed simulation hooks (security risk)
// - No IPC internal state queries (surface minimization)
// - Added TEST_FIXTURE_MODE for deterministic event emission
// - CRITICAL: Test must ASSERT event receipt, not pass on empty events
//
// Coverage Strategy:
// - Python unit tests: Downgrade trigger logic, debounce, state machine
// - Rust unit tests: WebSocket broadcast schema validation (commands.rs)
// - Rust E2E: IPC event path verification (Python → Rust event reception)
// - Manual: Full Tauri integration with WebSocket → Chrome extension

use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;
use tokio::time::{timeout, Duration};
use serial_test::serial;

/// RAII guard for TEST_FIXTURE_MODE environment variable
/// Ensures cleanup even on panic to prevent test pollution
struct TestFixtureModeGuard {
    previous_value: Option<std::ffi::OsString>,
}

impl TestFixtureModeGuard {
    fn new() -> Self {
        let previous_value = std::env::var_os("TEST_FIXTURE_MODE");
        std::env::set_var("TEST_FIXTURE_MODE", "1");
        Self { previous_value }
    }
}

impl Drop for TestFixtureModeGuard {
    fn drop(&mut self) {
        // Restore previous state or remove if not set
        match &self.previous_value {
            Some(val) => std::env::set_var("TEST_FIXTURE_MODE", val),
            None => std::env::remove_var("TEST_FIXTURE_MODE"),
        }
    }
}

/// Test Scenario: Deterministic model_change event end-to-end verification
///
/// GIVEN: Python sidecar in TEST_FIXTURE_MODE (scripted events, no Whisper loading)
/// WHEN: Sidecar starts
/// THEN: A model_change event is received via IPC with valid schema
///
/// This test addresses External Review critique:
/// "test_model_change_event_ipc_path_verification never asserts that a downgrade
///  event is emitted; it breaks out as soon as events is empty and treats that as
///  success, so the downgrade flow can remain completely broken while the test
///  still passes."
///
/// CRITICAL: This test MUST assert event receipt. Failure = broken IPC path.
///
/// NOTE: #[serial(env_test)] prevents parallel execution with other env-mutating tests
/// to avoid race conditions. Tests in different files run in separate processes, so
/// only same-file tests need serialization.
#[tokio::test]
#[serial(env_test)]
async fn test_model_change_event_end_to_end() {
    println!("==> Task 10.3.3: Model Change Event End-to-End Verification");

    // Step 1: Enable TEST_FIXTURE_MODE with RAII guard (prevents test pollution)
    let _guard = TestFixtureModeGuard::new();
    println!("  ✓ TEST_FIXTURE_MODE enabled (scripted events, auto-cleanup on drop)");

    // Step 2: Start Python sidecar
    let mut sidecar = PythonSidecarManager::new();
    sidecar
        .start()
        .await
        .expect("Python sidecar should start");

    // Step 3: Wait for ready signal (must succeed)
    sidecar
        .wait_for_ready()
        .await
        .expect("Should receive ready signal");

    println!("  ✓ Python sidecar started (ready signal received)");

    // Step 4: Collect IPC events (increased timeout for scripted event emission)
    let mut model_change_received = false;
    let mut received_event: Option<serde_json::Value> = None;

    for i in 0..20 {
        match timeout(Duration::from_millis(500), sidecar.receive_message()).await {
            Ok(Ok(event)) => {
                println!("  [{}] Event received: type={:?}, eventType={:?}",
                    i,
                    event.get("type"),
                    event.get("eventType")
                );

                // Check if this is a model_change event
                if event
                    .get("eventType")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "model_change")
                    .unwrap_or(false)
                {
                    model_change_received = true;
                    received_event = Some(event);
                    break;
                }
            }
            Ok(Err(e)) => {
                println!("  [{}] IPC error: {:?}", i, e);
                break;
            }
            Err(_) => {
                // Timeout - continue polling
                continue;
            }
        }
    }

    // Step 5: CRITICAL ASSERTION - Test MUST fail if no event received
    assert!(
        model_change_received,
        "CRITICAL: model_change event must be received in TEST_FIXTURE_MODE. \
         If this fails, the IPC event path is broken."
    );

    println!("  ✅ model_change event received");

    // Step 6: Verify event schema (STT-REQ-006.9)
    let event = received_event.expect("Event should be captured");
    let data = event
        .get("data")
        .expect("model_change event must have 'data' field");

    let old_model = data.get("old_model").and_then(|v| v.as_str());
    let new_model = data.get("new_model").and_then(|v| v.as_str());
    let reason = data.get("reason").and_then(|v| v.as_str());

    assert!(old_model.is_some(), "old_model field must be present");
    assert!(new_model.is_some(), "new_model field must be present");
    assert!(reason.is_some(), "reason field must be present");

    println!("  ✓ Event schema valid:");
    println!("    old_model: {}", old_model.unwrap());
    println!("    new_model: {}", new_model.unwrap());
    println!("    reason: {}", reason.unwrap());

    // Step 7: Verify scripted event content (TEST_FIXTURE_MODE emits medium→base)
    assert_eq!(
        old_model.unwrap(),
        "medium",
        "TEST_FIXTURE_MODE should emit old_model=medium"
    );
    assert_eq!(
        new_model.unwrap(),
        "base",
        "TEST_FIXTURE_MODE should emit new_model=base"
    );
    assert_eq!(
        reason.unwrap(),
        "cpu_high",
        "TEST_FIXTURE_MODE should emit reason=cpu_high"
    );

    println!("  ✅ Scripted event content validated");

    // Cleanup
    sidecar.shutdown().await.expect("Should shutdown cleanly");
    println!("  ✅ E2E test completed successfully");
    println!();
    println!("Coverage Summary:");
    println!("  - IPC event path: ✅ Verified (Python → Rust event reception)");
    println!("  - Event schema: ✅ Validated (old_model, new_model, reason)");
    println!("  - Downgrade triggers: ✅ Python unit tests (cpu/memory thresholds)");
    println!("  - WebSocket broadcast: ✅ Rust unit tests (commands.rs)");
    println!("  - Tauri integration: ⚠️  Manual verification required");
}

/// Test: Verify TestFixtureModeGuard cleanup (prevents test pollution)
///
/// This test verifies that the RAII guard properly restores environment state,
/// addressing the critical issue identified in external review:
/// "std::env::set_var mutates global state, any later test will keep running
///  the Python sidecar in fixture mode, so sidecar-based tests that expect
///  the real Whisper workflow will silently flip into the stub behaviour."
#[test]
#[serial(env_test)]
fn test_fixture_mode_guard_cleanup() {
    // Verify initial state (should not be set)
    assert!(
        std::env::var("TEST_FIXTURE_MODE").is_err(),
        "TEST_FIXTURE_MODE should not be set initially"
    );

    // Create guard and verify flag is set
    {
        let _guard = TestFixtureModeGuard::new();
        assert_eq!(
            std::env::var("TEST_FIXTURE_MODE").unwrap(),
            "1",
            "Guard should set TEST_FIXTURE_MODE=1"
        );
    } // Guard drops here

    // Verify cleanup (flag should be removed)
    assert!(
        std::env::var("TEST_FIXTURE_MODE").is_err(),
        "TEST_FIXTURE_MODE should be removed after guard drop"
    );
}

/// Test: Verify guard restores previous value (not just removes)
#[test]
#[serial(env_test)]
fn test_fixture_mode_guard_restores_previous_value() {
    // Set initial value
    std::env::set_var("TEST_FIXTURE_MODE", "previous_value");

    // Create guard (should overwrite with "1")
    {
        let _guard = TestFixtureModeGuard::new();
        assert_eq!(
            std::env::var("TEST_FIXTURE_MODE").unwrap(),
            "1",
            "Guard should overwrite with '1'"
        );
    } // Guard drops here

    // Verify previous value is restored
    assert_eq!(
        std::env::var("TEST_FIXTURE_MODE").unwrap(),
        "previous_value",
        "Guard should restore previous value"
    );

    // Cleanup for other tests
    std::env::remove_var("TEST_FIXTURE_MODE");
}

// Test Scenario 2 & 3: Removed per External Review
//
// Downgrade trigger conditions and state machine transitions are tested at:
// - Python unit tests: test_resource_monitor.py::TestDynamicDowngrade (8 tests)
// - Python unit tests: test_resource_monitor.py::TestDebounceLogic (1 test)
// - Python unit tests: test_resource_monitor.py::TestStateMachineTransitions (4 tests)
// - Rust unit tests: commands.rs::tests::test_model_change_event_schema_* (5 tests)
//
// E2E simulation without test hooks would require:
// 1. Actual memory pressure generation (unsafe/unreliable in CI)
// 2. Python sidecar modification for error injection (violates separation of concerns)
//
// Coverage strategy (per External Review recommendations):
// - Unit tests: Logic verification (Python resource_monitor.py, Rust commands.rs)
// - Integration test: IPC path verification (above, with TEST_FIXTURE_MODE)
// - Manual test: Full downgrade flow with real resource pressure + WebSocket broadcast
