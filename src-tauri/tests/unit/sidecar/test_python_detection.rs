// Unit Tests for Python Interpreter Detection
// TDD Red Phase: Tests that will drive the implementation

use meeting_minutes_automator_lib::python_sidecar::{PythonDetectionError, PythonSidecarManager};
use std::env;

#[tokio::test]
async fn ut_3_1_1_detect_from_env_variable() {
    // Test: Should detect Python from APP_PYTHON environment variable
    // Uses dynamically detected Python instead of hardcoded path

    // Skip if APP_PYTHON is already set
    if env::var("APP_PYTHON").is_err() {
        // Find a valid Python executable dynamically
        let python_path = find_valid_python().await
            .expect("No valid Python found for testing");

        // Set it temporarily for this test
        env::set_var("APP_PYTHON", &python_path);
        let result = PythonSidecarManager::detect_python_executable().await;
        env::remove_var("APP_PYTHON");

        match &result {
            Ok(path) => println!("✅ Detected Python: {:?}", path),
            Err(e) => println!("❌ Detection failed: {:?}", e),
        }

        assert!(result.is_ok(), "Should detect Python from APP_PYTHON env var");
        assert_eq!(result.unwrap().to_string_lossy(), python_path, "Should use APP_PYTHON path");
    } else {
        println!("Skipping - APP_PYTHON already set");
    }
}

/// Helper function to find a valid Python executable for testing
async fn find_valid_python() -> Option<String> {
    use tokio::process::Command;

    // Try common Python commands in order
    #[cfg(target_os = "windows")]
    let candidates = vec!["python3.12", "python3.11", "python3.10", "python3.9", "python3", "python"];

    #[cfg(not(target_os = "windows"))]
    let candidates = vec!["python3.12", "python3.11", "python3.10", "python3.9", "python3"];

    for cmd in candidates {
        // Try to run the command with --version
        if let Ok(output) = Command::new(cmd).arg("--version").output().await {
            if output.status.success() {
                // Find absolute path using which/where
                #[cfg(target_os = "windows")]
                let which_cmd = "where";

                #[cfg(not(target_os = "windows"))]
                let which_cmd = "which";

                if let Ok(path_output) = Command::new(which_cmd).arg(cmd).output().await {
                    if path_output.status.success() {
                        if let Ok(path) = String::from_utf8(path_output.stdout) {
                            return Some(path.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

#[tokio::test]
async fn ut_3_1_2_detect_from_virtual_env() {
    // Test: Should detect Python from VIRTUAL_ENV

    // This test will be refined based on actual venv setup
    // For now, we test the logic exists

    // Skip if no venv is active
    if env::var("VIRTUAL_ENV").is_err() && env::var("CONDA_PREFIX").is_err() {
        println!("Skipping venv test - no active virtual environment");
        return;
    }

    let result = PythonSidecarManager::detect_python_executable().await;
    assert!(result.is_ok(), "Should detect Python from virtual environment");
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn ut_3_1_3_detect_from_py_launcher() {
    // Test: Windows - Should detect Python from py.exe launcher

    let result = PythonSidecarManager::detect_python_executable().await;

    // Should succeed if py.exe exists, or fall back to PATH
    assert!(result.is_ok(), "Should detect Python from py.exe or fallback");
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn ut_3_1_4_detect_from_posix_path() {
    // Test: POSIX - Should detect Python from PATH (python3.12, python3.11, etc.)

    let result = PythonSidecarManager::detect_python_executable().await;

    assert!(result.is_ok(), "Should detect Python from PATH");
}

#[tokio::test]
async fn ut_3_1_5_validate_python_version() {
    // Test: Should validate Python version is in supported range (3.9 <= version < 3.13)

    let result = PythonSidecarManager::detect_python_executable().await;

    match result {
        Ok(path) => {
            // Verify version is in range
            let is_valid = PythonSidecarManager::validate_python_version(&path).await;
            assert!(is_valid.is_ok(), "Detected Python should be in supported version range");
        }
        Err(e) => {
            panic!("Python detection failed: {}", e);
        }
    }
}

#[tokio::test]
async fn ut_3_1_6_error_handling_python_not_found() {
    // Test: Should return ConfiguredPathInvalid error when APP_PYTHON points to non-existent path
    // Note: Testing PythonNotFound in unit tests is difficult due to parallel execution
    // This is better tested in integration tests

    env::set_var("APP_PYTHON", "/nonexistent/python");
    let result = PythonSidecarManager::detect_python_executable().await;
    env::remove_var("APP_PYTHON");

    match result {
        Err(PythonDetectionError::ConfiguredPathInvalid { .. }) => {
            // Expected error
        }
        Ok(_) => {
            panic!("Should have failed with ConfiguredPathInvalid error");
        }
        Err(e) => {
            panic!("Wrong error type: {:?}", e);
        }
    }
}

#[tokio::test]
async fn ut_3_1_7_architecture_validation() {
    // Test: Should validate 64-bit architecture

    let result = PythonSidecarManager::detect_python_executable().await;

    if let Ok(path) = result {
        let arch_result = PythonSidecarManager::validate_architecture(&path).await;
        assert!(arch_result.is_ok(), "Should validate as 64-bit architecture");
    }
}
