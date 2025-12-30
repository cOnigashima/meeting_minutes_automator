import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// Phase 4: Google Docs sync status types
type DocsSyncStatus = "idle" | "syncing" | "success" | "error" | "offline";

interface DocsSyncState {
  status: DocsSyncStatus;
  documentId?: string;
  queueSize?: number;
  lastError?: string;
  lastUpdated?: number;
}

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

  // Phase 4: Google Docs sync state
  const [docsSync, setDocsSync] = useState<DocsSyncState>({ status: "idle" });

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
      // Task 9.1: Tauri (v2) expects camelCase keys when bridging to Rust parameters
      const result = await invoke<string>("start_recording", {
        deviceId: selectedDeviceId,
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

  // Phase 4: Listen for Google Docs sync events from WebSocket
  useEffect(() => {
    const unlistenPromise = listen<{
      event: string;
      document_id?: string;
      queue_size?: number;
      error_message?: string;
      timestamp: number;
    }>("docs_sync", (event) => {
      const payload = event.payload;
      console.log("[Meeting Minutes] Docs sync event:", payload);

      switch (payload.event) {
        case "docs_sync_started":
          setDocsSync({
            status: "syncing",
            documentId: payload.document_id,
            lastUpdated: payload.timestamp,
          });
          break;
        case "docs_sync_success":
          setDocsSync({
            status: "success",
            documentId: payload.document_id,
            lastUpdated: payload.timestamp,
          });
          break;
        case "docs_sync_error":
          setDocsSync({
            status: "error",
            documentId: payload.document_id,
            lastError: payload.error_message,
            lastUpdated: payload.timestamp,
          });
          break;
        case "docs_sync_offline":
          setDocsSync((prev) => ({
            ...prev,
            status: "offline",
            queueSize: payload.queue_size,
            lastUpdated: payload.timestamp,
          }));
          break;
        case "docs_sync_online":
          setDocsSync((prev) => ({
            ...prev,
            status: "syncing",
            lastUpdated: payload.timestamp,
          }));
          break;
        case "docs_sync_queue_update":
          setDocsSync((prev) => ({
            ...prev,
            queueSize: payload.queue_size,
            lastUpdated: payload.timestamp,
          }));
          break;
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <main className="container">
      <h1>Meeting Minutes Automator</h1>
      <h2>Walking Skeleton (MVP0)</h2>

      <section className="card">
        <div className="card-header">
          <label htmlFor="audio-device-select">Audio Device</label>
          <select
            id="audio-device-select"
            value={selectedDeviceId}
            onChange={(e) => setSelectedDeviceId(e.target.value)}
            disabled={isRecording}
          >
            {audioDevices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name} ({device.sample_rate / 1000}kHz, {device.channels}ch)
                {device.is_loopback ? " [Loopback]" : ""}
              </option>
            ))}
          </select>
        </div>
        {audioDevices.length === 0 && (
          <p className="helper-text">No audio devices found</p>
        )}
      </section>

      <section className="card">
        <div className="checkbox-row">
          <input
            type="checkbox"
            id="auto-select-model"
            checked={useAutoSelect}
            onChange={(e) => setUseAutoSelect(e.target.checked)}
            disabled={isRecording}
          />
          <label htmlFor="auto-select-model">Auto-select model (recommended)</label>
        </div>

        {whisperModels && (
          <>
            <div className="card-header">
              <label htmlFor="whisper-model-select">Whisper Model</label>
              <select
                id="whisper-model-select"
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                disabled={isRecording || useAutoSelect}
              >
                {whisperModels.available_models.map((model) => (
                  <option key={model} value={model}>
                    {model}
                    {model === whisperModels.recommended_model ? " (recommended)" : ""}
                  </option>
                ))}
              </select>
            </div>

            <div className="helper-text">
              <div>
                System: {whisperModels.system_resources.cpu_cores} cores,
                {whisperModels.system_resources.total_memory_gb}GB RAM
                {whisperModels.system_resources.gpu_available &&
                  `, GPU: ${whisperModels.system_resources.gpu_memory_gb}GB`}
              </div>
              {!useAutoSelect && isModelTooHeavy(selectedModel, whisperModels.recommended_model) && (
                <div className="warning-text">
                  ⚠️ Selected model may exceed system resources (recommended: {whisperModels.recommended_model})
                </div>
              )}
            </div>
          </>
        )}
      </section>

      <section className="card">
        <div className="controls">
          <button className="primary" onClick={startRecording} disabled={isRecording}>
            {isRecording ? "Recording..." : "Start Recording"}
          </button>
          <button className="danger" onClick={stopRecording} disabled={!isRecording}>
            Stop Recording
          </button>
        </div>
        <div className="status-message">{statusMsg}</div>
      </section>

      {/* Phase 4: Google Docs Sync Status */}
      <section className="card">
        <h3>Google Docs Sync</h3>
        <div className="sync-status">
          <span
            className={`sync-badge sync-${docsSync.status}`}
          >
            {docsSync.status === "idle" && "Idle"}
            {docsSync.status === "syncing" && "Syncing..."}
            {docsSync.status === "success" && "Synced"}
            {docsSync.status === "error" && "Error"}
            {docsSync.status === "offline" && "Offline"}
          </span>
          {docsSync.queueSize !== undefined && docsSync.queueSize > 0 && (
            <span className="queue-badge">{docsSync.queueSize} queued</span>
          )}
        </div>
        {docsSync.documentId && (
          <div className="helper-text">
            Document: {docsSync.documentId.substring(0, 16)}...
          </div>
        )}
        {docsSync.lastError && (
          <div className="warning-text">{docsSync.lastError}</div>
        )}
        {docsSync.status === "idle" && (
          <div className="helper-text">
            Connect Chrome extension to start syncing transcriptions to Google Docs.
          </div>
        )}
      </section>

      <section className="card card-muted">
        <h3>Instructions for E2E Test</h3>
        <ol>
          <li>
            Open Chrome and load the extension from <code>chrome-extension/</code>
          </li>
          <li>
            Navigate to <a href="https://meet.google.com" target="_blank">Google Meet</a>
          </li>
          <li>Open Chrome DevTools Console (F12)</li>
          <li>Click "Start Recording" button above</li>
          <li>
            Watch Console for: <code>[Meeting Minutes] 📝 Transcription: This is a fake transcription result</code>
          </li>
          <li>Click "Stop Recording" when done</li>
        </ol>
      </section>
    </main>
  );
}

export default App;
