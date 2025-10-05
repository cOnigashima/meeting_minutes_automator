// Meeting Minutes Automator - Main Library
// Walking Skeleton (MVP0) - WebSocket Server Integration

pub mod audio;
pub mod python_sidecar;
pub mod websocket;
pub mod commands;
pub mod state;

use state::AppState;
use websocket::WebSocketServer;
use python_sidecar::PythonSidecarManager;
use audio::FakeAudioDevice;
use std::sync::Arc;
use tauri::Manager;

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
                        println!("[Meeting Minutes] ✅ Python sidecar started");

                        // Wait for ready signal
                        match sidecar.wait_for_ready().await {
                            Ok(_) => {
                                println!("[Meeting Minutes] ✅ Python sidecar ready");
                                let sidecar_arc = Arc::new(tokio::sync::Mutex::new(sidecar));
                                app_state.set_python_sidecar(sidecar_arc);
                            }
                            Err(e) => {
                                eprintln!("[Meeting Minutes] ❌ Python sidecar ready timeout: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Meeting Minutes] ❌ Failed to start Python sidecar: {:?}", e);
                    }
                }

                // 2. Initialize FakeAudioDevice
                let audio_device = FakeAudioDevice::new();
                println!("[Meeting Minutes] ✅ FakeAudioDevice initialized");
                let device_arc = Arc::new(tokio::sync::Mutex::new(audio_device));
                app_state.set_audio_device(device_arc);

                // 3. Start WebSocket server
                let mut ws_server = WebSocketServer::new();
                match ws_server.start().await {
                    Ok(port) => {
                        println!("[Meeting Minutes] ✅ WebSocket server started on port {}", port);
                        let server_arc = Arc::new(tokio::sync::Mutex::new(ws_server));
                        app_state.set_websocket_server(server_arc);
                    }
                    Err(e) => {
                        eprintln!("[Meeting Minutes] ❌ Failed to start WebSocket server: {:?}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
