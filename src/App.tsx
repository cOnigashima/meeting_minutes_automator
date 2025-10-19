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

interface WhisperModelsInfo {
  available_models: string[];
  system_resources: {
    cpu_cores: number;
    total_memory_gb: number;
    gpu_available: boolean;
    gpu_memory_gb: number;
  };
  recommended_model: string;
}

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("");

  // Task 9.2: Whisper model selection state
  const [whisperModels, setWhisperModels] = useState<WhisperModelsInfo | null>(null);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [useAutoSelect, setUseAutoSelect] = useState<boolean>(true);

  // Task 9.2: Model size ranking for STT-REQ-006.5 validation
  const modelRanking = ["tiny", "base", "small", "medium", "large-v3"];

  const getModelRank = (model: string): number => {
    return modelRanking.indexOf(model);
  };

  const isModelTooHeavy = (selected: string, recommended: string): boolean => {
    return getModelRank(selected) > getModelRank(recommended);
  };

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

  // Task 9.2: Load Whisper models on mount
  useEffect(() => {
    async function loadModels() {
      try {
        const modelsInfo = await invoke<WhisperModelsInfo>("get_whisper_models");
        setWhisperModels(modelsInfo);

        // Task 9.2: Restore saved model selection from localStorage
        const savedModel = localStorage.getItem("selectedWhisperModel");
        const savedAutoSelect = localStorage.getItem("useAutoModelSelect");

        if (savedAutoSelect === "false") {
          setUseAutoSelect(false);
          if (savedModel) {
            setSelectedModel(savedModel);
          } else {
            setSelectedModel(modelsInfo.recommended_model);
          }
        } else {
          setUseAutoSelect(true);
          setSelectedModel(modelsInfo.recommended_model);
        }

        console.log("[Meeting Minutes] Loaded Whisper models:", modelsInfo);
      } catch (error) {
        console.error("[Meeting Minutes] Failed to load Whisper models:", error);
        setStatusMsg(`Failed to load Whisper models: ${error}`);
      }
    }
    loadModels();
  }, []);

  // Task 9.1: Save device selection to localStorage when changed
  useEffect(() => {
    if (selectedDeviceId) {
      localStorage.setItem("selectedAudioDeviceId", selectedDeviceId);
      console.log("[Meeting Minutes] Saved device selection:", selectedDeviceId);
    }
  }, [selectedDeviceId]);

  // Task 9.2: Save model selection to localStorage when changed
  useEffect(() => {
    if (selectedModel) {
      localStorage.setItem("selectedWhisperModel", selectedModel);
      console.log("[Meeting Minutes] Saved model selection:", selectedModel);
    }
  }, [selectedModel]);

  // Task 9.2: Save auto-select preference to localStorage when changed
  useEffect(() => {
    localStorage.setItem("useAutoModelSelect", useAutoSelect.toString());
    console.log("[Meeting Minutes] Saved auto-select preference:", useAutoSelect);

    // If auto-select is enabled, use recommended model
    if (useAutoSelect && whisperModels) {
      setSelectedModel(whisperModels.recommended_model);
    }
  }, [useAutoSelect, whisperModels]);

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

      {/* Task 9.2: Whisper Model Selection */}
      <div style={{ marginTop: "2rem", padding: "1rem", backgroundColor: "#f9f9f9", borderRadius: "4px" }}>
        <div style={{ marginBottom: "12px" }}>
          <label style={{ fontWeight: "bold", display: "flex", alignItems: "center" }}>
            <input
              type="checkbox"
              checked={useAutoSelect}
              onChange={(e) => setUseAutoSelect(e.target.checked)}
              disabled={isRecording}
              style={{ marginRight: "8px" }}
            />
            Auto-select model (recommended)
          </label>
        </div>

        {whisperModels && (
          <>
            <label htmlFor="whisper-model-select" style={{ fontWeight: "bold", marginRight: "10px" }}>
              Whisper Model:
            </label>
            <select
              id="whisper-model-select"
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              disabled={isRecording || useAutoSelect}
              style={{
                padding: "8px 12px",
                fontSize: "14px",
                borderRadius: "4px",
                border: "1px solid #ccc",
                minWidth: "200px",
                backgroundColor: useAutoSelect ? "#e0e0e0" : "white",
              }}
            >
              {whisperModels.available_models.map((model) => (
                <option key={model} value={model}>
                  {model}
                  {model === whisperModels.recommended_model ? " (recommended)" : ""}
                </option>
              ))}
            </select>

            {/* System Resources Info */}
            <div style={{ marginTop: "12px", fontSize: "12px", color: "#666" }}>
              <div>
                System: {whisperModels.system_resources.cpu_cores} cores,
                {whisperModels.system_resources.total_memory_gb}GB RAM
                {whisperModels.system_resources.gpu_available &&
                  `, GPU: ${whisperModels.system_resources.gpu_memory_gb}GB`}
              </div>
              {/* Task 9.2: STT-REQ-006.5 - Only warn if selected model is heavier than recommended */}
              {!useAutoSelect && isModelTooHeavy(selectedModel, whisperModels.recommended_model) && (
                <div style={{ color: "#ff9800", marginTop: "4px" }}>
                  ⚠️ Selected model may exceed system resources (recommended: {whisperModels.recommended_model})
                </div>
              )}
            </div>
          </>
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
