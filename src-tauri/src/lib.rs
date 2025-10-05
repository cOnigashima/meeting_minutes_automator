// Meeting Minutes Automator - Main Library
// Walking Skeleton (MVP0) - WebSocket Server Integration

pub mod audio;
pub mod python_sidecar;
pub mod websocket;
pub mod commands;
pub mod state;

use state::AppState;
use websocket::WebSocketServer;
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

            // Start WebSocket server asynchronously
            // AC-006.1: WHEN Tauri app starts THEN start WebSocket server
            tauri::async_runtime::spawn(async move {
                let mut ws_server = WebSocketServer::new();

                match ws_server.start().await {
                    Ok(port) => {
                        println!("[Meeting Minutes] ✅ WebSocket server started on port {}", port);

                        // Store server in AppState
                        let server_arc = Arc::new(tokio::sync::Mutex::new(ws_server));
                        let app_state = app_handle.state::<AppState>();
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
