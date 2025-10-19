// E2E Test Helpers (BLOCK-007)
// Provides helper functions for stt_e2e_test.rs

use serde_json::Value;
use std::path::Path;

/// Verify partial/final text distribution in IPC events
///
/// Validates that:
/// - Partial text events have `isPartial: true` (or `is_final: false`)
/// - Final text events have `isPartial: false` (or `is_final: true`)
///
/// # Arguments
/// * `events` - Vector of IPC event JSON objects
///
/// # Returns
/// * `Result<(), String>` - Ok if distribution is correct, Err with details otherwise
pub fn verify_partial_final_text_distribution(events: &[Value]) -> Result<(), String> {
    let mut partial_count = 0;
    let mut final_count = 0;
    let mut errors = Vec::new();

    for (i, event) in events.iter().enumerate() {
        let event_type = event
            .get("eventType")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "partial_text" => {
                // Check is_final field in data
                let is_final = event
                    .get("data")
                    .and_then(|d| d.get("is_final"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true); // Default true for error detection

                if is_final {
                    errors.push(format!(
                        "Event {}: partial_text should have is_final=false, got true",
                        i
                    ));
                } else {
                    partial_count += 1;
                }
            }
            "final_text" => {
                let is_final = event
                    .get("data")
                    .and_then(|d| d.get("is_final"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false); // Default false for error detection

                if !is_final {
                    errors.push(format!(
                        "Event {}: final_text should have is_final=true, got false",
                        i
                    ));
                } else {
                    final_count += 1;
                }
            }
            _ => {
                // Ignore other event types (speech_start, speech_end, no_speech, etc.)
            }
        }
    }

    if !errors.is_empty() {
        return Err(format!(
            "Partial/final distribution errors:\n  {}",
            errors.join("\n  ")
        ));
    }

    // At least some partial or final text should exist
    if partial_count == 0 && final_count == 0 {
        return Err("No partial_text or final_text events found".to_string());
    }

    Ok(())
}

/// Verify local storage session files
///
/// Validates that the session directory contains:
/// - audio.wav (non-empty WAV file)
/// - transcription.jsonl (JSON Lines format)
/// - session.json (valid JSON with metadata)
///
/// # Arguments
/// * `session_dir` - Path to session directory
///
/// # Returns
/// * `Result<(), String>` - Ok if all files are valid, Err with details otherwise
pub fn verify_local_storage_session(session_dir: &Path) -> Result<(), String> {
    // Check session directory exists
    if !session_dir.exists() {
        return Err(format!(
            "Session directory does not exist: {}",
            session_dir.display()
        ));
    }

    let audio_path = session_dir.join("audio.wav");
    let transcription_path = session_dir.join("transcription.jsonl");
    let session_path = session_dir.join("session.json");

    // Check audio.wav
    if !audio_path.exists() {
        return Err(format!("audio.wav not found: {}", audio_path.display()));
    }

    let audio_metadata = std::fs::metadata(&audio_path)
        .map_err(|e| format!("Failed to read audio.wav metadata: {}", e))?;

    if audio_metadata.len() < 44 {
        return Err(format!(
            "audio.wav is too small ({} bytes, expected at least 44-byte WAV header)",
            audio_metadata.len()
        ));
    }

    // Check transcription.jsonl
    if !transcription_path.exists() {
        return Err(format!(
            "transcription.jsonl not found: {}",
            transcription_path.display()
        ));
    }

    let transcription_content = std::fs::read_to_string(&transcription_path)
        .map_err(|e| format!("Failed to read transcription.jsonl: {}", e))?;

    // Validate JSON Lines format
    let lines: Vec<&str> = transcription_content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue; // Allow empty lines
        }

        serde_json::from_str::<Value>(line).map_err(|e| {
            format!(
                "transcription.jsonl line {} is not valid JSON: {} (line: {})",
                i + 1,
                e,
                line
            )
        })?;
    }

    // Check session.json
    if !session_path.exists() {
        return Err(format!(
            "session.json not found: {}",
            session_path.display()
        ));
    }

    let session_content = std::fs::read_to_string(&session_path)
        .map_err(|e| format!("Failed to read session.json: {}", e))?;

    let session_json: Value = serde_json::from_str(&session_content)
        .map_err(|e| format!("session.json is not valid JSON: {}", e))?;

    // Validate required fields in session.json
    let required_fields = [
        "session_id",
        "start_time",
        "end_time",
        "duration_seconds",
        "audio_device",
        "model_size",
    ];

    for field in &required_fields {
        if session_json.get(field).is_none() {
            return Err(format!("session.json missing required field: {}", field));
        }
    }

    Ok(())
}

/// Measure partial text latency
///
/// Calculates the time between speech_start event and first partial_text event.
///
/// # Arguments
/// * `events` - Vector of IPC event JSON objects with timestamps
///
/// # Returns
/// * `Result<f64, String>` - Latency in seconds, or Err if events not found
pub fn measure_partial_text_latency(events: &[Value]) -> Result<f64, String> {
    let mut speech_start_time: Option<u64> = None;
    let mut first_partial_time: Option<u64> = None;

    for event in events {
        let event_type = event
            .get("eventType")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let timestamp = event
            .get("data")
            .and_then(|d| d.get("timestamp"))
            .or_else(|| event.get("timestamp"))
            .and_then(|v| v.as_u64());

        match event_type {
            "speech_start" if speech_start_time.is_none() => {
                speech_start_time = timestamp;
            }
            "partial_text" if first_partial_time.is_none() && speech_start_time.is_some() => {
                first_partial_time = timestamp;
            }
            _ => {}
        }

        if speech_start_time.is_some() && first_partial_time.is_some() {
            break;
        }
    }

    match (speech_start_time, first_partial_time) {
        (Some(start), Some(partial)) => {
            let latency_ms = partial.saturating_sub(start) as f64;
            Ok(latency_ms / 1000.0) // Convert to seconds
        }
        (None, _) => Err("No speech_start event found".to_string()),
        (_, None) => Err("No partial_text event found after speech_start".to_string()),
    }
}

/// Measure final text latency
///
/// Calculates the time between speech_end event and final_text event.
///
/// # Arguments
/// * `events` - Vector of IPC event JSON objects with timestamps
///
/// # Returns
/// * `Result<f64, String>` - Latency in seconds, or Err if events not found
pub fn measure_final_text_latency(events: &[Value]) -> Result<f64, String> {
    let mut speech_end_time: Option<u64> = None;
    let mut final_text_time: Option<u64> = None;

    for event in events {
        let event_type = event
            .get("eventType")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let timestamp = event
            .get("data")
            .and_then(|d| d.get("timestamp"))
            .or_else(|| event.get("timestamp"))
            .and_then(|v| v.as_u64());

        match event_type {
            "speech_end" if speech_end_time.is_none() => {
                speech_end_time = timestamp;
            }
            "final_text" if final_text_time.is_none() && speech_end_time.is_some() => {
                final_text_time = timestamp;
            }
            _ => {}
        }

        if speech_end_time.is_some() && final_text_time.is_some() {
            break;
        }
    }

    match (speech_end_time, final_text_time) {
        (Some(end), Some(final_t)) => {
            let latency_ms = final_t.saturating_sub(end) as f64;
            Ok(latency_ms / 1000.0) // Convert to seconds
        }
        (None, _) => Err("No speech_end event found".to_string()),
        (_, None) => Err("No final_text event found after speech_end".to_string()),
    }
}

/// Monitor memory usage during long-running test
///
/// Monitors memory usage at regular intervals and verifies it stays below threshold.
/// Designed for 2-hour continuous recording stress tests (Task 10.7, STT-NFR-002.1).
///
/// # Arguments
/// * `duration_seconds` - How long to monitor (e.g., 7200 for 2 hours)
/// * `sample_interval_seconds` - How often to sample memory (e.g., 60 for every minute)
/// * `max_memory_gb` - Maximum allowed memory in GB (e.g., 2.0)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - Vector of memory samples in GB, or Err if threshold exceeded
///
/// # Example
/// ```no_run
/// // Monitor for 1 hour, sample every 5 minutes, max 2GB
/// let samples = monitor_memory_usage_long_running(3600, 300, 2.0)?;
/// assert!(samples.iter().all(|&mem| mem < 2.0));
/// ```
pub async fn monitor_memory_usage_long_running(
    duration_seconds: u64,
    sample_interval_seconds: u64,
    max_memory_gb: f64,
) -> Result<Vec<f64>, String> {
    use std::process::Command;
    use tokio::time::{sleep, Duration};

    let mut samples = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed().as_secs() < duration_seconds {
        // Get memory usage (RSS) of current process
        let pid = std::process::id();

        // Platform-specific memory measurement
        #[cfg(target_os = "macos")]
        let mem_gb = get_memory_usage_macos(pid)?;

        #[cfg(target_os = "linux")]
        let mem_gb = get_memory_usage_linux(pid)?;

        #[cfg(target_os = "windows")]
        let mem_gb = get_memory_usage_windows(pid)?;

        samples.push(mem_gb);

        // Check threshold
        if mem_gb > max_memory_gb {
            return Err(format!(
                "Memory usage exceeded threshold: {:.2}GB > {:.2}GB (after {} samples)",
                mem_gb,
                max_memory_gb,
                samples.len()
            ));
        }

        sleep(Duration::from_secs(sample_interval_seconds)).await;
    }

    Ok(samples)
}

#[cfg(target_os = "macos")]
fn get_memory_usage_macos(pid: u32) -> Result<f64, String> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "rss="])
        .output()
        .map_err(|e| format!("Failed to run ps command: {}", e))?;

    if !output.status.success() {
        return Err(format!("ps command failed: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rss_kb: f64 = stdout
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse RSS value '{}': {}", stdout.trim(), e))?;

    Ok(rss_kb / (1024.0 * 1024.0)) // Convert KB to GB
}

#[cfg(target_os = "linux")]
fn get_memory_usage_linux(pid: u32) -> Result<f64, String> {
    use std::fs;

    let status_path = format!("/proc/{}/status", pid);
    let content = fs::read_to_string(&status_path)
        .map_err(|e| format!("Failed to read {}: {}", status_path, e))?;

    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let rss_kb: f64 = parts[1]
                    .parse()
                    .map_err(|e| format!("Failed to parse RSS value: {}", e))?;
                return Ok(rss_kb / (1024.0 * 1024.0)); // Convert KB to GB
            }
        }
    }

    Err(format!("VmRSS not found in {}", status_path))
}

#[cfg(target_os = "windows")]
fn get_memory_usage_windows(pid: u32) -> Result<f64, String> {
    use std::process::Command;

    let output = Command::new("tasklist")
        .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("Failed to run tasklist command: {}", e))?;

    if !output.status.success() {
        return Err(format!("tasklist command failed: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = stdout.split(',').collect();

    if fields.len() < 5 {
        return Err("Unexpected tasklist output format".to_string());
    }

    // Memory is in "1,234 K" format, need to clean it
    let mem_str = fields[4]
        .trim()
        .trim_matches('"')
        .replace(" K", "")
        .replace(",", "");
    let mem_kb: f64 = mem_str
        .parse()
        .map_err(|e| format!("Failed to parse memory value '{}': {}", mem_str, e))?;

    Ok(mem_kb / (1024.0 * 1024.0)) // Convert KB to GB
}

/// Monitor CPU usage during recording
///
/// Monitors CPU usage at regular intervals and verifies it stays below threshold.
/// Designed for recording performance tests (Task 10.7, STT-NFR-002.2).
///
/// # Arguments
/// * `duration_seconds` - How long to monitor (e.g., 300 for 5 minutes)
/// * `sample_interval_seconds` - How often to sample CPU (e.g., 5 for every 5 seconds)
/// * `max_cpu_percent` - Maximum allowed CPU percentage (e.g., 50.0 for 50%)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - Vector of CPU samples in %, or Err if threshold exceeded
///
/// # Example
/// ```no_run
/// // Monitor for 5 minutes, sample every 5 seconds, max 50% CPU
/// let samples = monitor_cpu_usage_during_recording(300, 5, 50.0)?;
/// assert!(samples.iter().all(|&cpu| cpu < 50.0));
/// ```
pub async fn monitor_cpu_usage_during_recording(
    duration_seconds: u64,
    sample_interval_seconds: u64,
    max_cpu_percent: f64,
) -> Result<Vec<f64>, String> {
    use tokio::time::{sleep, Duration};

    let mut samples = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed().as_secs() < duration_seconds {
        // Get CPU usage of current process
        let pid = std::process::id();

        // Platform-specific CPU measurement
        #[cfg(target_os = "macos")]
        let cpu_percent = get_cpu_usage_macos(pid)?;

        #[cfg(target_os = "linux")]
        let cpu_percent = get_cpu_usage_linux(pid)?;

        #[cfg(target_os = "windows")]
        let cpu_percent = get_cpu_usage_windows(pid)?;

        samples.push(cpu_percent);

        // Check threshold
        if cpu_percent > max_cpu_percent {
            return Err(format!(
                "CPU usage exceeded threshold: {:.1}% > {:.1}% (after {} samples)",
                cpu_percent,
                max_cpu_percent,
                samples.len()
            ));
        }

        sleep(Duration::from_secs(sample_interval_seconds)).await;
    }

    Ok(samples)
}

#[cfg(target_os = "macos")]
fn get_cpu_usage_macos(pid: u32) -> Result<f64, String> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "%cpu="])
        .output()
        .map_err(|e| format!("Failed to run ps command: {}", e))?;

    if !output.status.success() {
        return Err(format!("ps command failed: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cpu_percent: f64 = stdout
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse CPU value '{}': {}", stdout.trim(), e))?;

    Ok(cpu_percent)
}

#[cfg(target_os = "linux")]
fn get_cpu_usage_linux(pid: u32) -> Result<f64, String> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "%cpu="])
        .output()
        .map_err(|e| format!("Failed to run ps command: {}", e))?;

    if !output.status.success() {
        return Err(format!("ps command failed: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cpu_percent: f64 = stdout
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse CPU value '{}': {}", stdout.trim(), e))?;

    Ok(cpu_percent)
}

#[cfg(target_os = "windows")]
fn get_cpu_usage_windows(_pid: u32) -> Result<f64, String> {
    // Windows CPU monitoring is complex (requires performance counters)
    // For E2E tests, we'll return a conservative estimate
    // TODO: Implement proper Windows CPU monitoring using Performance Counters
    Err("CPU monitoring not yet implemented for Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_verify_partial_final_distribution_valid() {
        let events = vec![
            json!({
                "eventType": "speech_start",
                "data": {}
            }),
            json!({
                "eventType": "partial_text",
                "data": {
                    "text": "Hello",
                    "is_final": false
                }
            }),
            json!({
                "eventType": "final_text",
                "data": {
                    "text": "Hello world",
                    "is_final": true
                }
            }),
        ];

        assert!(verify_partial_final_text_distribution(&events).is_ok());
    }

    #[test]
    fn test_verify_partial_final_distribution_invalid() {
        let events = vec![json!({
            "eventType": "partial_text",
            "data": {
                "text": "Wrong",
                "is_final": true  // Should be false
            }
        })];

        let result = verify_partial_final_text_distribution(&events);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("should have is_final=false"));
    }

    #[test]
    fn test_measure_partial_text_latency() {
        let events = vec![
            json!({
                "eventType": "speech_start",
                "timestamp": 1000
            }),
            json!({
                "eventType": "partial_text",
                "timestamp": 1300,
                "data": {}
            }),
        ];

        let latency = measure_partial_text_latency(&events).unwrap();
        assert_eq!(latency, 0.3); // 300ms = 0.3s
    }

    #[test]
    fn test_measure_final_text_latency() {
        let events = vec![
            json!({
                "eventType": "speech_end",
                "timestamp": 2000
            }),
            json!({
                "eventType": "final_text",
                "timestamp": 3500,
                "data": {}
            }),
        ];

        let latency = measure_final_text_latency(&events).unwrap();
        assert_eq!(latency, 1.5); // 1500ms = 1.5s
    }
}
