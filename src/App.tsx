import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface AudioDeviceInfo {
  id: string;
  name: string;
  sample_rate: number;
  channels: number;
  is_loopback: boolean;
}

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("");

  async function startRecording() {
    // Task 9.1: Pass selected device_id to backend (STT-REQ-001.2)
    if (!selectedDeviceId) {
      setStatusMsg("Error: No device selected");
      return;
    }

    try {
      // Task 9.1: Use snake_case for Tauri command parameters (Rust convention)
      const result = await invoke<string>("start_recording", {
        device_id: selectedDeviceId,
      });
      setStatusMsg(result);
      setIsRecording(true);
      console.log(`[Meeting Minutes] Recording started with device: ${selectedDeviceId}`);
    } catch (error) {
      setStatusMsg(`Error: ${error}`);
      console.error("[Meeting Minutes] Failed to start recording:", error);
    }
  }

  async function stopRecording() {
    try {
      const result = await invoke("stop_recording");
      setStatusMsg(result as string);
      setIsRecording(false);
      console.log("[Meeting Minutes] Recording stopped");
    } catch (error) {
      setStatusMsg(`Error: ${error}`);
      console.error("[Meeting Minutes] Failed to stop recording:", error);
    }
  }

  // Task 9.1: Load audio devices on mount
  useEffect(() => {
    async function loadDevices() {
      try {
        const devices = await invoke<AudioDeviceInfo[]>("list_audio_devices");
        setAudioDevices(devices);

        // Task 9.1: Restore saved device selection from localStorage
        const savedDeviceId = localStorage.getItem("selectedAudioDeviceId");
        if (savedDeviceId && devices.some(d => d.id === savedDeviceId)) {
          setSelectedDeviceId(savedDeviceId);
        } else if (devices.length > 0) {
          setSelectedDeviceId(devices[0].id);
        }

        console.log("[Meeting Minutes] Loaded audio devices:", devices);
      } catch (error) {
        console.error("[Meeting Minutes] Failed to load audio devices:", error);
        setStatusMsg(`Failed to load audio devices: ${error}`);
      }
    }
    loadDevices();
  }, []);

  // Task 9.1: Save device selection to localStorage when changed
  useEffect(() => {
    if (selectedDeviceId) {
      localStorage.setItem("selectedAudioDeviceId", selectedDeviceId);
      console.log("[Meeting Minutes] Saved device selection:", selectedDeviceId);
    }
  }, [selectedDeviceId]);

  return (
    <main className="container">
      <h1>Meeting Minutes Automator</h1>
      <h2>Walking Skeleton (MVP0)</h2>

      {/* Task 9.1: Audio Device Selection */}
      <div style={{ marginTop: "2rem", padding: "1rem", backgroundColor: "#f9f9f9", borderRadius: "4px" }}>
        <label htmlFor="audio-device-select" style={{ fontWeight: "bold", marginRight: "10px" }}>
          Audio Device:
        </label>
        <select
          id="audio-device-select"
          value={selectedDeviceId}
          onChange={(e) => setSelectedDeviceId(e.target.value)}
          disabled={isRecording}
          style={{
            padding: "8px 12px",
            fontSize: "14px",
            borderRadius: "4px",
            border: "1px solid #ccc",
            minWidth: "300px",
          }}
        >
          {audioDevices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name} ({device.sample_rate / 1000}kHz, {device.channels}ch)
              {device.is_loopback ? " [Loopback]" : ""}
            </option>
          ))}
        </select>
        {audioDevices.length === 0 && (
          <p style={{ marginTop: "8px", color: "#999", fontSize: "12px" }}>
            No audio devices found
          </p>
        )}
      </div>

      <div className="row" style={{ marginTop: "2rem" }}>
        <button
          onClick={startRecording}
          disabled={isRecording}
          style={{
            padding: "10px 20px",
            fontSize: "16px",
            backgroundColor: isRecording ? "#ccc" : "#4CAF50",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: isRecording ? "not-allowed" : "pointer",
          }}
        >
          {isRecording ? "Recording..." : "Start Recording"}
        </button>

        <button
          onClick={stopRecording}
          disabled={!isRecording}
          style={{
            padding: "10px 20px",
            fontSize: "16px",
            backgroundColor: !isRecording ? "#ccc" : "#f44336",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: !isRecording ? "not-allowed" : "pointer",
            marginLeft: "10px",
          }}
        >
          Stop Recording
        </button>
      </div>

      <p style={{ marginTop: "1rem", color: "#666" }}>{statusMsg}</p>

      <div style={{ marginTop: "2rem", padding: "1rem", backgroundColor: "#f5f5f5", borderRadius: "4px" }}>
        <h3>Instructions for E2E Test:</h3>
        <ol style={{ textAlign: "left", maxWidth: "600px", margin: "0 auto" }}>
          <li>Open Chrome and load the extension from <code>chrome-extension/</code></li>
          <li>Navigate to <a href="https://meet.google.com" target="_blank">Google Meet</a></li>
          <li>Open Chrome DevTools Console (F12)</li>
          <li>Click "Start Recording" button above</li>
          <li>Watch Console for: <code>[Meeting Minutes] 📝 Transcription: This is a fake transcription result</code></li>
          <li>Click "Stop Recording" when done</li>
        </ol>
      </div>
    </main>
  );
}

export default App;
