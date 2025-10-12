// Unit Tests for PythonSidecarManager Process Lifecycle
// TDD Red Phase: Tests for process startup and management

use meeting_minutes_automator_lib::python_sidecar::{PythonSidecarError, PythonSidecarManager};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn ut_3_2_1_process_startup_success() {
    // Test: Should successfully start Python sidecar process

    let mut manager = PythonSidecarManager::new();

    let result = manager.start().await;

    match &result {
        Ok(_) => println!("✅ Process started successfully"),
        Err(e) => println!("❌ Process startup failed: {:?}", e),
    }

    assert!(
        result.is_ok(),
        "Should start Python sidecar process successfully"
    );

    // Cleanup
    let _ = manager.stop().await;
}

#[tokio::test]
async fn ut_3_2_2_wait_for_ready_success() {
    // Test: Should receive "ready" message within timeout

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");

    // Wait for ready signal with 10 second timeout
    let result = timeout(Duration::from_secs(10), async {
        // For Walking Skeleton, we'll implement a simple ready check
        manager.wait_for_ready().await
    })
    .await;

    match &result {
        Ok(Ok(_)) => println!("✅ Received ready signal"),
        Ok(Err(e)) => println!("❌ Ready check failed: {:?}", e),
        Err(_) => println!("❌ Timeout waiting for ready"),
    }

    assert!(result.is_ok(), "Should not timeout");
    assert!(result.unwrap().is_ok(), "Should receive ready signal");

    // Cleanup
    let _ = manager.stop().await;
}

#[tokio::test]
async fn ut_3_2_3_process_handle_management() {
    // Test: Should maintain valid process handle after startup

    let mut manager = PythonSidecarManager::new();

    // Before start: no process
    assert!(!manager.is_running(), "Should not be running before start");

    // After start: process running
    manager.start().await.expect("Process should start");
    assert!(manager.is_running(), "Should be running after start");

    // Cleanup
    manager.stop().await.expect("Should stop successfully");
    assert!(!manager.is_running(), "Should not be running after stop");
}

#[tokio::test]
async fn ut_3_2_4_stdin_stdout_streams() {
    // Test: Should establish stdin/stdout streams after startup

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");

    // For Walking Skeleton, we verify streams are available
    // by attempting to send a simple ping message
    let result = manager
        .send_message(serde_json::json!({
            "type": "ping",
            "id": "test-1"
        }))
        .await;

    match &result {
        Ok(_) => println!("✅ stdin stream is writable"),
        Err(e) => println!("❌ stdin write failed: {:?}", e),
    }

    assert!(result.is_ok(), "Should be able to write to stdin");

    // Cleanup
    let _ = manager.stop().await;
}

#[tokio::test]
async fn ut_3_2_5_error_python_not_found() {
    // Test: Should return appropriate error when Python is not found

    // Save original APP_PYTHON value
    let original_app_python = std::env::var("APP_PYTHON").ok();

    // This test is difficult to implement without modifying PATH
    // We'll test it by checking the error type when APP_PYTHON points to invalid path
    std::env::set_var("APP_PYTHON", "/nonexistent/python");

    let mut manager = PythonSidecarManager::new();
    let result = manager.start().await;

    // Restore original APP_PYTHON value
    if let Some(val) = original_app_python {
        std::env::set_var("APP_PYTHON", val);
    } else {
        std::env::remove_var("APP_PYTHON");
    }

    match &result {
        Err(PythonSidecarError::DetectionFailed(_)) => {
            println!("✅ Correct error type for Python not found");
        }
        Err(e) => {
            println!("❌ Unexpected error type: {:?}", e);
            panic!("Expected DetectionFailed error");
        }
        Ok(_) => {
            panic!("Should have failed with Python not found error");
        }
    }
}

#[tokio::test]
async fn ut_3_2_6_double_start_prevention() {
    // Test: Should prevent starting when already running

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("First start should succeed");

    let result = manager.start().await;

    match &result {
        Err(PythonSidecarError::ProcessNotRunning) => {
            // This error name is counterintuitive, but we'll check actual implementation
            println!("⚠️  Error type may need adjustment");
        }
        Err(e) => {
            println!("✅ Prevented double start with error: {:?}", e);
        }
        Ok(_) => {
            panic!("Should not allow starting when already running");
        }
    }

    // Cleanup
    let _ = manager.stop().await;
}
