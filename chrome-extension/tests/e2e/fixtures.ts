/**
 * Playwright Fixtures for Chrome Extension E2E Testing
 *
 * Chrome拡張機能をロードした状態でテストを実行するためのフィクスチャ。
 * - 拡張機能のロード
 * - Service Workerへのアクセス
 * - Popup/Offscreenページへのアクセス
 * - モック用のWebSocketサーバー
 */

import { test as base, chromium, BrowserContext, Page } from '@playwright/test';
import * as path from 'path';
import { WebSocketServer, WebSocket } from 'ws';

const EXTENSION_PATH = path.join(__dirname, '../../dist');

// Mock WebSocket server for Tauri communication simulation
interface MockTauriServer {
  server: WebSocketServer;
  port: number;
  clients: Set<WebSocket>;
  sendToAll: (message: object) => void;
  close: () => Promise<void>;
}

// Extended test context with Chrome extension support
interface ExtensionFixtures {
  context: BrowserContext;
  extensionId: string;
  popupPage: Page;
  backgroundPage: Page | null;
  mockTauri: MockTauriServer;
}

/**
 * Create a mock Tauri WebSocket server
 */
async function createMockTauriServer(port: number = 9001): Promise<MockTauriServer> {
  return new Promise((resolve, reject) => {
    const clients = new Set<WebSocket>();
    const server = new WebSocketServer({ port, host: '127.0.0.1' });

    server.on('listening', () => {
      const mockServer: MockTauriServer = {
        server,
        port,
        clients,
        sendToAll: (message: object) => {
          const json = JSON.stringify(message);
          clients.forEach((client) => {
            if (client.readyState === WebSocket.OPEN) {
              client.send(json);
            }
          });
        },
        close: () =>
          new Promise<void>((res) => {
            clients.forEach((c) => c.close());
            server.close(() => res());
          }),
      };
      resolve(mockServer);
    });

    server.on('connection', (ws) => {
      clients.add(ws);

      // Send connected message (simulating Tauri WebSocket server)
      ws.send(
        JSON.stringify({
          type: 'connected',
          messageId: 'ws-0',
          sessionId: 'test-session-' + Date.now(),
          timestamp: Date.now(),
        })
      );

      ws.on('close', () => {
        clients.delete(ws);
      });
    });

    server.on('error', reject);
  });
}

/**
 * Extended Playwright test with Chrome extension fixtures
 */
export const test = base.extend<ExtensionFixtures>({
  // Create a persistent browser context with extension loaded
  context: async ({}, use) => {
    const context = await chromium.launchPersistentContext('', {
      headless: false, // Extensions require headed mode
      args: [
        `--disable-extensions-except=${EXTENSION_PATH}`,
        `--load-extension=${EXTENSION_PATH}`,
        '--no-first-run',
        '--no-default-browser-check',
      ],
    });

    await use(context);
    await context.close();
  },

  // Get the extension ID from the loaded extension
  extensionId: async ({ context }, use) => {
    // Wait for service worker to be registered
    let extensionId = '';

    // Get extension ID from service worker URL
    const serviceWorkers = context.serviceWorkers();
    for (const sw of serviceWorkers) {
      const url = sw.url();
      if (url.includes('chrome-extension://')) {
        const match = url.match(/chrome-extension:\/\/([^/]+)/);
        if (match) {
          extensionId = match[1];
          break;
        }
      }
    }

    // If not found, wait for new service worker
    if (!extensionId) {
      const sw = await context.waitForEvent('serviceworker');
      const match = sw.url().match(/chrome-extension:\/\/([^/]+)/);
      if (match) {
        extensionId = match[1];
      }
    }

    await use(extensionId);
  },

  // Open the popup page
  popupPage: async ({ context, extensionId }, use) => {
    const popupPage = await context.newPage();
    await popupPage.goto(`chrome-extension://${extensionId}/dist/popup/popup.html`);
    await use(popupPage);
    await popupPage.close();
  },

  // Access the background service worker (if needed)
  backgroundPage: async ({ context }, use) => {
    const serviceWorkers = context.serviceWorkers();
    const bgWorker = serviceWorkers.find((sw) => sw.url().includes('background.js'));
    // Note: Service workers don't have a "page" but can be accessed via evaluate
    await use(null); // Placeholder - use context.serviceWorkers() directly in tests
  },

  // Mock Tauri WebSocket server
  mockTauri: async ({}, use) => {
    const mockServer = await createMockTauriServer(9001);
    await use(mockServer);
    await mockServer.close();
  },
});

export { expect } from '@playwright/test';

/**
 * Helper to wait for extension to be fully loaded
 */
export async function waitForExtensionReady(context: BrowserContext, timeoutMs = 5000): Promise<void> {
  const startTime = Date.now();
  while (Date.now() - startTime < timeoutMs) {
    const workers = context.serviceWorkers();
    if (workers.length > 0) {
      return;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('Extension service worker not ready within timeout');
}

/**
 * Helper to simulate transcription messages from Tauri
 */
export function createTranscriptionMessage(text: string, options?: {
  isPartial?: boolean;
  confidence?: number;
  language?: string;
}): object {
  return {
    type: 'transcription',
    messageId: `msg-${Date.now()}`,
    sessionId: 'test-session',
    text,
    timestamp: Date.now(),
    isPartial: options?.isPartial ?? false,
    confidence: options?.confidence ?? 0.95,
    language: options?.language ?? 'ja',
  };
}

/**
 * Helper to simulate error messages from Tauri
 */
export function createErrorMessage(message: string): object {
  return {
    type: 'error',
    messageId: `err-${Date.now()}`,
    sessionId: 'test-session',
    message,
    timestamp: Date.now(),
  };
}

/**
 * Helper to simulate notification messages from Tauri
 */
export function createNotificationMessage(notificationType: string, message: string, data?: object): object {
  return {
    type: 'notification',
    messageId: `notif-${Date.now()}`,
    sessionId: 'test-session',
    notificationType,
    message,
    timestamp: Date.now(),
    data,
  };
}
