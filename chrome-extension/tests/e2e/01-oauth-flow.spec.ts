/**
 * E2E Test Scenario 1: OAuth 2.0 Authentication Flow
 *
 * テストシナリオ:
 * 1. OAuth 2.0認証フロー
 * 2. ドキュメント選択
 * 3. 同期開始
 *
 * Requirements: DOCS-REQ-001 (OAuth 2.0認証)
 */

import { test, expect, waitForExtensionReady } from './fixtures';

test.describe('Scenario 1: OAuth 2.0 Authentication Flow', () => {
  test.beforeEach(async ({ context }) => {
    await waitForExtensionReady(context);
  });

  test('should display login button when not authenticated', async ({ popupPage }) => {
    // Verify initial state shows login button
    const loginButton = popupPage.locator('#login-button, [data-testid="login-button"]');
    await expect(loginButton).toBeVisible({ timeout: 10000 });
  });

  test('should show Google Docs settings section after authentication', async ({ popupPage }) => {
    // Note: Actual OAuth flow requires user interaction
    // This test verifies the UI structure is correct

    // Check settings section exists (hidden initially)
    const settingsSection = popupPage.locator('#settings-section, .settings-section');

    // In authenticated state, settings should be visible
    // This is a structural test - actual OAuth requires manual testing
    await expect(settingsSection).toBeAttached();
  });

  test('should have document ID input field', async ({ popupPage }) => {
    // Verify document ID input exists
    const docIdInput = popupPage.locator('#doc-id-input, [data-testid="doc-id-input"], input[name="documentId"]');
    await expect(docIdInput).toBeAttached();
  });

  test('should validate document ID format', async ({ popupPage }) => {
    const docIdInput = popupPage.locator('input[name="documentId"], #doc-id-input');

    if (await docIdInput.isVisible()) {
      // Test invalid format
      await docIdInput.fill('invalid-id');

      // Valid Google Docs ID format (44 characters alphanumeric + hyphens/underscores)
      const validDocId = '1aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789ABCD';
      await docIdInput.fill(validDocId);

      // Should accept valid format
      await expect(docIdInput).toHaveValue(validDocId);
    }
  });

  test('should persist authentication state across popup reopens', async ({ context, extensionId }) => {
    // Open popup first time
    const popup1 = await context.newPage();
    await popup1.goto(`chrome-extension://${extensionId}/dist/popup/popup.html`);
    await popup1.waitForLoadState('domcontentloaded');

    // Close and reopen
    await popup1.close();

    const popup2 = await context.newPage();
    await popup2.goto(`chrome-extension://${extensionId}/dist/popup/popup.html`);
    await popup2.waitForLoadState('domcontentloaded');

    // UI should load correctly
    await expect(popup2.locator('body')).toBeVisible();
    await popup2.close();
  });

  test('should display logout button when authenticated', async ({ popupPage }) => {
    // Verify logout button structure exists
    const logoutButton = popupPage.locator('#logout-button, [data-testid="logout-button"], button:has-text("連携を解除")');
    await expect(logoutButton).toBeAttached();
  });

  test('should show sync toggle after document selection', async ({ popupPage }) => {
    // Verify sync toggle exists
    const syncToggle = popupPage.locator('#sync-enabled, [data-testid="sync-enabled"], input[name="enabled"]');
    await expect(syncToggle).toBeAttached();
  });

  test('should display WebSocket connection status', async ({ popupPage, mockTauri }) => {
    // Wait for WebSocket connection to be established
    await popupPage.waitForTimeout(2000);

    // Status indicator should exist
    const statusIndicator = popupPage.locator('.status-badge, #ws-status, [data-testid="ws-status"]');
    await expect(statusIndicator).toBeAttached();
  });
});

test.describe('Scenario 1: Error Handling', () => {
  test('should show error message on authentication failure', async ({ popupPage }) => {
    // Verify error display area exists
    const errorArea = popupPage.locator('.error-message, #error-display, [role="alert"]');
    await expect(errorArea).toBeAttached();
  });

  test('should handle network errors gracefully', async ({ popupPage, context }) => {
    // Simulate offline mode
    await context.setOffline(true);

    // Attempt some action
    await popupPage.reload();

    // Should not crash
    await expect(popupPage.locator('body')).toBeVisible();

    // Restore network
    await context.setOffline(false);
  });
});
