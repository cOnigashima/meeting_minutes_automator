"""
IPC Handler for communication with Rust parent process
Walking Skeleton (MVP0) - Empty Implementation
"""

import asyncio
from typing import Dict, Any


class IpcHandler:
    """Handler for IPC communication via stdin/stdout"""

    def __init__(self):
        pass

    async def send_message(self, message: Dict[str, Any]) -> None:
        """Send a message to the Rust parent process via stdout"""
        raise NotImplementedError("IpcHandler.send_message - to be implemented in Task 4.1")

    async def receive_message(self) -> Dict[str, Any]:
        """Receive a message from the Rust parent process via stdin"""
        raise NotImplementedError("IpcHandler.receive_message - to be implemented in Task 4.1")

    async def start(self) -> None:
        """Start the IPC handler event loop"""
        raise NotImplementedError("IpcHandler.start - to be implemented in Task 4.1")

    async def stop(self) -> None:
        """Stop the IPC handler"""
        raise NotImplementedError("IpcHandler.stop - to be implemented in Task 5.1")
