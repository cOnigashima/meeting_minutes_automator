/**
 * Offscreen Document - WebSocket Client
 *
 * Maintains persistent WebSocket connection to Tauri app.
 * Survives Service Worker sleep cycles in Manifest V3.
 *
 * Implementation: Phase 4
 */

import type {
  InboundWebSocketMessage,
  OutboundWebSocketMessage,
  OffscreenRequest,
  OffscreenResponse,
  WebSocketConnectionState,
} from '../types/WebSocketTypes';
import { ReconnectionManager } from './ReconnectionManager';

// =========================================================================
// Constants
// =========================================================================

const PORT_RANGE_START = 9001;
const PORT_RANGE_END = 9100;
const PORT_SCAN_CHUNK_SIZE = 10;
const PORT_SCAN_TIMEOUT_MS = 500;
const CACHED_PORT_KEY = 'ws_cached_port';
const CACHED_PORT_TIMESTAMP_KEY = 'ws_cached_port_timestamp';
const CACHE_EXPIRY_MS = 24 * 60 * 60 * 1000; // 24 hours
const HANDSHAKE_TIMEOUT_MS = 1500;

// =========================================================================
// State
// =========================================================================

let ws: WebSocket | null = null;
let currentPort: number | null = null;
let sessionId: string | null = null;
let connectionState: WebSocketConnectionState = 'disconnected';
let handshakeTimer: ReturnType<typeof setTimeout> | null = null;

// Reconnection manager with exponential backoff
const reconnectionManager = new ReconnectionManager();

// =========================================================================
// Port Scanning
// =========================================================================

/**
 * Try to connect to a specific port
 * Returns the WebSocket if successful, null otherwise
 */
async function tryConnectPort(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const testWs = new WebSocket(`ws://127.0.0.1:${port}`);
    const timeout = setTimeout(() => {
      testWs.close();
      resolve(false);
    }, PORT_SCAN_TIMEOUT_MS);

    testWs.onopen = () => {
      clearTimeout(timeout);
      testWs.close();
      resolve(true);
    };

    testWs.onerror = () => {
      clearTimeout(timeout);
      testWs.close();
      resolve(false);
    };
  });
}

/**
 * Scan ports in chunks to find the Tauri WebSocket server
 */
async function scanPorts(): Promise<number | null> {
  // First, try cached port
  const cached = await getCachedPort();
  if (cached) {
    console.log(`[Offscreen] Trying cached port ${cached}...`);
    const cachedOk = await tryConnectPort(cached);
    if (cachedOk) {
      console.log(`[Offscreen] Connected to cached port ${cached}`);
      return cached;
    }
  }

  // Scan in chunks
  console.log(`[Offscreen] Scanning ports ${PORT_RANGE_START}-${PORT_RANGE_END}...`);

  for (let start = PORT_RANGE_START; start <= PORT_RANGE_END; start += PORT_SCAN_CHUNK_SIZE) {
    const end = Math.min(start + PORT_SCAN_CHUNK_SIZE - 1, PORT_RANGE_END);
    const ports = Array.from({ length: end - start + 1 }, (_, i) => start + i);

    // Try all ports in chunk concurrently
    const results = await Promise.all(ports.map(async (port) => ({ ok: await tryConnectPort(port), port })));

    // Find first successful connection
    // Note: Do NOT cache here - cache only after receiving 'connected' message
    // to verify it's actually the Tauri server (not another WebSocket service)
    const success = results.find((r) => r.ok);
    if (success && success.ok) {
      console.log(`[Offscreen] Found server on port ${success.port}`);
      return success.port;
    }
  }

  console.log('[Offscreen] No server found in port range');
  return null;
}

// =========================================================================
// Port Caching
// =========================================================================

async function getCachedPort(): Promise<number | null> {
  if (!chrome.storage?.local) {
    return null;
  }
  return new Promise((resolve) => {
    chrome.storage.local.get([CACHED_PORT_KEY, CACHED_PORT_TIMESTAMP_KEY], (result) => {
      const port = result[CACHED_PORT_KEY];
      const timestamp = result[CACHED_PORT_TIMESTAMP_KEY];

      // Validate cache: must have port, timestamp, and not expired (24h)
      if (port && timestamp && Date.now() - timestamp < CACHE_EXPIRY_MS) {
        resolve(port);
      } else {
        resolve(null);
      }
    });
  });
}

async function setCachedPort(port: number): Promise<void> {
  if (!chrome.storage?.local) {
    return;
  }
  return new Promise((resolve) => {
    chrome.storage.local.set(
      {
        [CACHED_PORT_KEY]: port,
        [CACHED_PORT_TIMESTAMP_KEY]: Date.now(),
      },
      resolve
    );
  });
}

async function clearCachedPort(): Promise<void> {
  if (!chrome.storage?.local) {
    return;
  }
  return new Promise((resolve) => {
    chrome.storage.local.remove([CACHED_PORT_KEY, CACHED_PORT_TIMESTAMP_KEY], resolve);
  });
}

// =========================================================================
// WebSocket Connection
// =========================================================================

function updateState(state: WebSocketConnectionState): void {
  connectionState = state;
  console.log(`[Offscreen] State: ${state}`);
}

function startHandshakeTimer(): void {
  if (handshakeTimer) {
    clearTimeout(handshakeTimer);
  }

  handshakeTimer = setTimeout(() => {
    console.warn('[Offscreen] Handshake timeout: closing connection and rescanning');
    clearCachedPort();
    if (ws) {
      ws.close();
    }
  }, HANDSHAKE_TIMEOUT_MS);
}

function clearHandshakeTimer(): void {
  if (!handshakeTimer) return;
  clearTimeout(handshakeTimer);
  handshakeTimer = null;
}

async function connect(): Promise<void> {
  // Cancel any pending reconnect timer to prevent duplicate connections
  reconnectionManager.cancelReconnect();

  if (ws && ws.readyState === WebSocket.OPEN) {
    console.log('[Offscreen] Already connected');
    return;
  }

  updateState('scanning');

  const port = await scanPorts();
  if (!port) {
    updateState('disconnected');
    sendToBackground({
      type: 'OFFSCREEN_DISCONNECTED',
      reason: 'No Tauri server found',
    });
    scheduleReconnect();
    return;
  }

  updateState('connecting');
  ws = new WebSocket(`ws://127.0.0.1:${port}`);
  currentPort = port;
  startHandshakeTimer();

  ws.onopen = () => {
    console.log(`[Offscreen] WebSocket connected to port ${currentPort}`);
    clearHandshakeTimer();
    updateState('connected');
    sendToBackground({
      type: 'OFFSCREEN_CONNECTED',
      port: currentPort!,
      sessionId: sessionId ?? 'pending',
    });
  };

  ws.onmessage = (event) => {
    try {
      const message = JSON.parse(event.data) as InboundWebSocketMessage;
      handleInboundMessage(message);
    } catch (error) {
      console.error('[Offscreen] Failed to parse message:', error);
    }
  };

  ws.onerror = (error) => {
    console.error('[Offscreen] WebSocket error:', error);
    updateState('error');
    sendToBackground({
      type: 'OFFSCREEN_ERROR',
      message: 'WebSocket error',
    });
  };

  ws.onclose = (event) => {
    console.log(`[Offscreen] WebSocket closed: code=${event.code}, reason=${event.reason}`);
    clearHandshakeTimer();
    ws = null;
    currentPort = null;
    sessionId = null;
    updateState('disconnected');
    sendToBackground({
      type: 'OFFSCREEN_DISCONNECTED',
      reason: event.reason || 'Connection closed',
    });
    scheduleReconnect();
  };
}

function handleInboundMessage(message: InboundWebSocketMessage): void {
  if (message.type === 'connected') {
    sessionId = message.sessionId;
    clearHandshakeTimer();
    updateState('connected');
    // Reset backoff on successful connection
    reconnectionManager.resetAttemptCount();
    // Cache port only after receiving 'connected' message from Tauri server
    // This prevents caching ports of non-Tauri WebSocket services
    if (currentPort) {
      void setCachedPort(currentPort);
    }
    sendToBackground({
      type: 'OFFSCREEN_CONNECTED',
      port: currentPort!,
      sessionId: message.sessionId,
    });
  }

  // Forward all messages to background
  sendToBackground({
    type: 'OFFSCREEN_MESSAGE',
    payload: message,
  });
}

function disconnect(): void {
  reconnectionManager.cancelReconnect();
  clearHandshakeTimer();

  if (ws) {
    ws.close();
    ws = null;
  }

  currentPort = null;
  sessionId = null;
  updateState('disconnected');
}

function send(message: OutboundWebSocketMessage): void {
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    console.warn('[Offscreen] Cannot send: not connected');
    return;
  }

  try {
    ws.send(JSON.stringify(message));
  } catch (error) {
    console.error('[Offscreen] Send error:', error);
  }
}

function scheduleReconnect(): void {
  reconnectionManager.scheduleReconnect(() => connect());
}

// =========================================================================
// Message Handling
// =========================================================================

function sendToBackground(response: OffscreenResponse): void {
  chrome.runtime.sendMessage(response).catch((error) => {
    // Service worker may be sleeping, this is expected
    console.debug('[Offscreen] sendMessage error (SW may be sleeping):', error);
  });
}

chrome.runtime.onMessage.addListener((message: OffscreenRequest, _sender, sendResponse) => {
  console.log('[Offscreen] Received message:', message.type);

  switch (message.type) {
    case 'OFFSCREEN_CONNECT':
      void connect();
      sendResponse({ success: true });
      break;

    case 'OFFSCREEN_DISCONNECT':
      disconnect();
      sendResponse({ success: true });
      break;

    case 'OFFSCREEN_SEND':
      send(message.payload);
      sendResponse({ success: true });
      break;

    case 'OFFSCREEN_STATUS':
      sendResponse({
        state: connectionState,
        port: currentPort,
        sessionId: sessionId,
      });
      break;
  }

  return false;
});

// =========================================================================
// Initialization
// =========================================================================

console.log('[Offscreen] Document loaded');

// Auto-connect on load
connect();
