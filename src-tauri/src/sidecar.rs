// Sidecar Full-Duplex IPC Facade API
// ADR-013: Sidecar Full-Duplex IPC Final Design
// Phase 1: Facade API Implementation

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

/// Sidecar process errors
#[derive(Error, Debug)]
pub enum SidecarError {
    #[error("Failed to spawn sidecar process: {0}")]
    SpawnFailed(String),

    #[error("Failed to send frame: {0}")]
    SendFailed(String),

    #[error("Failed to receive event: {0}")]
    ReceiveFailed(String),

    #[error("Sidecar process terminated unexpectedly")]
    ProcessTerminated,

    #[error("Channel closed")]
    ChannelClosed,
}

/// Event types emitted by the sidecar process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// Python process ready
    #[serde(rename = "ready")]
    Ready,

    /// Stream event wrapper (partial_text/final_text etc.)
    #[serde(rename = "event")]
    Stream {
        #[serde(rename = "eventType")]
        event_type: String,
        #[serde(default)]
        data: serde_json::Value,
    },

    /// Error from Python side
    #[serde(rename = "error")]
    Error { message: String },

    /// Legacy direct partial_text event (older protocol)
    #[serde(rename = "partial_text")]
    LegacyPartial { text: String },

    /// Legacy direct final_text event
    #[serde(rename = "final_text")]
    LegacyFinal { text: String },

    /// Legacy no_speech event
    #[serde(rename = "no_speech")]
    LegacyNoSpeech,

    /// Unknown event (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Sidecar command configuration
pub struct SidecarCmd {
    pub program: String,
    pub args: Vec<String>,
}

impl SidecarCmd {
    /// Create a new sidecar command
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Spawn the sidecar process
    async fn spawn(&self) -> Result<Child, SidecarError> {
        tokio::process::Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SidecarError::SpawnFailed(e.to_string()))
    }
}

/// Audio sink for sending frames to the sidecar
///
/// This is a facade over mpsc::Sender that hides the internal implementation.
/// The sender is connected to an internal writer task that owns ChildStdin exclusively.
pub struct AudioSink {
    tx: mpsc::Sender<Bytes>,
}

impl AudioSink {
    /// Send an audio frame (non-blocking, bounded channel)
    pub async fn send_frame(&self, frame: Bytes) -> Result<(), SidecarError> {
        self.tx
            .send(frame)
            .await
            .map_err(|_| SidecarError::ChannelClosed)
    }

    /// Try to send an audio frame without blocking
    /// Returns Err if channel is full (backpressure)
    pub fn try_send_frame(&self, frame: Bytes) -> Result<(), SidecarError> {
        self.tx.try_send(frame).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                SidecarError::SendFailed("Channel full (backpressure)".to_string())
            }
            mpsc::error::TrySendError::Closed(_) => SidecarError::ChannelClosed,
        })
    }
}

/// Event stream for receiving events from the sidecar
///
/// This is a facade over broadcast::Receiver that hides the internal implementation.
/// The receiver is connected to an internal reader task that owns ChildStdout exclusively.
pub struct EventStream {
    rx: broadcast::Receiver<Event>,
}

impl EventStream {
    /// Receive the next event (blocking)
    pub async fn recv(&mut self) -> Result<Event, SidecarError> {
        loop {
            match self.rx.recv().await {
                Ok(evt) => return Ok(evt),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("EventStream lagged by {n} events, skipping");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(SidecarError::ChannelClosed)
                }
            }
        }
    }

    /// Try to receive an event without blocking
    pub fn try_recv(&mut self) -> Result<Event, SidecarError> {
        match self.rx.try_recv() {
            Ok(evt) => Ok(evt),
            Err(broadcast::error::TryRecvError::Empty) => Err(SidecarError::ReceiveFailed(
                "No events available".to_string(),
            )),
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                tracing::warn!("EventStream lagged by {n} events in try_recv");
                Err(SidecarError::ReceiveFailed(format!(
                    "Lagged by {} events",
                    n
                )))
            }
            Err(broadcast::error::TryRecvError::Closed) => Err(SidecarError::ChannelClosed),
        }
    }
}

/// Internal control handle
struct Control {
    child_pid: Option<u32>,
    child: Option<Child>,
    _writer_join: JoinHandle<Result<()>>,
    _reader_join: JoinHandle<Result<()>>,
}

impl Control {
    fn new(
        child_pid: Option<u32>,
        child: Child,
        writer_join: JoinHandle<Result<()>>,
        reader_join: JoinHandle<Result<()>>,
    ) -> Self {
        Self {
            child_pid,
            child: Some(child),
            _writer_join: writer_join,
            _reader_join: reader_join,
        }
    }
}

/// Sidecar process facade
///
/// This is the main public API for interacting with the sidecar process.
/// It hides all internal complexity (Mutex, channels, tasks) and provides
/// clean send/receive interfaces.
///
/// # Example
///
/// ```no_run
/// use sidecar::{Sidecar, SidecarCmd};
///
/// let cmd = SidecarCmd::new("python3").arg("main.py");
/// let sidecar = Sidecar::spawn(&cmd).await?;
///
/// // Send audio frame
/// let frame_data = vec![0u8; 320];
/// sidecar.sink.send_frame(frame_data.into()).await?;
///
/// // Receive event
/// let event = sidecar.events.recv().await?;
/// ```
pub struct Sidecar {
    /// Audio sink for sending frames
    pub sink: AudioSink,
    /// Event stream for receiving events
    pub events: EventStream,
    /// Internal control (not public)
    ctrl: Control,
}

impl Sidecar {
    /// Spawn a new sidecar process
    ///
    /// This method:
    /// 1. Spawns the child process
    /// 2. Takes ownership of stdin/stdout
    /// 3. Spawns internal writer/reader tasks
    /// 4. Returns a Sidecar facade with clean AudioSink/EventStream APIs
    pub async fn spawn(cmd: &SidecarCmd) -> Result<Self, SidecarError> {
        let mut child = cmd.spawn().await?;

        // Take stdin/stdout ownership (each will be owned by a single task)
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SidecarError::SpawnFailed("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SidecarError::SpawnFailed("Failed to get stdout".to_string()))?;

        let child_pid = child.id();

        // Spawn internal tasks
        let (sink, writer_join) = spawn_stdio_writer(stdin);
        let (events, reader_join) = spawn_stdio_reader(stdout);

        let ctrl = Control::new(child_pid, child, writer_join, reader_join);

        Ok(Self { sink, events, ctrl })
    }

    /// Get process ID (for testing/debugging)
    pub fn pid(&self) -> Option<u32> {
        self.ctrl.child_pid
    }

    /// Subscribe to event stream (create a new receiver)
    pub fn subscribe(&self) -> EventStream {
        EventStream {
            rx: self.events.rx.resubscribe(),
        }
    }

    /// Wait for child process to exit
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, SidecarError> {
        if let Some(mut child) = self.ctrl.child.take() {
            child
                .wait()
                .await
                .map_err(|_| SidecarError::ProcessTerminated)
        } else {
            Err(SidecarError::ProcessTerminated)
        }
    }

    /// Kill child process
    pub async fn kill(&mut self) -> Result<(), SidecarError> {
        if let Some(child) = self.ctrl.child.as_mut() {
            child
                .kill()
                .await
                .map_err(|_| SidecarError::ProcessTerminated)
        } else {
            Err(SidecarError::ProcessTerminated)
        }
    }

    /// Terminate sidecar process (kill + wait) and consume self
    pub async fn shutdown(mut self) -> Result<(), SidecarError> {
        if let Err(err) = self.kill().await {
            tracing::warn!("Failed to send kill to sidecar: {:?}", err);
        }
        let _ = self.wait().await;
        Ok(())
    }
}

/// Spawn stdio writer task (owns ChildStdin exclusively)
///
/// This task:
/// - Receives frames from mpsc channel
/// - Writes Line-Delimited JSON to stdin
/// - Owns ChildStdin exclusively (no Mutex)
fn spawn_stdio_writer(mut stdin: ChildStdin) -> (AudioSink, JoinHandle<Result<()>>) {
    const CHANNEL_CAPACITY: usize = 500; // 5 seconds at 100 frames/sec

    let (tx, mut rx) = mpsc::channel::<Bytes>(CHANNEL_CAPACITY);

    let join_handle = tokio::spawn(async move {
        let mut frame_counter: u64 = 0;

        while let Some(frame) = rx.recv().await {
            let audio_vec: Vec<u8> = frame.to_vec();

            let msg = serde_json::json!({
                "type": "request",
                "id": format!("burnin-{}", frame_counter),
                "method": "process_audio_stream",
                "params": {
                    "audio_data": audio_vec,
                    "sample_rate": 16000,
                    "channels": 1
                }
            });

            frame_counter = frame_counter.wrapping_add(1);

            let json_line =
                serde_json::to_string(&msg).context("Failed to serialize audio frame request")?;

            stdin
                .write_all(json_line.as_bytes())
                .await
                .context("Failed to write to stdin")?;
            stdin
                .write_all(b"\n")
                .await
                .context("Failed to write newline")?;
            stdin.flush().await.context("Failed to flush stdin")?;
        }
        Ok(())
    });

    (AudioSink { tx }, join_handle)
}

/// Spawn stdio reader task (owns ChildStdout exclusively)
///
/// This task:
/// - Reads Line-Delimited JSON from stdout
/// - Parses JSON into Event
/// - Broadcasts events to all subscribers
/// - Owns ChildStdout exclusively (no Mutex)
fn spawn_stdio_reader(stdout: ChildStdout) -> (EventStream, JoinHandle<Result<()>>) {
    const CHANNEL_CAPACITY: usize = 100; // Broadcast buffer

    let (tx, rx) = broadcast::channel::<Event>(CHANNEL_CAPACITY);

    let join_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .context("Failed to read line from stdout")?;

            if n == 0 {
                // EOF - process terminated
                tracing::info!("Sidecar stdout EOF");
                break;
            }

            // Parse JSON event
            match serde_json::from_str::<Event>(&line) {
                Ok(event) => {
                    if tx.send(event).is_err() {
                        // No receivers - this is OK (may happen during shutdown)
                        tracing::debug!("No event receivers available");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse event JSON: {} (line: {})", e, line.trim());
                }
            }
        }

        Ok(())
    });

    (EventStream { rx }, join_handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_sidecar_cmd_builder() {
        let cmd = SidecarCmd::new("python3").arg("main.py").arg("--debug");
        assert_eq!(cmd.program, "python3");
        assert_eq!(cmd.args, vec!["main.py", "--debug"]);
    }

    /// Phase 1.6: Test concurrent send/receive (no mutex contention)
    ///
    /// This test verifies that:
    /// 1. Sending frames does not block receiving events
    /// 2. Writer/reader tasks execute in parallel (full-duplex)
    /// 3. No deadlock occurs under concurrent load
    #[tokio::test]
    async fn test_sidecar_concurrent_send_receive() {
        // Create a dummy Python process that echoes JSON events
        let dummy_script = r#"
import sys
import json
import time

# Unbuffered I/O
sys.stdout.reconfigure(line_buffering=True)
sys.stdin.reconfigure(line_buffering=True)

# Send ready event
sys.stdout.write(json.dumps({"type": "ready"}) + "\n")
sys.stdout.flush()

# Echo 50 events while consuming stdin frames
for i in range(50):
    # Read one line from stdin (audio_frame)
    try:
        line = sys.stdin.readline()
        if not line:
            sys.stderr.write(f"EOF at iteration {i}\n")
            break

        # Echo a partial_text event
        event = {"type": "partial_text", "text": f"frame_{i}"}
        sys.stdout.write(json.dumps(event) + "\n")
        sys.stdout.flush()

        time.sleep(0.001)  # 1ms delay to simulate STT processing
    except Exception as e:
        sys.stderr.write(f"Error at iteration {i}: {e}\n")
        break

# Keep process alive to consume remaining stdin
sys.stderr.write("Waiting for remaining stdin...\n")
for _ in range(50):
    try:
        line = sys.stdin.readline()
        if not line:
            break
    except:
        break

sys.stderr.write("Python script exiting normally\n")
"#;

        // Create temp script file
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(dummy_script.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Spawn sidecar with dummy script
        let cmd = SidecarCmd::new("python3").arg(temp_file.path().to_str().unwrap());
        let mut sidecar = Sidecar::spawn(&cmd).await.unwrap();

        // Wait for ready event
        let ready = timeout(Duration::from_secs(3), sidecar.events.recv())
            .await
            .expect("Timeout waiting for ready")
            .expect("Failed to receive ready");
        assert!(matches!(ready, Event::Ready));

        // Spawn send task (only send 50 frames to match Python's loop)
        let sink_clone = AudioSink {
            tx: sidecar.sink.tx.clone(),
        };
        let send_task = tokio::spawn(async move {
            for _i in 0..50 {
                let frame = vec![0u8; 320]; // 10ms frame
                if let Err(e) = sink_clone.send_frame(frame.into()).await {
                    // Channel closed - this is OK if Python exited
                    eprintln!("Send frame error (expected if Python exited): {:?}", e);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(2)).await; // 2ms per frame
            }
        });

        // Receive task (50 events expected)
        let recv_task = tokio::spawn(async move {
            let mut count = 0;
            while count < 50 {
                match timeout(Duration::from_secs(5), sidecar.events.recv()).await {
                    Ok(Ok(Event::PartialText { text })) => {
                        assert!(text.starts_with("frame_"));
                        count += 1;
                    }
                    Ok(Ok(_)) => {
                        // Ignore other events
                    }
                    Ok(Err(e)) => {
                        panic!("Receive error: {:?}", e);
                    }
                    Err(_) => {
                        panic!("Timeout receiving event #{}", count);
                    }
                }
            }
            count
        });

        // Wait for both tasks (parallel execution)
        let (send_result, recv_result) = tokio::join!(send_task, recv_task);
        send_result.expect("Send task failed");
        let received_count = recv_result.expect("Receive task failed");

        assert_eq!(received_count, 50, "Should receive 50 events");
    }

    /// Test EventStream::try_recv (non-blocking)
    #[tokio::test]
    async fn test_event_stream_try_recv() {
        let dummy_script = r#"
import sys
import json
sys.stdout.write(json.dumps({"type": "ready"}) + "\n")
sys.stdout.flush()
"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(dummy_script.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let cmd = SidecarCmd::new("python3").arg(temp_file.path().to_str().unwrap());
        let mut sidecar = Sidecar::spawn(&cmd).await.unwrap();

        // Wait for ready event (blocking)
        let ready = sidecar.events.recv().await.unwrap();
        assert!(matches!(ready, Event::Ready));

        // Try non-blocking receive (should fail - no more events)
        let result = sidecar.events.try_recv();
        assert!(result.is_err(), "try_recv should fail when no events");
    }

    /// Test AudioSink::try_send_frame (backpressure)
    #[tokio::test]
    async fn test_audio_sink_backpressure() {
        // Create sidecar that doesn't consume stdin (blocking writer)
        let dummy_script = r#"
import sys
import json
import time
sys.stdout.write(json.dumps({"type": "ready"}) + "\n")
sys.stdout.flush()
time.sleep(10)  # Block for 10 seconds
"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(dummy_script.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let cmd = SidecarCmd::new("python3").arg(temp_file.path().to_str().unwrap());
        let sidecar = Sidecar::spawn(&cmd).await.unwrap();

        // Fill the channel (500 capacity)
        for i in 0..500 {
            let frame = vec![i as u8; 320];
            sidecar
                .sink
                .try_send_frame(frame.into())
                .expect("Should send frame");
        }

        // Next send should fail (channel full)
        let frame = vec![0u8; 320];
        let result = sidecar.sink.try_send_frame(frame.into());
        assert!(
            matches!(result, Err(SidecarError::SendFailed(_))),
            "Should fail due to backpressure"
        );
    }
}
