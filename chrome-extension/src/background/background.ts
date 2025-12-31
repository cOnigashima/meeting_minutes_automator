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
import { GoogleDocsClient } from '../api/GoogleDocsClient';
import { NamedRangeManager } from '../api/NamedRangeManager';
import { NamedRangeRecoveryStrategy } from '../api/NamedRangeRecoveryStrategy';
import { QueueManager } from '../sync/QueueManager';
import { TokenBucketRateLimiter } from '../sync/TokenBucketRateLimiter';
import { NetworkMonitor } from '../sync/NetworkMonitor';
import { SyncStateMachine } from '../sync/SyncStateMachine';
import { SyncManager } from '../sync/SyncManager';
import { getSettingsManager } from '../sync/SettingsManager';
import type {
  OffscreenResponse,
  WebSocketStatus,
  InboundWebSocketMessage,
  DocsSyncEventType,
  DocsSyncEvent,
} from '../types/WebSocketTypes';
import type { TranscriptionMessage as SyncTranscriptionMessage } from '../types/SyncTypes';

const TOKEN_EXPIRY_ALARM = 'token_expiry_alarm';
const OFFSCREEN_DOCUMENT_PATH = 'dist/offscreen/offscreen.html';

// WebSocket connection state (maintained by offscreen document)
let wsStatus: WebSocketStatus = { state: 'disconnected' };

// Docs sync dependencies (Phase 4 bridge)
const authManager = getAuthManager();
const docsClient = new GoogleDocsClient(authManager);
const recoveryStrategy = new NamedRangeRecoveryStrategy(authManager);
const namedRangeManager = new NamedRangeManager(docsClient, recoveryStrategy);
const queueManager = new QueueManager();
const rateLimiter = new TokenBucketRateLimiter();
const networkMonitor = new NetworkMonitor();
const stateMachine = new SyncStateMachine();
const syncManager = new SyncManager(
  docsClient,
  namedRangeManager,
  queueManager,
  rateLimiter,
  networkMonitor,
  stateMachine
);
const settingsManager = getSettingsManager();
let activeDocumentId: string | null = null;

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

async function sendDocsSyncEvent(
  event: DocsSyncEventType,
  payload: {
    documentId?: string;
    queueSize?: number;
    errorMessage?: string;
  } = {}
): Promise<void> {
  const message: DocsSyncEvent = {
    type: 'docsSync',
    event,
    documentId: payload.documentId,
    queueSize: payload.queueSize,
    errorMessage: payload.errorMessage,
    timestamp: Date.now(),
  };

  await sendToOffscreen({ type: 'OFFSCREEN_SEND', payload: message });
}

async function ensureDocsSyncInitialized(): Promise<boolean> {
  const settingsResult = await settingsManager.getSettings();
  if (!settingsResult.ok) {
    await sendDocsSyncEvent('docs_sync_error', { errorMessage: settingsResult.error.message });
    return false;
  }

  const settings = settingsResult.value;
  if (!settings.enabled) {
    return false;
  }

  if (!settings.documentId) {
    await sendDocsSyncEvent('docs_sync_error', { errorMessage: 'Document ID is not configured' });
    return false;
  }

  if (activeDocumentId !== settings.documentId) {
    const startResult = await syncManager.startSync(settings.documentId);
    if (!startResult.ok) {
      await sendDocsSyncEvent('docs_sync_error', { errorMessage: startResult.error.message });
      return false;
    }
    activeDocumentId = settings.documentId;
    await sendDocsSyncEvent('docs_sync_started', { documentId: settings.documentId });
  }

  return true;
}

async function reportQueueSize(documentId?: string): Promise<void> {
  const sizeResult = await queueManager.size();
  if (!sizeResult.ok) {
    return;
  }
  await sendDocsSyncEvent('docs_sync_queue_update', {
    documentId,
    queueSize: sizeResult.value,
  });
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

async function refreshWebSocketStatus(): Promise<WebSocketStatus> {
  const offscreenStatus = await sendToOffscreen<{
    state: WebSocketStatus['state'];
    port?: number;
    sessionId?: string;
  } | null>({ type: 'OFFSCREEN_STATUS' });

  if (offscreenStatus) {
    wsStatus = {
      ...wsStatus,
      ...offscreenStatus,
      lastConnectedAt:
        offscreenStatus.state === 'connected' ? Date.now() : wsStatus.lastConnectedAt,
    };
  }

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
      void handleTranscriptionMessage(message);
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

async function handleTranscriptionMessage(message: Extract<InboundWebSocketMessage, { type: 'transcription' }>): Promise<void> {
  if (message.isPartial) {
    return;
  }

  const initialized = await ensureDocsSyncInitialized();
  if (!initialized || !activeDocumentId) {
    return;
  }

  const syncMessage: SyncTranscriptionMessage = {
    text: message.text,
    timestamp: message.timestamp,
    isPartial: Boolean(message.isPartial),
    confidence: message.confidence,
    language: message.language,
  };

  const result = await syncManager.processTranscription(syncMessage);
  if (result.ok) {
    await sendDocsSyncEvent('docs_sync_success', { documentId: activeDocumentId });
    await reportQueueSize(activeDocumentId);
    return;
  }

  if (result.error.type === 'QueueError') {
    await sendDocsSyncEvent('docs_sync_offline', { documentId: activeDocumentId });
    await reportQueueSize(activeDocumentId);
    return;
  }

  await sendDocsSyncEvent('docs_sync_error', {
    documentId: activeDocumentId,
    errorMessage: result.error.message,
  });
  await reportQueueSize(activeDocumentId);
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
    refreshWebSocketStatus().then(sendResponse);
    return true;
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
