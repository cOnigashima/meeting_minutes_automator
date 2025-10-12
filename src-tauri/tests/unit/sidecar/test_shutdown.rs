// Unit Tests for Graceful Shutdown and Cleanup
// TDD Red Phase: Tests for graceful shutdown and Drop trait

use meeting_minutes_automator_lib::python_sidecar::{PythonSidecarError, PythonSidecarManager};
use std::time::Duration;

#[tokio::test]
async fn ut_3_3_1_graceful_shutdown_with_message() {
    // Test: Should send shutdown message and wait for graceful exit

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager
        .wait_for_ready()
        .await
        .expect("Should receive ready");

    // Call shutdown (which sends shutdown message)
    let result = manager.shutdown().await;

    match &result {
        Ok(_) => println!("✅ Graceful shutdown succeeded"),
        Err(e) => println!("❌ Shutdown failed: {:?}", e),
    }

    assert!(result.is_ok(), "Should shutdown gracefully");
    assert!(
        !manager.is_running(),
        "Process should not be running after shutdown"
    );
}

#[tokio::test]
async fn ut_3_3_2_forced_shutdown_on_timeout() {
    // Test: Should force kill process if graceful shutdown times out

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager
        .wait_for_ready()
        .await
        .expect("Should receive ready");

    // For this test, we'll modify the Python script temporarily to ignore shutdown
    // For Walking Skeleton, we'll just verify the timeout mechanism works

    // Close stdin without sending shutdown message to simulate timeout
    manager.force_close_stdin();

    let result = manager.shutdown().await;

    match &result {
        Ok(_) => println!("✅ Forced shutdown succeeded"),
        Err(e) => println!("❌ Forced shutdown failed: {:?}", e),
    }

    // Even with timeout, should succeed (with force kill)
    assert!(result.is_ok(), "Should force shutdown on timeout");
    assert!(
        !manager.is_running(),
        "Process should not be running after force shutdown"
    );
}

#[tokio::test]
async fn ut_3_3_3_drop_trait_cleanup() {
    // Test: Should clean up process when manager is dropped

    let process_id = {
        let mut manager = PythonSidecarManager::new();
        manager.start().await.expect("Process should start");
        manager
            .wait_for_ready()
            .await
            .expect("Should receive ready");

        // Get process ID before dropping
        manager.get_process_id().expect("Should have process ID")
        // manager is dropped here
    };

    // Wait a bit for cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify process is not running
    let is_running = check_process_running(process_id);
    assert!(!is_running, "Process should be cleaned up after drop");
}

#[tokio::test]
async fn ut_3_3_4_no_zombie_processes() {
    // Test: Verify no zombie processes remain after shutdown

    let process_id = {
        let mut manager = PythonSidecarManager::new();
        manager.start().await.expect("Process should start");
        manager
            .wait_for_ready()
            .await
            .expect("Should receive ready");

        let pid = manager.get_process_id().expect("Should have process ID");

        manager.shutdown().await.expect("Should shutdown");
        pid
    };

    // Wait for OS to clean up
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check no zombie process
    let is_zombie = check_process_zombie(process_id);
    assert!(!is_zombie, "Should not create zombie processes");
}

#[tokio::test]
async fn ut_3_3_5_multiple_shutdown_calls() {
    // Test: Should handle multiple shutdown calls gracefully

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager
        .wait_for_ready()
        .await
        .expect("Should receive ready");

    // First shutdown
    let result1 = manager.shutdown().await;
    assert!(result1.is_ok(), "First shutdown should succeed");

    // Second shutdown (should return error but not panic)
    let result2 = manager.shutdown().await;
    match result2 {
        Err(PythonSidecarError::ProcessNotRunning) => {
            println!("✅ Correctly returned ProcessNotRunning error");
        }
        Ok(_) => panic!("Second shutdown should fail with ProcessNotRunning"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

// Helper functions

fn check_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .expect("Failed to execute tasklist");

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains(&pid.to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string()])
            .output()
            .expect("Failed to execute ps");

        output.status.success()
    }
}

fn check_process_zombie(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows doesn't have zombie processes in the Unix sense
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "state="])
            .output()
            .expect("Failed to execute ps");

        if output.status.success() {
            let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
            state == "Z" || state == "Z+"
        } else {
            false
        }
    }
}
