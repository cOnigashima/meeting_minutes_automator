// Meeting Minutes Automator - Main Library
// Walking Skeleton (MVP0) - WebSocket Server Integration
// MVP1 - Real STT Implementation
// Task 10.4 Phase 2 - Device Reconnection

#[macro_use]
pub mod logger;
pub mod audio;
pub mod audio_device_adapter;
pub mod commands;
pub mod ipc_protocol;
pub mod python_sidecar;
pub mod reconnection_manager; // Task 10.4 Phase 2 - STT-REQ-004.11
pub mod ring_buffer; // ADR-013: Phase 2 - Ring Buffer
pub mod sidecar; // ADR-013: Phase 1 - Facade API
pub mod state;
pub mod storage;
pub mod websocket;

use audio_device_adapter::create_audio_adapter;
use python_sidecar::PythonSidecarManager;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use websocket::WebSocketServer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .setup(|app| {
            // Get AppHandle for use in async task
            let app_handle = app.handle().clone();

            // Initialize all components asynchronously
            // AC-003.2: Start Python sidecar process
            // AC-006.1: Start WebSocket server
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();

                // 1. Start Python sidecar
                let mut sidecar = PythonSidecarManager::new();
                match sidecar.start().await {
                    Ok(_) => {
                        log_info!("bootstrap::python", "sidecar_started", "");

                        // Wait for ready signal
                        match sidecar.wait_for_ready().await {
                            Ok(_) => {
                                log_info!("bootstrap::python", "sidecar_ready", "");
                                let sidecar_arc = Arc::new(tokio::sync::Mutex::new(sidecar));
                                app_state.set_python_sidecar(sidecar_arc);
                            }
                            Err(e) => {
                                log_error!(
                                    "bootstrap::python",
                                    "sidecar_ready_timeout",
                                    format!("{:?}", e)
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log_error!(
                            "bootstrap::python",
                            "sidecar_start_failed",
                            format!("{:?}", e)
                        );
                    }
                }

                // 2. Initialize real audio device adapter
                match create_audio_adapter() {
                    Ok(audio_device) => {
                        log_info!("bootstrap::audio", "real_device_initialized", "");
                        let device_arc = Arc::new(tokio::sync::Mutex::new(audio_device));
                        app_state.set_audio_device(device_arc);
                    }
                    Err(e) => {
                        log_error!(
                            "bootstrap::audio",
                            "device_init_failed",
                            format!("{:?}", e)
                        );
                    }
                }

                // 2.5. Initialize audio event channel (MVP1 - STT-REQ-004.9/10/11)
                let (audio_event_tx, audio_event_rx) = std::sync::mpsc::channel();
                app_state.set_audio_event_channel(audio_event_tx, audio_event_rx);

                // 2.6. Start monitoring audio device events
                let app_clone = app_handle.clone();
                tokio::spawn(async move {
                    commands::monitor_audio_events(app_clone).await;
                });

                // 3. Start WebSocket server
                let mut ws_server = WebSocketServer::new_with_app_handle(app_handle.clone());
                match ws_server.start().await {
                    Ok(port) => {
                        log_info!(
                            "bootstrap::websocket",
                            "server_started",
                            format!("port={}", port)
                        );
                        let server_arc = Arc::new(tokio::sync::Mutex::new(ws_server));
                        app_state.set_websocket_server(server_arc);
                    }
                    Err(e) => {
                        log_error!(
                            "bootstrap::websocket",
                            "server_start_failed",
                            format!("{:?}", e)
                        );
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::list_audio_devices,
            commands::get_whisper_models,
            commands::cancel_reconnection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
