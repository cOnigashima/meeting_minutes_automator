//! STT Burn-in Harness (ADR-013)
//!
//! Long-running stability test for the complete STT pipeline.
//!
//! # Purpose
//! Validates end-to-end stability by:
//! - Generating synthetic audio frames at 100 fps (10ms interval)
//! - Sending frames through ring buffer and IPC to Python sidecar
//! - Monitoring Python STT processing and IPC responses
//! - Detecting memory leaks and resource exhaustion
//!
//! # Known Limitations
//! **Rust Process Monitoring**:
//! The monitoring script (`long_running_monitor.py`) only tracks Python sidecar memory,
//! not the Rust burn-in process itself.
//!
//! - **Root Cause**: Script searches for process name "meeting-minutes-automator" but burn-in binary is "stt_burn_in"
//! - **Impact**: Rust memory metrics show 0MB in test results (Python metrics are valid)
//! - **Workaround**: Manual process inspection during test runs
//! - **Future**: Update monitoring script to detect burn-in binary name (MVP2)
//!
//! # Usage
//! ```bash
//! cargo run --bin stt_burn_in -- --duration-secs 7200  # 2-hour stability test
//! cargo run --bin stt_burn_in -- --duration-secs 60    # Quick validation
//! ```

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;
use meeting_minutes_automator_lib::ring_buffer::{
    new_shared_ring_buffer, pop_audio, push_audio_drop_oldest, BUFFER_CAPACITY, BYTES_PER_SAMPLE,
    SAMPLE_RATE,
};
use meeting_minutes_automator_lib::sidecar::{Event, Sidecar, SidecarCmd, SidecarError};
use ringbuf::traits::Observer;
use std::env;
use std::f32::consts::TAU;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tokio::time::{sleep, timeout};

#[derive(Clone)]
struct LogSink {
    file: Arc<Mutex<Option<File>>>,
}

impl LogSink {
    fn new(path: Option<PathBuf>) -> Result<Self> {
        let file = if let Some(path) = path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create log directory {}", parent.display())
                })?;
            }
            Some(
                File::create(&path)
                    .with_context(|| format!("Failed to create log file {}", path.display()))?,
            )
        } else {
            None
        };

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    fn log(&self, message: &str) {
        println!("{}", message);
        if let Ok(mut guard) = self.file.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = writeln!(file, "{}", message);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EventStats {
    ready_observed: bool,
    partial_text: u64,
    final_text: u64,
    no_speech: u64,
    error_events: u64,
}

#[derive(Debug, Default)]
struct FrameStats {
    frames_generated: u64,
    frames_sent: u64,
    try_send_full: u64,
    ring_overflows: u64,
    max_occupancy: f32,
}

#[derive(Debug)]
struct BurnInConfig {
    duration_secs: u64,
    frame_interval_ms: u64,
    log_interval_secs: u64,
    python_path: Option<PathBuf>,
    log_file: Option<PathBuf>,
}

impl BurnInConfig {
    fn parse() -> Result<Self> {
        let mut duration_secs = 1800; // 30 minutes default burn-in
        let mut frame_interval_ms = 10;
        let mut log_interval_secs = 60;
        let mut python_path: Option<PathBuf> = None;
        let mut log_file: Option<PathBuf> = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--duration-secs" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow!("--duration-secs requires a value"))?;
                    duration_secs = value
                        .parse::<u64>()
                        .context("Failed to parse --duration-secs")?;
                    if duration_secs == 0 {
                        return Err(anyhow!("--duration-secs must be greater than zero"));
                    }
                }
                "--frame-interval-ms" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow!("--frame-interval-ms requires a value"))?;
                    frame_interval_ms = value
                        .parse::<u64>()
                        .context("Failed to parse --frame-interval-ms")?;
                    if frame_interval_ms == 0 || 1000 % frame_interval_ms != 0 {
                        return Err(anyhow!(
                            "--frame-interval-ms must be a divisor of 1000 (e.g. 10, 20, 25, 40)"
                        ));
                    }
                }
                "--log-interval-secs" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow!("--log-interval-secs requires a value"))?;
                    log_interval_secs = value
                        .parse::<u64>()
                        .context("Failed to parse --log-interval-secs")?;
                    if log_interval_secs == 0 {
                        return Err(anyhow!("--log-interval-secs must be greater than zero"));
                    }
                }
                "--python" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow!("--python requires a value"))?;
                    python_path = Some(PathBuf::from(value));
                }
                "--log-file" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow!("--log-file requires a value"))?;
                    log_file = Some(PathBuf::from(value));
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => {
                    return Err(anyhow!("Unknown argument: {}", other));
                }
            }
        }

        Ok(Self {
            duration_secs,
            frame_interval_ms,
            log_interval_secs,
            python_path,
            log_file,
        })
    }
}

fn print_usage() {
    println!(
        r#"stt_burn_in - Long-run IPC/VAD burn-in harness (ADR-013)

USAGE:
    cargo run --bin stt_burn_in -- [OPTIONS]

OPTIONS:
    --duration-secs <seconds>      Total test duration (default: 1800)
    --frame-interval-ms <ms>       Frame interval in milliseconds (default: 10)
    --log-interval-secs <seconds>  Progress log interval (default: 60)
    --python <path>                Override Python executable
    --log-file <path>              Override log file path (default: logs/platform/<epoch>-burnin.log)
    --help                         Show this message
"#
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have a parent")
}

fn default_log_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    workspace_root().join(format!("logs/platform/{}-burnin.log", ts))
}

fn python_main_path() -> PathBuf {
    workspace_root().join("python-stt/main.py")
}

fn generate_frame(frame_index: u64, samples_per_frame: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(samples_per_frame * BYTES_PER_SAMPLE);
    let sample_rate = SAMPLE_RATE as f32;
    let frames_per_second = (SAMPLE_RATE as u64) / (samples_per_frame as u64);
    let frames_per_cycle = frames_per_second * 2; // 1s speech + 1s silence
    let in_speech = (frame_index % frames_per_cycle) < frames_per_second;
    for i in 0..samples_per_frame {
        let sample = if in_speech {
            let absolute_sample = frame_index as f32 * samples_per_frame as f32 + i as f32;
            let t = absolute_sample / sample_rate;
            (0.9_f32 * (TAU * 440.0 * t).sin() * i16::MAX as f32) as i16
        } else {
            0
        };
        buf.extend_from_slice(&sample.to_le_bytes());
    }
    buf
}

async fn wait_for_ready(
    events: &mut meeting_minutes_automator_lib::sidecar::EventStream,
    logger: &LogSink,
    timeout_secs: u64,
) -> Result<()> {
    logger.log("Waiting for Python sidecar ready event…");
    timeout(Duration::from_secs(timeout_secs), async {
        loop {
            match events.recv().await {
                Ok(Event::Ready) => {
                    logger.log("Sidecar reported ready");
                    return Ok(());
                }
                Ok(other) => {
                    logger.log(&format!("Received early event before ready: {:?}", other));
                }
                Err(err) => {
                    return Err(anyhow!("Event stream error before ready: {:?}", err));
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow!("Timed out waiting for ready event"))?
}

async fn monitor_events(
    mut events: meeting_minutes_automator_lib::sidecar::EventStream,
    stats: Arc<AsyncMutex<EventStats>>,
    mut shutdown_rx: oneshot::Receiver<()>,
    logger: LogSink,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                logger.log("Event monitor shutting down");
                break;
            }
            event = events.recv() => {
                match event {
                    Ok(Event::Stream { event_type, data }) => {
                        match event_type.as_str() {
                            "partial_text" => {
                                if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                                    logger.log(&format!("[event] partial_text: {}", text));
                                } else {
                                    logger.log("[event] partial_text (no text)");
                                }
                                let mut guard = stats.lock().await;
                                guard.partial_text += 1;
                            }
                            "final_text" => {
                                if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                                    logger.log(&format!("[event] final_text: {}", text));
                                } else {
                                    logger.log("[event] final_text (no text)");
                                }
                                let mut guard = stats.lock().await;
                                guard.final_text += 1;
                            }
                            "no_speech" => {
                                logger.log("[event] no_speech");
                                let mut guard = stats.lock().await;
                                guard.no_speech += 1;
                            }
                            other => {
                                logger.log(&format!("[event] {} (raw): {}", other, data));
                            }
                        }
                    }
                    Ok(Event::PartialText { text }) => {
                        logger.log(&format!("[event] partial_text: {}", text));
                        let mut guard = stats.lock().await;
                        guard.partial_text += 1;
                    }
                    Ok(Event::FinalText { text }) => {
                        logger.log(&format!("[event] final_text: {}", text));
                        let mut guard = stats.lock().await;
                        guard.final_text += 1;
                    }
                    Ok(Event::NoSpeech) => {
                        logger.log("[event] no_speech");
                        let mut guard = stats.lock().await;
                        guard.no_speech += 1;
                    }
                    Ok(Event::Error { message }) => {
                        logger.log(&format!("[event] error: {}", message));
                        let mut guard = stats.lock().await;
                        guard.error_events += 1;
                    }
                    Ok(Event::Ready) => {
                        logger.log("[event] ready (duplicate)");
                        let mut guard = stats.lock().await;
                        guard.ready_observed = true;
                    }
                    Ok(Event::Unknown) => {
                        logger.log("[event] unknown (ignored)");
                    }
                    Err(SidecarError::ChannelClosed) => {
                        logger.log("Event stream closed");
                        break;
                    }
                    Err(err) => {
                        logger.log(&format!("Event stream error: {:?}", err));
                        break;
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = BurnInConfig::parse()?;
    let log_path = config.log_file.clone().unwrap_or_else(default_log_path);
    let logger = LogSink::new(Some(log_path.clone()))?;

    logger.log("=== STT Burn-in Harness (ADR-013) ===");
    logger.log(&format!("Duration: {} seconds", config.duration_secs));
    logger.log(&format!(
        "Frame interval: {} ms ({} frames/sec)",
        config.frame_interval_ms,
        1000 / config.frame_interval_ms
    ));
    logger.log(&format!(
        "Progress log interval: {} seconds",
        config.log_interval_secs
    ));
    logger.log(&format!("Log file: {}", log_path.display()));

    let python_path = if let Some(ref override_path) = config.python_path {
        override_path.clone()
    } else {
        PythonSidecarManager::detect_python_executable()
            .await
            .context("Python detection failed")?
    };
    logger.log(&format!(
        "Using Python executable: {}",
        python_path.display()
    ));

    let python_script = python_main_path();
    if !python_script.exists() {
        return Err(anyhow!(
            "Python sidecar script not found at {}",
            python_script.display()
        ));
    }
    logger.log(&format!(
        "Python sidecar entrypoint: {}",
        python_script.display()
    ));

    let sidecar = Sidecar::spawn(
        &SidecarCmd::new(
            python_path
                .to_str()
                .ok_or_else(|| anyhow!("Python path contains invalid UTF-8"))?,
        )
        .arg(
            python_script
                .to_str()
                .ok_or_else(|| anyhow!("Python script path contains invalid UTF-8"))?,
        ),
    )
    .await
    .context("Failed to spawn sidecar")?;

    let mut events_stream = sidecar.subscribe();
    wait_for_ready(&mut events_stream, &logger, 30).await?;

    let event_stats = Arc::new(AsyncMutex::new(EventStats {
        ready_observed: true,
        ..Default::default()
    }));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let event_logger = logger.clone();
    let event_stats_task = event_stats.clone();
    let event_handle = tokio::spawn(monitor_events(
        events_stream,
        event_stats_task,
        shutdown_rx,
        event_logger,
    ));

    let frames_per_second = (1000 / config.frame_interval_ms) as usize;
    let samples_per_frame = SAMPLE_RATE / frames_per_second;
    let total_frames = (config.duration_secs * 1000) / config.frame_interval_ms;
    let mut frame_stats = FrameStats::default();
    let mut last_log = Instant::now();
    let ring_buffer = new_shared_ring_buffer();
    let mut stop_due_to_overflow = false;
    let start_time = Instant::now();

    logger.log(&format!(
        "Starting burn-in loop for {} frames (~{} seconds)",
        total_frames, config.duration_secs
    ));

    for frame_index in 0..total_frames {
        let frame = generate_frame(frame_index, samples_per_frame);

        let (pushed, dropped, _level) = {
            let mut rb = ring_buffer.lock().unwrap();
            push_audio_drop_oldest(&mut rb, &frame)
        };
        if dropped > 0 {
            frame_stats.ring_overflows += 1;
            // With drop-oldest, we continue (old data dropped for new)
        }

        frame_stats.frames_generated += 1;
        let occupancy = {
            let rb = ring_buffer.lock().unwrap();
            rb.occupied_len() as f32 / BUFFER_CAPACITY as f32
        };
        frame_stats.max_occupancy = frame_stats.max_occupancy.max(occupancy);

        let mut transfer = vec![0u8; pushed];
        let read = {
            let mut rb = ring_buffer.lock().unwrap();
            pop_audio(&mut rb, &mut transfer)
        };
        if read > 0 {
            frame_stats.frames_sent += 1;
            let bytes = Bytes::from(transfer);
            match sidecar.sink.try_send_frame(bytes.clone()) {
                Ok(_) => {}
                Err(_) => {
                    frame_stats.try_send_full += 1;
                    if let Err(e) = sidecar.sink.send_frame(bytes).await {
                        return Err(anyhow!("Failed to send frame: {:?}", e));
                    }
                }
            }
        }

        if last_log.elapsed() >= Duration::from_secs(config.log_interval_secs) {
            logger.log(&format!(
                "[progress] t={:?}, frames_sent={}, occupancy={:.2}%",
                start_time.elapsed(),
                frame_stats.frames_sent,
                occupancy * 100.0
            ));
            last_log = Instant::now();
        }

        sleep(Duration::from_millis(config.frame_interval_ms)).await;
    }

    // Flush remaining frames if any
    loop {
        let remaining = {
            let rb = ring_buffer.lock().unwrap();
            rb.occupied_len()
        };
        if remaining == 0 {
            break;
        }
        let mut transfer = vec![0u8; remaining];
        let read = {
            let mut rb = ring_buffer.lock().unwrap();
            pop_audio(&mut rb, &mut transfer)
        };
        if read == 0 {
            break;
        }
        frame_stats.frames_sent += 1;
        let bytes = Bytes::from(transfer);
        if let Err(e) = sidecar.sink.send_frame(bytes).await {
            logger.log(&format!(
                "Failed to flush remaining buffer to sidecar: {:?}",
                e
            ));
            break;
        }
    }

    let elapsed = start_time.elapsed();
    logger.log(&format!(
        "Burn-in loop completed in {:.2?} (stop_due_to_overflow={})",
        elapsed, stop_due_to_overflow
    ));

    let _ = shutdown_tx.send(());
    let _ = event_handle.await;

    logger.log("Terminating Python sidecar…");
    if let Err(e) = sidecar.shutdown().await {
        logger.log(&format!("Failed to shutdown sidecar: {:?}", e));
    }

    let event_report = event_stats.lock().await.clone();

    logger.log("=== Burn-in Summary ===");
    logger.log(&format!(
        "Frames generated: {} (sent: {}, try_send_full: {}, ring_overflow: {})",
        frame_stats.frames_generated,
        frame_stats.frames_sent,
        frame_stats.try_send_full,
        frame_stats.ring_overflows
    ));
    logger.log(&format!(
        "Max ring buffer occupancy: {:.2}%",
        frame_stats.max_occupancy * 100.0
    ));
    logger.log(&format!(
        "Events – ready: {}, partial: {}, final: {}, no_speech: {}, error: {}",
        event_report.ready_observed,
        event_report.partial_text,
        event_report.final_text,
        event_report.no_speech,
        event_report.error_events
    ));
    logger.log(&format!("Log saved to {}", log_path.display()));

    if stop_due_to_overflow || frame_stats.try_send_full > 0 || event_report.error_events > 0 {
        logger.log("Result: ⚠️  Issues detected during burn-in");
    } else {
        logger.log("Result: ✅ Burn-in completed without detected issues");
    }

    Ok(())
}
