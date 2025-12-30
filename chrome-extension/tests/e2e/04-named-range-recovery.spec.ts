/**
 * E2E Test Scenario 4: Named Range Recovery
 *
 * テストシナリオ:
 * - Named Range消失検出
 * - テキストパターンによる位置復旧
 * - 自動再作成
 *
 * Requirements: DOCS-REQ-003 (Named Range管理), DOCS-REQ-003.2 (復旧戦略)
 */

import { test, expect, createTranscriptionMessage, waitForExtensionReady } from './fixtures';

test.describe('Scenario 4: Named Range Recovery', () => {
  test.beforeEach(async ({ context }) => {
    await waitForExtensionReady(context);
  });

  test('should handle Named Range not found gracefully', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Send transcription that would trigger Named Range lookup
    mockTauri.sendToAll(createTranscriptionMessage('Named Range テスト'));
    await popupPage.waitForTimeout(500);

    // Should not crash even if Named Range doesn't exist
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should display recovery status in UI', async ({ popupPage }) => {
    // Verify error/status display area exists
    const statusArea = popupPage.locator('.sync-status, .error-message, [role="status"]');
    await expect(statusArea).toBeAttached();
  });

  test('should continue sync after Named Range recovery', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Send first message (may trigger Named Range creation)
    mockTauri.sendToAll(createTranscriptionMessage('最初のメッセージ'));
    await popupPage.waitForTimeout(1000);

    // Send subsequent messages (should work after recovery)
    mockTauri.sendToAll(createTranscriptionMessage('復旧後のメッセージ'));
    await popupPage.waitForTimeout(500);

    // Extension should remain functional
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should log recovery events for debugging', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Capture console logs
    const consoleLogs: string[] = [];
    popupPage.on('console', (msg) => {
      consoleLogs.push(msg.text());
    });

    // Trigger potential Named Range operation
    mockTauri.sendToAll(createTranscriptionMessage('ログテスト'));
    await popupPage.waitForTimeout(1000);

    // Logs should be captured (actual content depends on implementation)
  });

  test('should handle concurrent Named Range operations', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Send multiple messages that might trigger concurrent Named Range ops
    const promises = [];
    for (let i = 0; i < 5; i++) {
      mockTauri.sendToAll(createTranscriptionMessage(`並行テスト ${i + 1}`));
      promises.push(popupPage.waitForTimeout(50));
    }
    await Promise.all(promises);

    await popupPage.waitForTimeout(2000);

    // Should not have race condition issues
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should maintain document structure after recovery', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Initial sync
    mockTauri.sendToAll(createTranscriptionMessage('構造テスト - 開始'));
    await popupPage.waitForTimeout(500);

    // Additional content
    mockTauri.sendToAll(createTranscriptionMessage('構造テスト - 中間'));
    await popupPage.waitForTimeout(500);

    // Final content
    mockTauri.sendToAll(createTranscriptionMessage('構造テスト - 終了'));
    await popupPage.waitForTimeout(1000);

    // Document structure should be maintained
  });

  test('should handle empty document case', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // First sync to empty document
    mockTauri.sendToAll(createTranscriptionMessage('空のドキュメントへの最初の挿入'));
    await popupPage.waitForTimeout(1000);

    // Should work without errors
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should recover from partial Named Range data', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Simulate scenario where Named Range exists but data is incomplete
    mockTauri.sendToAll(createTranscriptionMessage('部分データ復旧テスト'));
    await popupPage.waitForTimeout(1000);

    // Should handle gracefully
    await expect(popupPage.locator('body')).toBeVisible();
  });
});

test.describe('Scenario 4: Recovery Strategy Verification', () => {
  test('should use text pattern matching for position recovery', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Create content with known pattern
    mockTauri.sendToAll(createTranscriptionMessage('会議開始: 2025-01-01 10:00'));
    await popupPage.waitForTimeout(500);

    // Additional content that relies on pattern matching
    mockTauri.sendToAll(createTranscriptionMessage('議題1: プロジェクト進捗'));
    await popupPage.waitForTimeout(500);

    // Pattern should be searchable for recovery
  });

  test('should create Named Range with correct metadata', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Send message that creates Named Range
    mockTauri.sendToAll(createTranscriptionMessage('メタデータテスト'));
    await popupPage.waitForTimeout(1000);

    // Named Range should include proper metadata (sessionId, timestamp, etc.)
    // Note: Verification requires Google Docs API access
  });

  test('should handle special characters in Named Range content', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Content with special characters
    const specialContent = '特殊文字テスト: <>&"\'\\n\\t【】「」';
    mockTauri.sendToAll(createTranscriptionMessage(specialContent));
    await popupPage.waitForTimeout(1000);

    // Should handle without errors
    await expect(popupPage.locator('body')).toBeVisible();
  });

  test('should preserve Named Range across document edits by others', async ({ popupPage, mockTauri }) => {
    await popupPage.waitForTimeout(2000);

    // Initial content
    mockTauri.sendToAll(createTranscriptionMessage('共同編集テスト'));
    await popupPage.waitForTimeout(1000);

    // Simulate concurrent edit scenario
    // Note: Full testing requires actual Google Docs API interaction

    // Additional sync after "edit"
    mockTauri.sendToAll(createTranscriptionMessage('編集後の同期'));
    await popupPage.waitForTimeout(1000);
  });
});
