#!/usr/bin/env python3
"""
Test script to verify ready message output format.
This script simulates the ready message send logic without dependencies.
"""

import sys
import json

def main():
    # Simulate ready message (matching main.py L650-653)
    message = {
        'type': 'ready',
        'version': '1.0',  # Added by IpcHandler if not present
        'message': 'Python sidecar ready (MVP1 Real STT)'
    }

    # Serialize to JSON (matching IpcHandler.send_message L103)
    json_str = json.dumps(message, separators=(',', ':'))

    # Add newline delimiter (matching IpcHandler.send_message L112)
    data = (json_str + '\n').encode('utf-8')

    # Write to stdout (matching IpcHandler.send_message L115-116)
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()

    # Write debug info to stderr
    print(f"[DEBUG] Sent to stdout: {json_str}", file=sys.stderr)
    print(f"[DEBUG] Length: {len(data)} bytes", file=sys.stderr)

if __name__ == '__main__':
    main()
