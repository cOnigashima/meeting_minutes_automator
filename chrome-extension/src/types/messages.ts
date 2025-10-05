/**
 * WebSocket Message Types
 * Walking Skeleton (MVP0) - Type Definitions
 */

export type WebSocketMessageType = 'connected' | 'transcription' | 'error';

export interface ConnectedMessage {
  type: 'connected';
  sessionId: string;
}

export interface TranscriptionMessage {
  type: 'transcription';
  text: string;
  timestamp: number;
}

export interface ErrorMessage {
  type: 'error';
  message: string;
}

export type WebSocketMessage = ConnectedMessage | TranscriptionMessage | ErrorMessage;
