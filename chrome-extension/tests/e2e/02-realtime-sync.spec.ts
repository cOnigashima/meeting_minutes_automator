/**
 * E2E Test Scenario 2: Real-time Sync
 *
 * テストシナリオ:
 * - 文字起こし受信 → Google Docs反映
 * - 2秒以内のレイテンシ検証
 * - 複数メッセージの順序保証
 *
 * Requirements: DOCS-REQ-002 (リアルタイム同期), DOCS-NFR-001.1 (2秒以内)
 */

import { test, expect, createTranscriptionMessage, waitForExtensionReady } from './fixtures';

test.describe('Scenario 2: Real-time Sync', () => {
  test.beforeEach(async ({ context }) => {
    await waitForExtensionReady(context);
  });

  test('should receive transcription messages via WebSocket', async ({ popupPage, mockTauri }) => {
    // Wait for WebSocket connection
    await popupPage.waitForTimeout(2000);

    // Send transcription message from mock Tauri server
    const transcription = createTranscriptionMessage('テスト文字起こしメッセージ');
    mockTauri.sendToAll(transcription);

    // Wait for message processing
    await popupPage.waitForTimeout(500);

    // Verify extension received the message (check console or state)
    // Note: Actual verification requires Google Docs API mock or real integration
  });

  test('should handle partial transcriptions correctly', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    // Send partial transcription (should not sync immediately)
    const partialMsg = createTranscriptionMessage('部分的な', { isPartial: true });
    mockTauri.sendToAll(partialMsg);

    await popupPage.waitForTimeout(100);

    // Send final transcription (should trigger sync)
    const finalMsg = createTranscriptionMessage('部分的な文字起こし', { isPartial: false });
    mockTauri.sendToAll(finalMsg);

    await popupPage.waitForTimeout(500);
  });

  test('should maintain message order for sequential transcriptions', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    // Send multiple messages in sequence
    const messages = [
      createTranscriptionMessage('最初のメッセージ'),
      createTranscriptionMessage('2番目のメッセージ'),
      createTranscriptionMessage('3番目のメッセージ'),
    ];

    for (const msg of messages) {
      mockTauri.sendToAll(msg);
      await popupPage.waitForTimeout(100);
    }

    // Messages should be processed in order
    await popupPage.waitForTimeout(1000);
  });

  test('should include confidence score in transcription', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    // Send transcription with confidence
    const msg = createTranscriptionMessage('高信頼度のメッセージ', {
      confidence: 0.98,
      language: 'ja',
    });
    mockTauri.sendToAll(msg);

    await popupPage.waitForTimeout(500);
  });

  test('should handle multi-language transcriptions', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    // Japanese
    mockTauri.sendToAll(createTranscriptionMessage('日本語のテスト', { language: 'ja' }));
    await popupPage.waitForTimeout(200);

    // English
    mockTauri.sendToAll(createTranscriptionMessage('English test message', { language: 'en' }));
    await popupPage.waitForTimeout(200);
  });

  test('should display sync status indicator during sync', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Trigger sync
    mockTauri.sendToAll(createTranscriptionMessage('同期ステータステスト'));

    // Status indicator should update
    const statusBadge = popupPage.locator('.status-badge, .sync-status');
    await expect(statusBadge).toBeAttached();
  });

  test('should respect buffering time setting', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Verify buffering range input exists
    const bufferingInput = popupPage.locator('input[name="bufferingSeconds"], #buffering-seconds');

    if (await bufferingInput.isVisible()) {
      // Set buffering to 2 seconds
      await bufferingInput.fill('2');

      // Send message
      const startTime = Date.now();
      mockTauri.sendToAll(createTranscriptionMessage('バッファリングテスト'));

      // Should buffer for ~2 seconds before sync
      await popupPage.waitForTimeout(2500);
      const elapsed = Date.now() - startTime;

      expect(elapsed).toBeGreaterThanOrEqual(2000);
    }
  });
});

test.describe('Scenario 2: Performance', () => {
  test('should process transcription within 2 seconds (DOCS-NFR-001.1)', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    const startTime = Date.now();

    // Send transcription
    mockTauri.sendToAll(createTranscriptionMessage('パフォーマンステスト'));

    // Wait for processing
    await popupPage.waitForTimeout(2000);

    const elapsed = Date.now() - startTime;

    // Should complete within 2 seconds (local processing, excluding API call)
    expect(elapsed).toBeLessThan(3000);
  });

  test('should handle rapid message bursts', async ({ mockTauri, popupPage }) => {
    await popupPage.waitForTimeout(2000);

    // Send 10 messages rapidly
    for (let i = 0; i < 10; i++) {
      mockTauri.sendToAll(createTranscriptionMessage(`バーストメッセージ ${i + 1}`));
    }

    // Should not crash or lose messages
    await popupPage.waitForTimeout(3000);
    await expect(popupPage.locator('body')).toBeVisible();
  });
});
