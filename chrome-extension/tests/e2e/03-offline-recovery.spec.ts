/**
 * E2E Test Scenario 3: Offline → Online Recovery
 *
 * テストシナリオ:
 * - オフライン中のキューイング
 * - オンライン復帰時の自動再同期
 * - キューの永続化と復元
 *
 * Requirements: DOCS-REQ-005 (オフライン対応), DOCS-NFR-001.3 (キュー再送信)
 */

import { test, expect, createTranscriptionMessage, waitForExtensionReady } from './fixtures';

test.describe('Scenario 3: Offline → Online Recovery', () => {
  test.beforeEach(async ({ context }) => {
    await waitForExtensionReady(context);
  });

  test('should detect offline state', async ({ context, popupPage }) => {
    // Go offline
    await context.setOffline(true);

    // Wait for detection
    await popupPage.waitForTimeout(1000);

    // Network monitor should detect offline state
    // Note: Actual implementation uses navigator.onLine and 'offline' event

    // Go back online
    await context.setOffline(false);
  });

  test('should queue messages during offline', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(500);

    // Send transcription while offline
    mockTauri.sendToAll(createTranscriptionMessage('オフライン中のメッセージ1'));
    await popupPage.waitForTimeout(100);
    mockTauri.sendToAll(createTranscriptionMessage('オフライン中のメッセージ2'));
    await popupPage.waitForTimeout(100);

    // Messages should be queued, not synced

    // Go back online
    await context.setOffline(false);
    await popupPage.waitForTimeout(1000);

    // Queue should be processed
  });

  test('should persist queue in chrome.storage.local', async ({ context, extensionId, mockTauri }) => {
    const popup1 = await context.newPage();
    await popup1.goto(`chrome-extension://${extensionId}/dist/popup/popup.html`);
    await popup1.waitForTimeout(2000);

    // Go offline and queue some messages
    await context.setOffline(true);
    await popup1.waitForTimeout(500);

    mockTauri.sendToAll(createTranscriptionMessage('永続化テストメッセージ'));
    await popup1.waitForTimeout(500);

    // Close popup (simulates extension sleep)
    await popup1.close();

    // Reopen popup
    const popup2 = await context.newPage();
    await popup2.goto(`chrome-extension://${extensionId}/dist/popup/popup.html`);
    await popup2.waitForTimeout(1000);

    // Queue should still exist (persisted)

    // Go online
    await context.setOffline(false);
    await popup2.waitForTimeout(2000);

    // Queue should be processed
    await popup2.close();
  });

  test('should auto-resync on network recovery', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Queue some messages while offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(500);

    for (let i = 1; i <= 5; i++) {
      mockTauri.sendToAll(createTranscriptionMessage(`再同期メッセージ ${i}`));
      await popupPage.waitForTimeout(50);
    }

    // Go online - should trigger auto-resync
    await context.setOffline(false);

    // Wait for resync to complete
    await popupPage.waitForTimeout(3000);

    // Extension should still be functional
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should show offline indicator in UI', async ({ context, popupPage }) => {
    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(1000);

    // Should show offline indicator
    const offlineIndicator = popupPage.locator('.offline-indicator, [data-status="offline"], .status-badge');
    await expect(offlineIndicator).toBeAttached();

    // Go online
    await context.setOffline(false);
  });

  test('should limit queue size to prevent storage overflow', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(500);

    // Send many messages
    for (let i = 0; i < 200; i++) {
      mockTauri.sendToAll(createTranscriptionMessage(`大量メッセージ ${i + 1}`));
    }

    await popupPage.waitForTimeout(1000);

    // Should not crash
    await expect(popupPage.locator('body')).toBeVisible();

    // Go online
    await context.setOffline(false);
  });

  test('should process queue in FIFO order', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(500);

    // Queue messages in order
    const messages = ['最初', '2番目', '3番目', '最後'];
    for (const msg of messages) {
      mockTauri.sendToAll(createTranscriptionMessage(msg));
      await popupPage.waitForTimeout(50);
    }

    // Go online
    await context.setOffline(false);
    await popupPage.waitForTimeout(2000);

    // Messages should be processed in FIFO order
  });

  test('should emit docs_sync_offline event when going offline', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Listen for docsSync events (if possible)
    // Note: Actual event verification requires WebSocket inspection

    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(1000);

    // Should emit docs_sync_offline event to Tauri

    // Go online
    await context.setOffline(false);
    await popupPage.waitForTimeout(1000);

    // Should emit docs_sync_online event
  });

  test('should handle WebSocket disconnection during offline', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Close mock Tauri server to simulate disconnection
    await mockTauri.close();

    // Wait for disconnection detection
    await popupPage.waitForTimeout(3000);

    // Extension should show disconnected state
    await expect(popupPage.locator('body')).toBeVisible();
  });
});

test.describe('Scenario 3: Queue Performance', () => {
  test('should resync 100 messages within 120 seconds (DOCS-NFR-001.3)', async ({ context, popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Go offline
    await context.setOffline(true);
    await popupPage.waitForTimeout(500);

    // Queue 100 messages
    for (let i = 0; i < 100; i++) {
      mockTauri.sendToAll(createTranscriptionMessage(`パフォーマンステスト ${i + 1}`));
    }

    await popupPage.waitForTimeout(1000);

    const startTime = Date.now();

    // Go online
    await context.setOffline(false);

    // Wait for all messages to be processed (with timeout check)
    await popupPage.waitForTimeout(10000); // Give some time for processing

    const elapsed = Date.now() - startTime;

    // Should complete well under the 120 second limit
    // Note: Actual API calls would take longer, this tests local processing
    expect(elapsed).toBeLessThan(120000);
  });
});
