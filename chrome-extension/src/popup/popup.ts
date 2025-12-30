/**
 * Popup Script
 *
 * 責務: Popup UIの状態管理と認証フロー制御
 * ファイルパス: chrome-extension/src/popup/popup.ts
 *
 * Implementation: Phase 1, Phase 5 (Settings UI)
 */

import { getAuthManager } from '../auth/AuthFactory';
import type { IAuthManager } from '../auth/IAuthManager';
import { runIntegrationTest } from './integration-test';
import { getSettingsManager } from '../sync/SettingsManager';
import type { DocsSyncSettings } from '../types/SyncTypes';

// =========================================================================
// DOM Elements
// =========================================================================

interface PopupElements {
  authBadge: HTMLElement;
  authStatusText: HTMLElement;
  authButton: HTMLButtonElement;
  disconnectButton: HTMLButtonElement;
  authGuide: HTMLElement;
  notification: HTMLElement;
  status: HTMLElement;
  testSection: HTMLElement;
  documentIdInput: HTMLInputElement;
  testButton: HTMLButtonElement;
  // Settings elements (Phase 5)
  settingsSection: HTMLElement;
  settingEnabled: HTMLInputElement;
  settingTimestamp: HTMLInputElement;
  settingSpeaker: HTMLInputElement;
  settingBuffering: HTMLInputElement;
  bufferingValue: HTMLElement;
}

function getElements(): PopupElements {
  return {
    authBadge: document.getElementById('authBadge')!,
    authStatusText: document.getElementById('authStatusText')!,
    authButton: document.getElementById('authButton') as HTMLButtonElement,
    disconnectButton: document.getElementById('disconnectButton') as HTMLButtonElement,
    authGuide: document.getElementById('authGuide')!,
    notification: document.getElementById('notification')!,
    status: document.getElementById('status')!,
    testSection: document.getElementById('testSection')!,
    documentIdInput: document.getElementById('documentIdInput') as HTMLInputElement,
    testButton: document.getElementById('testButton') as HTMLButtonElement,
    // Settings elements (Phase 5)
    settingsSection: document.getElementById('settingsSection')!,
    settingEnabled: document.getElementById('settingEnabled') as HTMLInputElement,
    settingTimestamp: document.getElementById('settingTimestamp') as HTMLInputElement,
    settingSpeaker: document.getElementById('settingSpeaker') as HTMLInputElement,
    settingBuffering: document.getElementById('settingBuffering') as HTMLInputElement,
    bufferingValue: document.getElementById('bufferingValue')!,
  };
}

// =========================================================================
// UI State Management
// =========================================================================

type AuthState = 'unauthenticated' | 'authenticated' | 'loading' | 'error';

function updateAuthUI(elements: PopupElements, state: AuthState, errorMessage?: string): void {
  const { authBadge, authStatusText, authButton, disconnectButton, authGuide, testSection, settingsSection } = elements;

  // Reset classes
  authBadge.className = 'status-badge';
  authStatusText.className = 'auth-status-text';

  switch (state) {
    case 'unauthenticated':
      authStatusText.textContent = 'Google Docs: 未連携';
      authButton.classList.remove('hidden');
      disconnectButton.classList.add('hidden');
      authGuide.classList.add('show');
      authButton.disabled = false;
      testSection.classList.add('hidden');
      settingsSection.classList.add('hidden');
      break;

    case 'authenticated':
      authBadge.classList.add('authenticated');
      authStatusText.classList.add('authenticated');
      authStatusText.textContent = 'Google Docs: 連携済み';
      authButton.classList.add('hidden');
      disconnectButton.classList.remove('hidden');
      authGuide.classList.remove('show');
      disconnectButton.disabled = false;
      testSection.classList.remove('hidden');
      settingsSection.classList.remove('hidden');
      break;

    case 'loading':
      authBadge.classList.add('loading');
      authStatusText.textContent = 'Google Docs: 認証中...';
      authButton.disabled = true;
      disconnectButton.disabled = true;
      break;

    case 'error':
      authBadge.classList.add('error');
      authStatusText.classList.add('error');
      authStatusText.textContent = `Google Docs: エラー`;
      authButton.classList.remove('hidden');
      disconnectButton.classList.add('hidden');
      authGuide.classList.add('show');
      authButton.disabled = false;
      testSection.classList.add('hidden');
      settingsSection.classList.add('hidden');
      if (errorMessage) {
        showNotification(elements, 'error', errorMessage);
      }
      break;
  }
}

function showNotification(
  elements: PopupElements,
  type: 'success' | 'error' | 'info',
  message: string
): void {
  const { notification } = elements;
  notification.className = `notification show ${type}`;
  notification.textContent = message;

  // Auto-hide after 5 seconds
  setTimeout(() => {
    notification.classList.remove('show');
  }, 5000);
}

// =========================================================================
// Auth Handlers
// =========================================================================

async function handleConnect(authManager: IAuthManager, elements: PopupElements): Promise<void> {
  updateAuthUI(elements, 'loading');
  showNotification(elements, 'info', 'Googleアカウントでログインしてください...');

  const result = await authManager.initiateAuth();

  if (result.ok) {
    updateAuthUI(elements, 'authenticated');
    showNotification(elements, 'success', 'Google Docsとの連携が完了しました');
  } else {
    const error = result.error;
    let message: string;

    switch (error.type) {
      case 'UserCancelled':
        message = '認証がキャンセルされました';
        break;
      case 'NetworkError':
        message = `ネットワークエラー: ${error.message}`;
        break;
      case 'InvalidGrant':
        message = '認証に失敗しました。再度お試しください';
        break;
      default:
        message = '認証エラーが発生しました';
    }

    updateAuthUI(elements, 'error', message);
  }
}

async function handleDisconnect(authManager: IAuthManager, elements: PopupElements): Promise<void> {
  updateAuthUI(elements, 'loading');

  const result = await authManager.revokeToken();

  if (result.ok) {
    updateAuthUI(elements, 'unauthenticated');
    showNotification(elements, 'info', '連携を解除しました');
  } else {
    // Revoke失敗してもローカルは削除されているので、UIは未認証に
    updateAuthUI(elements, 'unauthenticated');
    showNotification(elements, 'info', '連携を解除しました');
  }
}

// =========================================================================
// Settings Management (Phase 5)
// =========================================================================

/**
 * Load settings and update UI
 */
async function loadSettings(elements: PopupElements): Promise<void> {
  const settingsManager = getSettingsManager();
  const result = await settingsManager.getSettings();

  if (result.ok) {
    updateSettingsUI(elements, result.value);
  } else {
    console.error('Failed to load settings:', result.error);
  }
}

/**
 * Update settings UI from settings object
 */
function updateSettingsUI(elements: PopupElements, settings: DocsSyncSettings): void {
  elements.settingEnabled.checked = settings.enabled;
  elements.settingTimestamp.checked = settings.showTimestamp;
  elements.settingSpeaker.checked = settings.showSpeaker;
  elements.settingBuffering.value = settings.bufferingSeconds.toString();
  elements.bufferingValue.textContent = `${settings.bufferingSeconds}秒`;
}

/**
 * Save a single setting change
 */
async function handleSettingChange(
  key: keyof DocsSyncSettings,
  value: boolean | number,
  elements: PopupElements
): Promise<void> {
  const settingsManager = getSettingsManager();
  const result = await settingsManager.updateSettings({ [key]: value });

  if (result.ok) {
    console.log(`[Settings] Updated ${key}:`, value);
  } else {
    console.error(`[Settings] Failed to update ${key}:`, result.error);
    showNotification(elements, 'error', '設定の保存に失敗しました');
    // Reload settings to restore UI to correct state
    await loadSettings(elements);
  }
}

/**
 * Setup settings event listeners
 */
function setupSettingsListeners(elements: PopupElements): void {
  // Enable/Disable sync
  elements.settingEnabled.addEventListener('change', () => {
    handleSettingChange('enabled', elements.settingEnabled.checked, elements);
  });

  // Timestamp toggle
  elements.settingTimestamp.addEventListener('change', () => {
    handleSettingChange('showTimestamp', elements.settingTimestamp.checked, elements);
  });

  // Speaker toggle
  elements.settingSpeaker.addEventListener('change', () => {
    handleSettingChange('showSpeaker', elements.settingSpeaker.checked, elements);
  });

  // Buffering time slider
  elements.settingBuffering.addEventListener('input', () => {
    const value = parseInt(elements.settingBuffering.value, 10);
    elements.bufferingValue.textContent = `${value}秒`;
  });

  elements.settingBuffering.addEventListener('change', () => {
    const value = parseInt(elements.settingBuffering.value, 10);
    handleSettingChange('bufferingSeconds', value, elements);
  });
}

// =========================================================================
// Initialization
// =========================================================================

async function checkAuthStatus(authManager: IAuthManager, elements: PopupElements): Promise<void> {
  // AuthManagerのisAuthenticatedを使用（実装追加が必要）
  const tokenResult = await authManager.getAccessToken();

  if (tokenResult.ok) {
    updateAuthUI(elements, 'authenticated');
  } else {
    updateAuthUI(elements, 'unauthenticated');
  }
}

async function initPopup(): Promise<void> {
  console.log('Meeting Minutes Automator - Popup loaded');

  const elements = getElements();
  const authManager = getAuthManager();

  // Check initial auth status
  await checkAuthStatus(authManager, elements);

  // Load settings (Phase 5)
  await loadSettings(elements);

  // Setup event listeners
  elements.authButton.addEventListener('click', () => {
    handleConnect(authManager, elements);
  });

  elements.disconnectButton.addEventListener('click', () => {
    handleDisconnect(authManager, elements);
  });

  // Integration Test button
  elements.testButton.addEventListener('click', () => {
    const documentId = elements.documentIdInput.value.trim();
    if (!documentId) {
      showNotification(elements, 'error', 'Document IDを入力してください');
      return;
    }
    showNotification(elements, 'info', 'Integration Test実行中... (DevTools Consoleを確認)');
    runIntegrationTest(documentId);
  });

  // Settings event listeners (Phase 5)
  setupSettingsListeners(elements);

  // MVP0: Tauri connection status (placeholder)
  elements.status.textContent = 'Skeleton Ready';
}

// Run on DOM ready
document.addEventListener('DOMContentLoaded', initPopup);
