/**
 * Named Range統合実装
 *
 * 責務: transcript_cursor Named Rangeの管理（作成、更新、自動復旧）
 * ファイルパス: chrome-extension/src/api/NamedRangeManager.ts
 *
 * Implementation: Phase 2
 *
 * Named Range名: transcript_cursor
 * 復旧優先度:
 *   1. 見出し「## 文字起こし」検索 → 見出し直後に再作成
 *   2. ドキュメント末尾に再作成
 *   3. ドキュメント先頭（index=1）に再作成
 */

import { ok, err, type Result } from '../types/Result';
import type { ApiError } from '../types/ApiTypes';
import type { INamedRangeManager } from './INamedRangeManager';
import type { IGoogleDocsClient } from './IGoogleDocsClient';
import type { INamedRangeRecoveryStrategy } from './INamedRangeRecoveryStrategy';

const CURSOR_NAME = 'transcript_cursor';

export class NamedRangeManager implements INamedRangeManager {
  constructor(
    private docsClient: IGoogleDocsClient,
    private recoveryStrategy: INamedRangeRecoveryStrategy
  ) {}

  /**
   * カーソルNamed Rangeを初期化
   *
   * 既存のNamed Rangeがあれば何もしない
   * なければ復旧戦略に従って作成
   */
  async initializeCursor(documentId: string): Promise<Result<void, ApiError>> {
    // 既存のNamed Rangeを確認
    const positionResult = await this.docsClient.getNamedRangePosition(documentId, CURSOR_NAME);

    if (positionResult.ok) {
      // 既存のNamed Rangeがあればそのまま使用
      return ok(undefined);
    }

    // Named Rangeがない場合は復旧戦略で作成
    return this.recoverCursor(documentId);
  }

  /**
   * カーソル位置を更新
   *
   * テキスト挿入後に呼び出し、Named Rangeを新しい位置に移動
   */
  async updateCursorPosition(
    documentId: string,
    newIndex: number
  ): Promise<Result<void, ApiError>> {
    // 既存のNamed Rangeを削除
    const deleteResult = await this.docsClient.deleteNamedRange(documentId, CURSOR_NAME);
    if (!deleteResult.ok) {
      return deleteResult;
    }

    // 新しい位置にNamed Rangeを作成
    return this.docsClient.createNamedRange(
      documentId,
      CURSOR_NAME,
      newIndex,
      newIndex + 1 // Named Rangeは最低1文字必要
    );
  }

  /**
   * カーソルNamed Rangeを復旧
   *
   * 優先度順に復旧を試行:
   * 1. 見出し「## 文字起こし」の直後
   * 2. ドキュメント末尾
   * 3. ドキュメント先頭（index=1）
   */
  async recoverCursor(documentId: string): Promise<Result<void, ApiError>> {
    const recoveryResult = await this.recoveryStrategy.findRecoveryPosition(documentId);

    if (!recoveryResult.ok) {
      return err({
        code: 500,
        message: recoveryResult.error.message,
      });
    }

    const position = recoveryResult.value;

    // Named Rangeを作成
    return this.docsClient.createNamedRange(
      documentId,
      CURSOR_NAME,
      position,
      position + 1
    );
  }

  /**
   * 現在のカーソル位置を取得
   */
  async getCursorPosition(documentId: string): Promise<Result<number, ApiError>> {
    const result = await this.docsClient.getNamedRangePosition(documentId, CURSOR_NAME);

    if (!result.ok) {
      return err({
        code: 404,
        message: result.error.message,
      });
    }

    return ok(result.value.startIndex);
  }
}
