#!/usr/bin/env python3
"""
Python Sidecar Process - Walking Skeleton Implementation
Handles stdin/stdout JSON IPC communication with Tauri backend
"""

import sys
import json
import time
from typing import Dict, Any

def send_message(msg: Dict[str, Any]) -> None:
    """Send JSON message to stdout"""
    json_str = json.dumps(msg)
    print(json_str, flush=True)

def receive_message() -> Dict[str, Any]:
    """Receive JSON message from stdin"""
    line = sys.stdin.readline()
    if not line:
        return None
    return json.loads(line.strip())

def handle_ping(msg_id: str) -> Dict[str, Any]:
    """Handle ping message"""
    return {
        "type": "pong",
        "id": msg_id,
        "timestamp": int(time.time() * 1000)
    }

def handle_process_audio(msg_id: str, audio_data: list) -> Dict[str, Any]:
    """Handle process_audio message (Fake implementation)"""
    # Walking Skeleton: Return fixed fake transcription (Task 5.2 requirement)
    return {
        "type": "transcription_result",
        "id": msg_id,
        "text": "This is a fake transcription result",
        "timestamp": int(time.time() * 1000)
    }

def main():
    """Main IPC loop"""
    # Send ready signal
    send_message({
        "type": "ready",
        "timestamp": int(time.time() * 1000)
    })

    # IPC message loop
    while True:
        try:
            msg = receive_message()
            if msg is None:
                break  # EOF

            msg_type = msg.get("type")
            msg_id = msg.get("id", "unknown")

            if msg_type == "ping":
                response = handle_ping(msg_id)
                send_message(response)

            elif msg_type == "process_audio":
                audio_data = msg.get("audio_data", [])
                response = handle_process_audio(msg_id, audio_data)
                send_message(response)

            elif msg_type == "shutdown":
                send_message({
                    "type": "shutdown_ack",
                    "id": msg_id
                })
                break

            else:
                # Unknown message type
                send_message({
                    "type": "error",
                    "id": msg_id,
                    "message": f"Unknown message type: {msg_type}"
                })

        except json.JSONDecodeError as e:
            send_message({
                "type": "error",
                "message": f"JSON decode error: {str(e)}"
            })
        except Exception as e:
            send_message({
                "type": "error",
                "message": f"Unexpected error: {str(e)}"
            })
            break

if __name__ == "__main__":
    main()
