import { useState, useEffect, useRef } from "react";
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

// STTMIX Task 8: Platform info for feature gating
interface PlatformInfo {
  os: string;
  multi_input_supported: boolean;
}

// STTMIX Task 8.3: Multi-input status display
interface InputStatus {
  device_id: string;
  role: "Microphone" | "Loopback";
  is_active: boolean;
  buffer_occupancy_percent: number;
  buffer_level_bytes: number;
  buffer_max_bytes: number;
  gain_db: number;
  is_muted: boolean;
  lock_contention_drops: number;
}

interface MixerMetrics {
  drift_correction_count: number;
  clip_count: number;
  silence_insertion_count: number;
  frames_mixed: number;
  // Task 9.1: Latency metrics
  max_mix_latency_ms: number;
  avg_mix_latency_ms: number;
}

interface MultiInputStatusResponse {
  inputs: InputStatus[];
  is_recording: boolean;
  mixer_metrics: MixerMetrics | null;
}

// Debug: Real-time transcription display
interface TranscriptionEntry {
  id: string;
  text: string;
  is_partial: boolean;
  confidence?: number;
  language?: string;
  timestamp: number;
}

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("");

  // STTMIX Task 8: Multi-input state
  const [platformInfo, setPlatformInfo] = useState<PlatformInfo | null>(null);
  const [multiInputEnabled, setMultiInputEnabled] = useState(false);
  const [selectedDeviceIds, setSelectedDeviceIds] = useState<string[]>([]);
  const [deviceRoles, setDeviceRoles] = useState<Record<string, "Microphone" | "Loopback">>({});
  // STTMIX Task 8.3: Multi-input status display
  const [multiInputStatus, setMultiInputStatus] = useState<MultiInputStatusResponse | null>(null);

  // Task 9.2: Whisper model selection state
  const [whisperModels, setWhisperModels] = useState<WhisperModelsInfo | null>(null);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [useAutoSelect, setUseAutoSelect] = useState<boolean>(true);

  // Phase 4: Google Docs sync state
  const [docsSync, setDocsSync] = useState<DocsSyncState>({ status: "idle" });
  const lastDocsSyncUpdateRef = useRef(0);

  // Debug: Real-time transcription display
  const [transcriptions, setTranscriptions] = useState<TranscriptionEntry[]>([]);

  // Task 9.2: Model size ranking for STT-REQ-006.5 validation
  const modelRanking = ["tiny", "base", "small", "medium", "large-v3"];

  const getModelRank = (model: string): number => {
    return modelRanking.indexOf(model);
  };

  const isModelTooHeavy = (selected: string, recommended: string): boolean => {
    return getModelRank(selected) > getModelRank(recommended);
  };

  async function startRecording() {
    // STTMIX Task 8: Use multi-input mode if enabled
    if (multiInputEnabled && platformInfo?.multi_input_supported) {
      if (selectedDeviceIds.length === 0) {
        setStatusMsg("Error: No devices selected for multi-input");
        return;
      }
      if (selectedDeviceIds.length > 2) {
        setStatusMsg("Error: Maximum 2 devices allowed");
        return;
      }

      try {
        const result = await invoke<string>("start_recording_multi", {
          deviceIds: selectedDeviceIds,
        });
        setStatusMsg(result);
        setIsRecording(true);
        console.log(`[Meeting Minutes] Multi-input recording started with devices:`, selectedDeviceIds);
      } catch (error) {
        setStatusMsg(`Error: ${error}`);
        console.error("[Meeting Minutes] Failed to start multi-input recording:", error);
      }
      return;
    }

    // Task 9.1: Single device mode (backward compatible)
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

  // STTMIX Task 8: Load platform info on mount
  useEffect(() => {
    async function loadPlatformInfo() {
      try {
        const info = await invoke<PlatformInfo>("get_platform_info");
        setPlatformInfo(info);
        console.log("[Meeting Minutes] Platform info:", info);

        // Critical fix: Force disable multi-input on non-macOS to prevent UI deadlock
        // Even if localStorage has multiInputEnabled=true, we must disable it
        // otherwise the single-device UI is hidden but multi-input can't start
        if (!info.multi_input_supported) {
          setMultiInputEnabled(false);
          console.log("[Meeting Minutes] Multi-input disabled: not supported on this platform");
        }
      } catch (error) {
        console.error("[Meeting Minutes] Failed to load platform info:", error);
        // On error, also disable multi-input for safety
        setMultiInputEnabled(false);
      }
    }
    loadPlatformInfo();
  }, []);

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

        // STTMIX Task 8: Restore multi-input settings
        const savedMultiInput = localStorage.getItem("multiInputEnabled");
        const savedDeviceIds = localStorage.getItem("selectedDeviceIds");
        const savedRoles = localStorage.getItem("deviceRoles");

        if (savedMultiInput === "true") {
          setMultiInputEnabled(true);
          if (savedDeviceIds) {
            const ids = JSON.parse(savedDeviceIds) as string[];
            // Filter to only include available devices
            const validIds = ids.filter(id => devices.some(d => d.id === id));
            setSelectedDeviceIds(validIds);
          }
          if (savedRoles) {
            setDeviceRoles(JSON.parse(savedRoles));
          }
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

  // STTMIX Task 8: Save multi-input settings to localStorage
  useEffect(() => {
    localStorage.setItem("multiInputEnabled", multiInputEnabled.toString());
    localStorage.setItem("selectedDeviceIds", JSON.stringify(selectedDeviceIds));
    localStorage.setItem("deviceRoles", JSON.stringify(deviceRoles));
  }, [multiInputEnabled, selectedDeviceIds, deviceRoles]);

  // STTMIX Task 8: Toggle device selection for multi-input mode
  const toggleDeviceSelection = (deviceId: string, device: AudioDeviceInfo) => {
    setSelectedDeviceIds(prev => {
      if (prev.includes(deviceId)) {
        // Remove device
        const newIds = prev.filter(id => id !== deviceId);
        setDeviceRoles(roles => {
          const newRoles = { ...roles };
          delete newRoles[deviceId];
          return newRoles;
        });
        return newIds;
      } else {
        // Add device (max 2)
        if (prev.length >= 2) {
          setStatusMsg("Maximum 2 devices can be selected");
          return prev;
        }
        // Auto-assign role based on device type
        const role = device.is_loopback ? "Loopback" : "Microphone";
        setDeviceRoles(roles => ({ ...roles, [deviceId]: role }));
        return [...prev, deviceId];
      }
    });
  };

  // Debug: Listen for real-time transcription events
  useEffect(() => {
    const unlistenPromise = listen<{
      session_id: string;
      text: string;
      is_partial: boolean;
      confidence?: number;
      language?: string;
      timestamp: number;
    }>("transcription", (event) => {
      const payload = event.payload;
      console.log("[Meeting Minutes] Transcription:", payload);

      const entry: TranscriptionEntry = {
        id: `${payload.timestamp}-${payload.is_partial ? "partial" : "final"}`,
        text: payload.text,
        is_partial: payload.is_partial,
        confidence: payload.confidence,
        language: payload.language,
        timestamp: payload.timestamp,
      };

      setTranscriptions((prev) => {
        // Replace partial with final, or add new entry
        if (!payload.is_partial) {
          // Final text: remove partial entries with similar text
          const filtered = prev.filter((t) => t.is_partial === false || t.text !== payload.text);
          return [...filtered.slice(-49), entry]; // Keep last 50
        }
        // Partial text: just append (will be replaced by final)
        return [...prev.slice(-49), entry];
      });
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Clear transcriptions when recording stops
  useEffect(() => {
    if (!isRecording) {
      // Keep transcriptions for review after stop
      // setTranscriptions([]);
    }
  }, [isRecording]);

  // STTMIX Task 8.3: Poll multi-input status during recording
  useEffect(() => {
    if (!isRecording || !multiInputEnabled) {
      setMultiInputStatus(null);
      return;
    }

    const pollStatus = async () => {
      try {
        const status = await invoke<MultiInputStatusResponse>("get_multi_input_status");
        setMultiInputStatus(status);
      } catch (error) {
        console.error("[Meeting Minutes] Failed to get multi-input status:", error);
      }
    };

    // Initial poll
    pollStatus();

    // Poll every 500ms during recording
    const intervalId = setInterval(pollStatus, 500);

    return () => clearInterval(intervalId);
  }, [isRecording, multiInputEnabled]);

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
      const throttledEvents = new Set(["docs_sync_success", "docs_sync_queue_update"]);
      const now = Date.now();

      if (throttledEvents.has(payload.event)) {
        if (now - lastDocsSyncUpdateRef.current < 1000) {
          return;
        }
        lastDocsSyncUpdateRef.current = now;
      }

      switch (payload.event) {
        case "docs_sync_started":
          setDocsSync({
            status: "syncing",
            documentId: payload.document_id,
            lastUpdated: payload.timestamp,
          });
          break;
        case "docs_sync_success":
          setDocsSync((prev) => {
            const next = {
              status: "success" as DocsSyncStatus,
              documentId: payload.document_id,
              lastUpdated: payload.timestamp,
            };
            if (
              prev.status === next.status &&
              prev.documentId === next.documentId
            ) {
              return prev;
            }
            return next;
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
          setDocsSync((prev) => {
            if (prev.queueSize === payload.queue_size) {
              return prev;
            }
            return {
              ...prev,
              queueSize: payload.queue_size,
              lastUpdated: payload.timestamp,
            };
          });
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
        {/* STTMIX Task 8: Multi-input toggle (only on macOS) */}
        {platformInfo?.multi_input_supported && (
          <div className="checkbox-row" style={{ marginBottom: "12px" }}>
            <input
              type="checkbox"
              id="multi-input-toggle"
              checked={multiInputEnabled}
              onChange={(e) => setMultiInputEnabled(e.target.checked)}
              disabled={isRecording}
            />
            <label htmlFor="multi-input-toggle">
              Multi-Input Mode (Mic + Loopback)
            </label>
          </div>
        )}

        {!platformInfo?.multi_input_supported && platformInfo && (
          <div className="helper-text" style={{ marginBottom: "12px", color: "#888" }}>
            Multi-input mode is only available on macOS (current: {platformInfo.os})
          </div>
        )}

        {/* Single device selection (default mode) */}
        {!multiInputEnabled && (
          <>
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
          </>
        )}

        {/* STTMIX Task 8.1: Multi-device selection */}
        {multiInputEnabled && (
          <>
            <div className="card-header">
              <label>Select Devices (max 2)</label>
            </div>
            <div style={{ maxHeight: "200px", overflowY: "auto" }}>
              {audioDevices.map((device) => (
                <div
                  key={device.id}
                  className="checkbox-row"
                  style={{
                    padding: "6px 8px",
                    borderRadius: "4px",
                    backgroundColor: selectedDeviceIds.includes(device.id) ? "#2a3f5f" : "transparent",
                  }}
                >
                  <input
                    type="checkbox"
                    id={`device-${device.id}`}
                    checked={selectedDeviceIds.includes(device.id)}
                    onChange={() => toggleDeviceSelection(device.id, device)}
                    disabled={isRecording || (!selectedDeviceIds.includes(device.id) && selectedDeviceIds.length >= 2)}
                  />
                  <label htmlFor={`device-${device.id}`} style={{ flex: 1 }}>
                    <span style={{ color: device.is_loopback ? "#f39c12" : "#27ae60" }}>
                      {device.is_loopback ? "🔊" : "🎤"}
                    </span>{" "}
                    {device.name}
                    <span style={{ color: "#888", marginLeft: "8px" }}>
                      ({device.sample_rate / 1000}kHz)
                    </span>
                  </label>
                  {selectedDeviceIds.includes(device.id) && (
                    <span
                      style={{
                        fontSize: "11px",
                        padding: "2px 6px",
                        borderRadius: "3px",
                        backgroundColor: deviceRoles[device.id] === "Loopback" ? "#f39c12" : "#27ae60",
                        color: "#000",
                      }}
                    >
                      {deviceRoles[device.id]}
                    </span>
                  )}
                </div>
              ))}
            </div>
            {selectedDeviceIds.length === 0 && (
              <p className="helper-text">Select 1-2 devices for multi-input recording</p>
            )}
            {selectedDeviceIds.length > 0 && (
              <p className="helper-text" style={{ color: "#27ae60" }}>
                {selectedDeviceIds.length} device(s) selected
              </p>
            )}

            {/* STTMIX Task 8.3: Multi-input status display during recording */}
            {isRecording && multiInputStatus && multiInputStatus.inputs.length > 0 && (
              <div style={{ marginTop: "12px", padding: "8px", backgroundColor: "#1a2538", borderRadius: "6px" }}>
                <div style={{ fontSize: "11px", color: "#888", marginBottom: "8px" }}>
                  INPUT STATUS
                </div>
                {multiInputStatus.inputs.map((input) => {
                  const deviceName = audioDevices.find(d => d.id === input.device_id)?.name || input.device_id;
                  const occupancyColor = input.buffer_occupancy_percent > 80 ? "#e74c3c" :
                                         input.buffer_occupancy_percent > 50 ? "#f39c12" : "#27ae60";
                  return (
                    <div key={input.device_id} style={{ marginBottom: "8px" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "4px" }}>
                        <span style={{ color: input.role === "Loopback" ? "#f39c12" : "#27ae60" }}>
                          {input.role === "Loopback" ? "🔊" : "🎤"}
                        </span>
                        <span style={{ fontSize: "12px", flex: 1 }}>{deviceName}</span>
                        <span style={{
                          fontSize: "10px",
                          padding: "2px 4px",
                          borderRadius: "3px",
                          backgroundColor: input.is_active ? "#27ae60" : "#e74c3c",
                          color: "#fff"
                        }}>
                          {input.is_active ? "ACTIVE" : "LOST"}
                        </span>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                        <div style={{
                          flex: 1,
                          height: "6px",
                          backgroundColor: "#2a3f5f",
                          borderRadius: "3px",
                          overflow: "hidden"
                        }}>
                          <div style={{
                            width: `${Math.min(100, input.buffer_occupancy_percent)}%`,
                            height: "100%",
                            backgroundColor: occupancyColor,
                            transition: "width 0.3s ease"
                          }} />
                        </div>
                        <span style={{ fontSize: "10px", color: "#888", minWidth: "40px" }}>
                          {input.buffer_occupancy_percent.toFixed(0)}%
                        </span>
                      </div>
                    </div>
                  );
                })}
                {multiInputStatus.mixer_metrics && (
                  <div style={{ marginTop: "8px", fontSize: "10px", color: "#888", display: "flex", gap: "12px", flexWrap: "wrap" }}>
                    <span>Frames: {multiInputStatus.mixer_metrics.frames_mixed}</span>
                    <span style={{ color: multiInputStatus.mixer_metrics.clip_count > 0 ? "#e74c3c" : "#888" }}>
                      Clips: {multiInputStatus.mixer_metrics.clip_count}
                    </span>
                    <span>Drift: {multiInputStatus.mixer_metrics.drift_correction_count}</span>
                    <span style={{ color: multiInputStatus.mixer_metrics.max_mix_latency_ms > 10 ? "#f39c12" : "#888" }}>
                      Latency: {multiInputStatus.mixer_metrics.avg_mix_latency_ms.toFixed(2)}ms (max: {multiInputStatus.mixer_metrics.max_mix_latency_ms.toFixed(2)}ms)
                    </span>
                  </div>
                )}
              </div>
            )}
          </>
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

      {/* Debug: Real-time Transcription Display */}
      <section className="card">
        <h3>
          Transcription
          {transcriptions.length > 0 && (
            <button
              className="clear-btn"
              onClick={() => setTranscriptions([])}
              style={{ marginLeft: "8px", fontSize: "12px", padding: "2px 8px" }}
            >
              Clear
            </button>
          )}
        </h3>
        <div
          className="transcription-box"
          style={{
            maxHeight: "200px",
            overflowY: "auto",
            padding: "8px",
            backgroundColor: "#1a1a2e",
            borderRadius: "4px",
            fontFamily: "monospace",
            fontSize: "13px",
          }}
        >
          {transcriptions.length === 0 ? (
            <div style={{ color: "#666" }}>No transcriptions yet. Start recording to see real-time text.</div>
          ) : (
            transcriptions.map((t) => (
              <div
                key={t.id}
                style={{
                  marginBottom: "4px",
                  color: t.is_partial ? "#888" : "#fff",
                  fontStyle: t.is_partial ? "italic" : "normal",
                }}
              >
                <span style={{ color: "#666", marginRight: "8px" }}>
                  {new Date(t.timestamp).toLocaleTimeString()}
                </span>
                {t.is_partial && <span style={{ color: "#f39c12" }}>[partial] </span>}
                {t.text}
                {t.confidence !== undefined && (
                  <span style={{ color: "#27ae60", marginLeft: "8px" }}>
                    ({(t.confidence * 100).toFixed(0)}%)
                  </span>
                )}
              </div>
            ))
          )}
        </div>
      </section>

      {/* Phase 4: Google Docs Sync Status */}
      <section className="card">
        <h3>Google Docs Sync</h3>
        <div className="sync-status">
          <span
            className={`sync-badge sync-${docsSync.status}`}
          >
            <span
              className={`sync-dot sync-${docsSync.status}`}
              aria-hidden="true"
            ></span>
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
            Waiting for the first transcription. Ensure the Chrome extension is connected and Docs Sync is enabled.
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
            In the extension popup, authenticate Google Docs, set the Document ID, and enable Docs Sync
          </li>
          <li>Select your loopback device (e.g., BlackHole) in the audio input list above</li>
          <li>Click "Start Recording" button above</li>
          <li>
            Speak audio and confirm Google Docs updates plus the sync badge changes
          </li>
          <li>Click "Stop Recording" when done</li>
        </ol>
      </section>
    </main>
  );
}

export default App;
