/**
 * WebSocket Types for Chrome Extension <-> Tauri Communication
 *
 * Implementation: Phase 4
 */

// =========================================================================
// Inbound Messages (from Tauri to Chrome Extension)
// =========================================================================

export interface ConnectedMessage {
  type: 'connected';
  messageId: string;
  sessionId: string;
  timestamp: number;
}

export interface TranscriptionMessage {
  type: 'transcription';
  messageId: string;
  sessionId: string;
  text: string;
  timestamp: number;
  isPartial?: boolean;
  confidence?: number;
  language?: string;
  processingTimeMs?: number;
}

export interface ErrorMessage {
  type: 'error';
  messageId: string;
  sessionId: string;
  message: string;
  timestamp: number;
}

export interface NotificationMessage {
  type: 'notification';
  messageId: string;
  sessionId: string;
  notificationType: string;
  message: string;
  timestamp: number;
  data?: Record<string, unknown>;
}

export type InboundWebSocketMessage =
  | ConnectedMessage
  | TranscriptionMessage
  | ErrorMessage
  | NotificationMessage;

// =========================================================================
// Outbound Messages (from Chrome Extension to Tauri)
// =========================================================================

export type DocsSyncEventType =
  | 'docs_sync_started'
  | 'docs_sync_success'
  | 'docs_sync_error'
  | 'docs_sync_offline'
  | 'docs_sync_online'
  | 'docs_sync_queue_update';

export interface DocsSyncEvent {
  type: 'docsSync';
  event: DocsSyncEventType;
  documentId?: string;
  queueSize?: number;
  errorMessage?: string;
  timestamp: number;
}

export type OutboundWebSocketMessage = DocsSyncEvent;

// =========================================================================
// Internal Messages (Background <-> Offscreen)
// =========================================================================

export type OffscreenMessageType =
  | 'OFFSCREEN_CONNECT'
  | 'OFFSCREEN_DISCONNECT'
  | 'OFFSCREEN_SEND'
  | 'OFFSCREEN_STATUS'
  | 'OFFSCREEN_CONNECTED'
  | 'OFFSCREEN_DISCONNECTED'
  | 'OFFSCREEN_MESSAGE'
  | 'OFFSCREEN_ERROR';

export interface OffscreenConnectMessage {
  type: 'OFFSCREEN_CONNECT';
}

export interface OffscreenDisconnectMessage {
  type: 'OFFSCREEN_DISCONNECT';
}

export interface OffscreenSendMessage {
  type: 'OFFSCREEN_SEND';
  payload: OutboundWebSocketMessage;
}

export interface OffscreenStatusMessage {
  type: 'OFFSCREEN_STATUS';
}

export interface OffscreenConnectedMessage {
  type: 'OFFSCREEN_CONNECTED';
  port: number;
  sessionId: string;
}

export interface OffscreenDisconnectedMessage {
  type: 'OFFSCREEN_DISCONNECTED';
  reason?: string;
}

export interface OffscreenInboundMessage {
  type: 'OFFSCREEN_MESSAGE';
  payload: InboundWebSocketMessage;
}

export interface OffscreenErrorMessage {
  type: 'OFFSCREEN_ERROR';
  message: string;
}

export type OffscreenRequest =
  | OffscreenConnectMessage
  | OffscreenDisconnectMessage
  | OffscreenSendMessage
  | OffscreenStatusMessage;

export type OffscreenResponse =
  | OffscreenConnectedMessage
  | OffscreenDisconnectedMessage
  | OffscreenInboundMessage
  | OffscreenErrorMessage;

// =========================================================================
// Connection State
// =========================================================================

export type WebSocketConnectionState =
  | 'disconnected'
  | 'scanning'
  | 'connecting'
  | 'connected'
  | 'error';

export interface WebSocketStatus {
  state: WebSocketConnectionState;
  port?: number;
  sessionId?: string;
  lastError?: string;
  lastConnectedAt?: number;
}
