/**
 * Named Range自動復旧戦略実装
 *
 * 責務: Named Range削除時の復旧位置決定
 * ファイルパス: chrome-extension/src/api/NamedRangeRecoveryStrategy.ts
 *
 * Implementation: Phase 2
 *
 * 復旧優先度:
 *   1. 見出し「## 文字起こし」検索 → 見出し直後
 *   2. ドキュメント末尾
 *   3. ドキュメント先頭（index=1）
 */

import { ok, err, type Result } from '../types/Result';
import type { ApiError } from '../types/ApiTypes';
import type { INamedRangeRecoveryStrategy } from './INamedRangeRecoveryStrategy';
import type { IAuthManager } from '../auth/IAuthManager';

const DOCS_API_BASE = 'https://docs.googleapis.com/v1/documents';
const TRANSCRIPT_HEADING = '## 文字起こし';

export class NamedRangeRecoveryStrategy implements INamedRangeRecoveryStrategy {
  constructor(private authManager: IAuthManager) {}

  /**
   * 復旧位置を検索
   *
   * 優先度順に検索:
   * 1. 見出し「## 文字起こし」の直後
   * 2. ドキュメント末尾
   * 3. ドキュメント先頭（index=1）
   */
  async findRecoveryPosition(documentId: string): Promise<Result<number, ApiError>> {
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        code: 401,
        message: tokenResult.error.message,
      });
    }

    try {
      const response = await fetch(
        `${DOCS_API_BASE}/${documentId}`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${tokenResult.value}`,
          },
        }
      );

      if (!response.ok) {
        return err({
          code: response.status,
          message: `Failed to get document (HTTP ${response.status})`,
        });
      }

      const doc = await response.json();

      // Priority 1: 見出し検索
      const headingPosition = this.findHeadingPosition(doc, TRANSCRIPT_HEADING);
      if (headingPosition !== null) {
        console.log(`[NamedRangeRecovery] Found heading at index ${headingPosition}`);
        return ok(headingPosition);
      }

      // Priority 2: ドキュメント末尾
      const endIndex = this.getDocumentEndIndex(doc);
      if (endIndex > 1) {
        console.log(`[NamedRangeRecovery] Using document end at index ${endIndex}`);
        return ok(endIndex);
      }

      // Priority 3: ドキュメント先頭
      console.log('[NamedRangeRecovery] Using document start (index=1)');
      return ok(1);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        code: 500,
        message,
      });
    }
  }

  /**
   * ドキュメント内で見出しを検索し、その直後の位置を返す
   */
  private findHeadingPosition(doc: GoogleDocsDocument, heading: string): number | null {
    const content = doc.body?.content;
    if (!content) return null;

    for (const element of content) {
      if (element.paragraph?.elements) {
        const text = element.paragraph.elements
          .map((e: ParagraphElement) => e.textRun?.content || '')
          .join('');

        if (text.includes(heading)) {
          // 見出しの末尾（改行の後）の位置を返す
          return element.endIndex ?? null;
        }
      }
    }

    return null;
  }

  /**
   * ドキュメントの末尾インデックスを取得
   */
  private getDocumentEndIndex(doc: GoogleDocsDocument): number {
    const content = doc.body?.content;
    if (!content || content.length === 0) return 1;

    const lastElement = content[content.length - 1];
    // 末尾の改行を考慮して、endIndex - 1 を返す
    return Math.max(1, (lastElement.endIndex || 1) - 1);
  }
}

// Google Docs API型定義
interface GoogleDocsDocument {
  body?: {
    content?: StructuralElement[];
  };
}

interface StructuralElement {
  paragraph?: {
    elements?: ParagraphElement[];
  };
  endIndex?: number;
}

interface ParagraphElement {
  textRun?: {
    content?: string;
  };
}
