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

// =========================================================================
// Constants
// =========================================================================

const PORT_RANGE_START = 9001;
const PORT_RANGE_END = 9100;
const PORT_SCAN_CHUNK_SIZE = 10;
const PORT_SCAN_TIMEOUT_MS = 500;
const RECONNECT_DELAY_MS = 3000;
const CACHED_PORT_KEY = 'ws_cached_port';
const HANDSHAKE_TIMEOUT_MS = 1500;

// =========================================================================
// State
// =========================================================================

let ws: WebSocket | null = null;
let currentPort: number | null = null;
let sessionId: string | null = null;
let connectionState: WebSocketConnectionState = 'disconnected';
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let handshakeTimer: ReturnType<typeof setTimeout> | null = null;

// =========================================================================
// Port Scanning
// =========================================================================

/**
 * Try to connect to a specific port
 * Returns the WebSocket if successful, null otherwise
 */
async function tryConnectPort(port: number): Promise<WebSocket | null> {
  return new Promise((resolve) => {
    const testWs = new WebSocket(`ws://127.0.0.1:${port}`);
    const timeout = setTimeout(() => {
      testWs.close();
      resolve(null);
    }, PORT_SCAN_TIMEOUT_MS);

    testWs.onopen = () => {
      clearTimeout(timeout);
      resolve(testWs);
    };

    testWs.onerror = () => {
      clearTimeout(timeout);
      testWs.close();
      resolve(null);
    };
  });
}

/**
 * Scan ports in chunks to find the Tauri WebSocket server
 */
async function scanPorts(): Promise<{ ws: WebSocket; port: number } | null> {
  // First, try cached port
  const cached = await getCachedPort();
  if (cached) {
    console.log(`[Offscreen] Trying cached port ${cached}...`);
    const cachedWs = await tryConnectPort(cached);
    if (cachedWs) {
      console.log(`[Offscreen] Connected to cached port ${cached}`);
      return { ws: cachedWs, port: cached };
    }
  }

  // Scan in chunks
  console.log(`[Offscreen] Scanning ports ${PORT_RANGE_START}-${PORT_RANGE_END}...`);

  for (let start = PORT_RANGE_START; start <= PORT_RANGE_END; start += PORT_SCAN_CHUNK_SIZE) {
    const end = Math.min(start + PORT_SCAN_CHUNK_SIZE - 1, PORT_RANGE_END);
    const ports = Array.from({ length: end - start + 1 }, (_, i) => start + i);

    // Try all ports in chunk concurrently
    const results = await Promise.all(ports.map((port) => tryConnectPort(port).then((ws) => ({ ws, port }))));

    // Find first successful connection
    const success = results.find((r) => r.ws !== null);
    if (success && success.ws) {
      console.log(`[Offscreen] Found server on port ${success.port}`);
      await setCachedPort(success.port);
      return { ws: success.ws, port: success.port };
    }
  }

  console.log('[Offscreen] No server found in port range');
  return null;
}

// =========================================================================
// Port Caching
// =========================================================================

async function getCachedPort(): Promise<number | null> {
  return new Promise((resolve) => {
    chrome.storage.local.get(CACHED_PORT_KEY, (result) => {
      resolve(result[CACHED_PORT_KEY] ?? null);
    });
  });
}

async function setCachedPort(port: number): Promise<void> {
  return new Promise((resolve) => {
    chrome.storage.local.set({ [CACHED_PORT_KEY]: port }, resolve);
  });
}

async function clearCachedPort(): Promise<void> {
  return new Promise((resolve) => {
    chrome.storage.local.remove(CACHED_PORT_KEY, resolve);
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
  if (ws && ws.readyState === WebSocket.OPEN) {
    console.log('[Offscreen] Already connected');
    return;
  }

  updateState('scanning');

  const result = await scanPorts();
  if (!result) {
    updateState('error');
    sendToBackground({
      type: 'OFFSCREEN_ERROR',
      message: 'No Tauri server found',
    });
    scheduleReconnect();
    return;
  }

  updateState('connecting');
  ws = result.ws;
  currentPort = result.port;
  startHandshakeTimer();

  ws.onopen = () => {
    console.log(`[Offscreen] WebSocket connected to port ${currentPort}`);
    // Wait for 'connected' message to get sessionId
    startHandshakeTimer();
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
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
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
  if (reconnectTimer) {
    return;
  }

  console.log(`[Offscreen] Scheduling reconnect in ${RECONNECT_DELAY_MS}ms`);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, RECONNECT_DELAY_MS);
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
      connect().then(() => sendResponse({ success: true }));
      return true; // Async response

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
