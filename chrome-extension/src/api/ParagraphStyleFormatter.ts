/**
 * 段落スタイル設定実装
 *
 * 責務: Google Docs batchUpdate用のスタイルリクエスト生成
 * ファイルパス: chrome-extension/src/api/ParagraphStyleFormatter.ts
 *
 * Implementation: Phase 2
 *
 * スタイル定義:
 * - 見出し: HEADING_2（14pt、太字）
 * - 本文: NORMAL_TEXT（11pt、通常）
 * - タイムスタンプ: [HH:MM:SS] 形式
 * - 話者名: **[話者名]**: 形式（設定で有効化時）
 */

import type { IParagraphStyleFormatter } from './IParagraphStyleFormatter';

/**
 * Google Docs batchUpdateリクエスト型
 */
export interface BatchUpdateRequest {
  insertText?: {
    location: { index: number };
    text: string;
  };
  updateParagraphStyle?: {
    range: { startIndex: number; endIndex: number };
    paragraphStyle: ParagraphStyle;
    fields: string;
  };
  updateTextStyle?: {
    range: { startIndex: number; endIndex: number };
    textStyle: TextStyle;
    fields: string;
  };
}

interface ParagraphStyle {
  namedStyleType?: string;
  alignment?: string;
}

interface TextStyle {
  bold?: boolean;
  italic?: boolean;
  fontSize?: { magnitude: number; unit: string };
  foregroundColor?: {
    color: { rgbColor: { red: number; green: number; blue: number } };
  };
}

export class ParagraphStyleFormatter implements IParagraphStyleFormatter {
  /**
   * 見出しスタイル（HEADING_2）のリクエストを生成
   *
   * Markdown形式: "## 見出し" を挿入し、HEADING_2スタイルを適用
   * NamedRangeRecoveryStrategyと整合: "## 文字起こし" で検索可能
   *
   * @param text 見出しテキスト（"## " prefixなし）
   * @returns テキスト挿入 + スタイル適用のリクエスト配列
   */
  formatHeading(text: string): {
    text: string;
    createRequests: (index: number) => BatchUpdateRequest[];
  } {
    // Markdown形式: "## " prefixを追加
    const headingText = text.startsWith('## ') ? text : `## ${text}`;
    const formattedText = headingText.endsWith('\n') ? headingText : `${headingText}\n`;

    return {
      text: formattedText,
      createRequests: (index: number) => [
        {
          insertText: {
            location: { index },
            text: formattedText,
          },
        },
        {
          updateParagraphStyle: {
            range: {
              startIndex: index,
              endIndex: index + formattedText.length,
            },
            paragraphStyle: {
              namedStyleType: 'HEADING_2',
            },
            fields: 'namedStyleType',
          },
        },
      ],
    };
  }

  /**
   * タイムスタンプフォーマット（[HH:MM:SS]）
   *
   * @param date 日時（Dateオブジェクトまたはタイムスタンプ文字列）
   * @returns フォーマット済み文字列とリクエスト生成関数
   */
  formatTimestamp(date: string | Date): {
    text: string;
    createRequests: (index: number) => BatchUpdateRequest[];
  } {
    const d = typeof date === 'string' ? new Date(date) : date;
    const hours = String(d.getHours()).padStart(2, '0');
    const minutes = String(d.getMinutes()).padStart(2, '0');
    const seconds = String(d.getSeconds()).padStart(2, '0');
    const formattedText = `[${hours}:${minutes}:${seconds}] `;

    return {
      text: formattedText,
      createRequests: (index: number) => [
        {
          insertText: {
            location: { index },
            text: formattedText,
          },
        },
        {
          updateTextStyle: {
            range: {
              startIndex: index,
              endIndex: index + formattedText.length,
            },
            textStyle: {
              foregroundColor: {
                color: {
                  rgbColor: { red: 0.5, green: 0.5, blue: 0.5 }, // グレー
                },
              },
            },
            fields: 'foregroundColor',
          },
        },
      ],
    };
  }

  /**
   * 話者名フォーマット（**[話者名]**: ）
   *
   * @param name 話者名
   * @returns フォーマット済み文字列とリクエスト生成関数
   */
  formatSpeakerName(name: string): {
    text: string;
    createRequests: (index: number) => BatchUpdateRequest[];
  } {
    const formattedText = `${name}: `;

    return {
      text: formattedText,
      createRequests: (index: number) => [
        {
          insertText: {
            location: { index },
            text: formattedText,
          },
        },
        {
          updateTextStyle: {
            range: {
              startIndex: index,
              endIndex: index + name.length, // コロンは太字にしない
            },
            textStyle: {
              bold: true,
            },
            fields: 'bold',
          },
        },
      ],
    };
  }

  /**
   * 通常テキスト（NORMAL_TEXT）
   */
  formatNormalText(text: string): {
    text: string;
    createRequests: (index: number) => BatchUpdateRequest[];
  } {
    const formattedText = text.endsWith('\n') ? text : `${text}\n`;

    return {
      text: formattedText,
      createRequests: (index: number) => [
        {
          insertText: {
            location: { index },
            text: formattedText,
          },
        },
      ],
    };
  }

  /**
   * 文字起こし行をフォーマット
   *
   * @param timestamp タイムスタンプ
   * @param speaker 話者名（オプション）
   * @param text 文字起こしテキスト
   * @param options オプション設定
   */
  formatTranscriptLine(
    timestamp: Date,
    text: string,
    options: {
      showTimestamp?: boolean;
      showSpeaker?: boolean;
      speaker?: string;
    } = {}
  ): {
    text: string;
    createRequests: (index: number) => BatchUpdateRequest[];
  } {
    const parts: string[] = [];
    const styleGenerators: { offset: number; generator: (index: number) => BatchUpdateRequest[] }[] = [];

    let currentOffset = 0;

    // タイムスタンプ
    if (options.showTimestamp !== false) {
      const ts = this.formatTimestamp(timestamp);
      parts.push(ts.text);
      const tsOffset = currentOffset;
      styleGenerators.push({
        offset: tsOffset,
        generator: (index) => {
          // insertTextは除外（全体で1回だけ挿入するため）、スタイルのみ返す
          const requests = ts.createRequests(index + tsOffset);
          return requests.filter(r => !r.insertText);
        },
      });
      currentOffset += ts.text.length;
    }

    // 話者名
    if (options.showSpeaker && options.speaker) {
      const sp = this.formatSpeakerName(options.speaker);
      parts.push(sp.text);
      const spOffset = currentOffset;
      styleGenerators.push({
        offset: spOffset,
        generator: (index) => {
          const requests = sp.createRequests(index + spOffset);
          return requests.filter(r => !r.insertText);
        },
      });
      currentOffset += sp.text.length;
    }

    // テキスト
    const normalText = text.endsWith('\n') ? text : `${text}\n`;
    parts.push(normalText);

    const fullText = parts.join('');

    return {
      text: fullText,
      createRequests: (index: number) => {
        // 1. テキスト挿入（1回のみ）
        const requests: BatchUpdateRequest[] = [
          {
            insertText: {
              location: { index },
              text: fullText,
            },
          },
        ];

        // 2. スタイル適用（挿入後のインデックスで適用）
        for (const { generator } of styleGenerators) {
          requests.push(...generator(index));
        }

        return requests;
      },
    };
  }
}
