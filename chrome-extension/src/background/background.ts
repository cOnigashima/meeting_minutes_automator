/**
 * Background Service Worker (Manifest V3)
 *
 * 責務:
 * - トークン有効期限監視とバックグラウンドリフレッシュ
 * - Offscreen Documentライフサイクル管理
 * - WebSocket接続状態管理
 *
 * ファイルパス: chrome-extension/src/background/background.ts
 *
 * Implementation: Phase 1-4
 */

import { getAuthManager } from '../auth/AuthFactory';
import type {
  OffscreenResponse,
  WebSocketStatus,
  InboundWebSocketMessage,
} from '../types/WebSocketTypes';

const TOKEN_EXPIRY_ALARM = 'token_expiry_alarm';
const OFFSCREEN_DOCUMENT_PATH = 'dist/offscreen/offscreen.html';

// WebSocket connection state (maintained by offscreen document)
let wsStatus: WebSocketStatus = { state: 'disconnected' };

console.log('Meeting Minutes Automator - Background Service Worker started');

// =========================================================================
// Offscreen Document Management
// =========================================================================

/**
 * Check if offscreen document exists
 */
async function hasOffscreenDocument(): Promise<boolean> {
  const contexts = await chrome.runtime.getContexts({
    contextTypes: [chrome.runtime.ContextType.OFFSCREEN_DOCUMENT],
    documentUrls: [chrome.runtime.getURL(OFFSCREEN_DOCUMENT_PATH)],
  });
  return contexts.length > 0;
}

/**
 * Ensure offscreen document is created
 */
async function ensureOffscreenDocument(): Promise<void> {
  if (await hasOffscreenDocument()) {
    return;
  }

  console.log('[Background] Creating offscreen document...');

  await chrome.offscreen.createDocument({
    url: OFFSCREEN_DOCUMENT_PATH,
    reasons: [chrome.offscreen.Reason.WEB_RTC],
    justification: 'Maintain WebSocket connection to Tauri app for real-time transcription sync',
  });

  console.log('[Background] Offscreen document created');
}

/**
 * Close offscreen document
 */
async function closeOffscreenDocument(): Promise<void> {
  if (!(await hasOffscreenDocument())) {
    return;
  }

  console.log('[Background] Closing offscreen document...');
  await chrome.offscreen.closeDocument();
  wsStatus = { state: 'disconnected' };
}

/**
 * Send message to offscreen document
 */
async function sendToOffscreen<T>(message: Record<string, unknown>): Promise<T | null> {
  try {
    await ensureOffscreenDocument();
    return await chrome.runtime.sendMessage(message) as T;
  } catch (error) {
    console.error('[Background] Failed to send to offscreen:', error);
    return null;
  }
}

/**
 * Connect WebSocket via offscreen document
 */
export async function connectWebSocket(): Promise<void> {
  await sendToOffscreen({ type: 'OFFSCREEN_CONNECT' });
}

/**
 * Disconnect WebSocket
 */
export async function disconnectWebSocket(): Promise<void> {
  await sendToOffscreen({ type: 'OFFSCREEN_DISCONNECT' });
}

/**
 * Get WebSocket status
 */
export function getWebSocketStatus(): WebSocketStatus {
  return { ...wsStatus };
}

/**
 * Handle messages from offscreen document
 */
function handleOffscreenMessage(message: OffscreenResponse): void {
  switch (message.type) {
    case 'OFFSCREEN_CONNECTED':
      wsStatus = {
        state: 'connected',
        port: message.port,
        sessionId: message.sessionId,
        lastConnectedAt: Date.now(),
      };
      console.log(`[Background] WebSocket connected: port=${message.port}, session=${message.sessionId}`);
      // Clear any error badge
      chrome.action.setBadgeText({ text: '' });
      break;

    case 'OFFSCREEN_DISCONNECTED':
      wsStatus = {
        state: 'disconnected',
        lastError: message.reason,
      };
      console.log(`[Background] WebSocket disconnected: ${message.reason}`);
      break;

    case 'OFFSCREEN_MESSAGE':
      handleWebSocketMessage(message.payload);
      break;

    case 'OFFSCREEN_ERROR':
      wsStatus = {
        state: 'error',
        lastError: message.message,
      };
      console.error(`[Background] WebSocket error: ${message.message}`);
      // Show error badge
      chrome.action.setBadgeText({ text: '!' });
      chrome.action.setBadgeBackgroundColor({ color: '#EF4444' });
      break;
  }
}

/**
 * Handle WebSocket messages from Tauri
 */
function handleWebSocketMessage(message: InboundWebSocketMessage): void {
  console.log(`[Background] WebSocket message: type=${message.type}`);

  switch (message.type) {
    case 'transcription':
      // TODO: Phase 5 - Forward to SyncManager for Google Docs sync
      console.log(`[Background] Transcription: ${message.text.substring(0, 50)}...`);
      break;

    case 'notification':
      // Show Chrome notification for important events
      if (message.notificationType === 'model_changed') {
        chrome.notifications.create({
          type: 'basic',
          iconUrl: 'icon128.png',
          title: 'Meeting Minutes Automator',
          message: message.message,
        });
      }
      break;

    case 'error':
      console.error(`[Background] Tauri error: ${message.message}`);
      break;
  }
}

/**
 * Handle extension installation
 */
chrome.runtime.onInstalled.addListener(async () => {
  console.log('[Background] Extension installed');
  // Auto-connect WebSocket on install
  await ensureOffscreenDocument();
});

/**
 * Handle extension startup (browser restart)
 */
chrome.runtime.onStartup.addListener(async () => {
  console.log('[Background] Extension started');
  // Auto-connect WebSocket on startup
  await ensureOffscreenDocument();
});

/**
 * Handle token expiry alarm
 *
 * アラーム発火時にアクセストークンをリフレッシュする。
 * getAccessToken()は期限切れ検出時に自動的にリフレッシュを行う。
 */
chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name !== TOKEN_EXPIRY_ALARM) {
    return;
  }

  console.log('[TokenRefresh] Alarm triggered, refreshing access token...');

  try {
    const authManager = getAuthManager();
    const result = await authManager.getAccessToken();

    if (result.ok) {
      console.log('[TokenRefresh] Token refreshed successfully');
    } else {
      console.warn('[TokenRefresh] Token refresh failed:', result.error.type, result.error.message);

      // RefreshRequired の場合は再認証が必要（ユーザーアクション待ち）
      if (result.error.type === 'RefreshRequired') {
        // バッジで通知（オプション）
        chrome.action.setBadgeText({ text: '!' });
        chrome.action.setBadgeBackgroundColor({ color: '#EF4444' });
      }
    }
  } catch (error) {
    console.error('[TokenRefresh] Unexpected error:', error);
  }
});

/**
 * Handle messages from content scripts / popup / offscreen document
 */
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log('[Background] Message received:', message.type);

  // Handle offscreen document messages
  if (message.type?.startsWith('OFFSCREEN_')) {
    handleOffscreenMessage(message as OffscreenResponse);
    sendResponse({ received: true });
    return false;
  }

  // Clear badge when user opens popup (indicating they've seen the notification)
  if (message.type === 'POPUP_OPENED') {
    chrome.action.setBadgeText({ text: '' });
    sendResponse({ received: true });
    return false;
  }

  // Get WebSocket status
  if (message.type === 'GET_WS_STATUS') {
    sendResponse(getWebSocketStatus());
    return false;
  }

  // Connect WebSocket
  if (message.type === 'CONNECT_WS') {
    connectWebSocket().then(() => sendResponse({ success: true }));
    return true; // Async response
  }

  // Disconnect WebSocket
  if (message.type === 'DISCONNECT_WS') {
    disconnectWebSocket().then(() => sendResponse({ success: true }));
    return true; // Async response
  }

  return false;
});
