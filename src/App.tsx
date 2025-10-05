import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");

  async function startRecording() {
    try {
      const result = await invoke("start_recording");
      setStatusMsg(result as string);
      setIsRecording(true);
      console.log("[Meeting Minutes] Recording started");
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

  return (
    <main className="container">
      <h1>Meeting Minutes Automator</h1>
      <h2>Walking Skeleton (MVP0)</h2>

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
