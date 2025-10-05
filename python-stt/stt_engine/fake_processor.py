"""
Fake Audio Processor for Walking Skeleton
Walking Skeleton (MVP0) - Empty Implementation
"""

from typing import Dict, Any


class FakeProcessor:
    """Fake processor that returns dummy transcription results"""

    def __init__(self):
        pass

    async def process_audio(self, audio_data: bytes) -> Dict[str, Any]:
        """Process audio data and return fake transcription result"""
        raise NotImplementedError("FakeProcessor.process_audio - to be implemented in Task 4.2")

    async def start(self) -> None:
        """Start the processor"""
        raise NotImplementedError("FakeProcessor.start - to be implemented in Task 4.2")

    async def stop(self) -> None:
        """Stop the processor"""
        raise NotImplementedError("FakeProcessor.stop - to be implemented in Task 5.1")
