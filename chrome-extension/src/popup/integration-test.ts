/**
 * Integration Test: Production Code + Real Google API
 *
 * テスト対象:
 * 1. AuthManager.initiateAuth() - OAuth認証フロー
 * 2. AuthManager.getAccessToken() - トークン取得
 * 3. GoogleDocsClient.insertText() - テキスト挿入
 * 4. NamedRangeManager - Named Range管理
 *
 * 使用方法:
 * 1. popup.htmlで「Integration Test」ボタンをクリック
 * 2. DevTools Consoleで結果を確認
 */

import { getAuthManager } from '../auth/AuthFactory';
import { GoogleDocsClient } from '../api/GoogleDocsClient';
import { ExponentialBackoffHandler } from '../api/ExponentialBackoffHandler';
import { NamedRangeManager } from '../api/NamedRangeManager';
import { NamedRangeRecoveryStrategy } from '../api/NamedRangeRecoveryStrategy';
import { ParagraphStyleFormatter } from '../api/ParagraphStyleFormatter';

type TestResult = {
  step: string;
  status: 'PASS' | 'FAIL';
  details?: string;
};

export async function runIntegrationTest(documentId: string): Promise<void> {
  const results: TestResult[] = [];

  console.log('='.repeat(80));
  console.log('Integration Test: Production Code + Real Google API');
  console.log('='.repeat(80));
  console.log(`Document ID: ${documentId}`);
  console.log('');

  // Initialize components
  const authManager = getAuthManager();
  const backoffHandler = new ExponentialBackoffHandler();
  const docsClient = new GoogleDocsClient(authManager, backoffHandler);
  const recoveryStrategy = new NamedRangeRecoveryStrategy(authManager);
  const namedRangeManager = new NamedRangeManager(docsClient, recoveryStrategy);
  const formatter = new ParagraphStyleFormatter();

  try {
    // Step 1: Authentication
    console.log('[Step 1] AuthManager.initiateAuth()...');
    const authResult = await authManager.initiateAuth();
    if (authResult.ok) {
      results.push({ step: 'AuthManager.initiateAuth()', status: 'PASS' });
      console.log('[PASS] OAuth認証成功');
    } else {
      const errorMessage = 'message' in authResult.error ? authResult.error.message : authResult.error.type;
      results.push({ step: 'AuthManager.initiateAuth()', status: 'FAIL', details: errorMessage });
      console.error('[FAIL] OAuth認証失敗:', authResult.error);
      printSummary(results);
      return;
    }

    // Step 2: Get Access Token
    console.log('\n[Step 2] AuthManager.getAccessToken()...');
    const tokenResult = await authManager.getAccessToken();
    if (tokenResult.ok) {
      results.push({ step: 'AuthManager.getAccessToken()', status: 'PASS' });
      console.log('[PASS] トークン取得成功');
    } else {
      results.push({ step: 'AuthManager.getAccessToken()', status: 'FAIL', details: tokenResult.error.message });
      console.error('[FAIL] トークン取得失敗:', tokenResult.error);
      printSummary(results);
      return;
    }

    // Step 3: Insert Text
    console.log('\n[Step 3] GoogleDocsClient.insertText()...');
    const timestamp = new Date().toISOString();
    const testText = `[Integration Test] ${timestamp}`;
    const insertResult = await docsClient.insertText(documentId, testText + '\n', 1);
    if (insertResult.ok) {
      results.push({ step: 'GoogleDocsClient.insertText()', status: 'PASS' });
      console.log('[PASS] テキスト挿入成功:', testText);
    } else {
      results.push({ step: 'GoogleDocsClient.insertText()', status: 'FAIL', details: insertResult.error.message });
      console.error('[FAIL] テキスト挿入失敗:', insertResult.error);
      printSummary(results);
      return;
    }

    // Step 4: Initialize Named Range
    console.log('\n[Step 4] NamedRangeManager.initializeCursor()...');
    const initResult = await namedRangeManager.initializeCursor(documentId);
    if (initResult.ok) {
      results.push({ step: 'NamedRangeManager.initializeCursor()', status: 'PASS' });
      console.log('[PASS] Named Range初期化成功');
    } else {
      results.push({ step: 'NamedRangeManager.initializeCursor()', status: 'FAIL', details: initResult.error.message });
      console.error('[FAIL] Named Range初期化失敗:', initResult.error);
      printSummary(results);
      return;
    }

    // Step 5: Get Cursor Position
    console.log('\n[Step 5] NamedRangeManager.getCursorPosition()...');
    const posResult = await namedRangeManager.getCursorPosition(documentId);
    if (posResult.ok) {
      results.push({ step: 'NamedRangeManager.getCursorPosition()', status: 'PASS', details: `index=${posResult.value}` });
      console.log('[PASS] カーソル位置取得成功:', posResult.value);
    } else {
      results.push({ step: 'NamedRangeManager.getCursorPosition()', status: 'FAIL', details: posResult.error.message });
      console.error('[FAIL] カーソル位置取得失敗:', posResult.error);
      printSummary(results);
      return;
    }

    // Step 6: Update Cursor Position
    // Note: 同じ位置を再設定してAPIが動作することを確認
    console.log('\n[Step 6] NamedRangeManager.updateCursorPosition()...');
    const newPosition = posResult.value;
    const updateResult = await namedRangeManager.updateCursorPosition(documentId, newPosition);
    if (updateResult.ok) {
      results.push({ step: 'NamedRangeManager.updateCursorPosition()', status: 'PASS', details: `newIndex=${newPosition}` });
      console.log('[PASS] カーソル位置更新成功:', newPosition);
    } else {
      results.push({ step: 'NamedRangeManager.updateCursorPosition()', status: 'FAIL', details: updateResult.error.message });
      console.error('[FAIL] カーソル位置更新失敗:', updateResult.error);
      printSummary(results);
      return;
    }

    // Step 7: ParagraphStyleFormatter (pure function test)
    console.log('\n[Step 7] ParagraphStyleFormatter...');
    const formatted = formatter.formatTranscriptLine(new Date(), 'テスト文字起こし', {
      showTimestamp: true,
      showSpeaker: true,
      speaker: 'Speaker1',
    });
    console.log('[PASS] フォーマット生成:', formatted.text);
    results.push({ step: 'ParagraphStyleFormatter', status: 'PASS', details: formatted.text.trim() });

  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    results.push({ step: 'Unexpected Error', status: 'FAIL', details: message });
    console.error('[FAIL] 予期しないエラー:', error);
  }

  printSummary(results);
}

function printSummary(results: TestResult[]): void {
  console.log('\n' + '='.repeat(80));
  console.log('Integration Test Summary');
  console.log('='.repeat(80));

  const passed = results.filter(r => r.status === 'PASS').length;
  const failed = results.filter(r => r.status === 'FAIL').length;

  results.forEach(r => {
    const icon = r.status === 'PASS' ? '✅' : '❌';
    const details = r.details ? ` (${r.details})` : '';
    console.log(`${icon} ${r.step}${details}`);
  });

  console.log('');
  console.log(`結果: ${passed}/${results.length} テスト合格`);

  if (failed === 0) {
    console.log('\n🎉 Integration Test Complete! All tests passed.');
  } else {
    console.log(`\n⚠️ ${failed} test(s) failed.`);
  }
}

// Expose to window for console access
(window as unknown as { runIntegrationTest: typeof runIntegrationTest }).runIntegrationTest = runIntegrationTest;
