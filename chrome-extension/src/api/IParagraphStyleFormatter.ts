/**
 * 段落スタイル設定インターフェース
 *
 * 責務: 見出し、タイムスタンプ、話者名のスタイル設定
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */

export interface IParagraphStyleFormatter {
  formatHeading(text: string): any;
  formatTimestamp(text: string): any;
  formatSpeakerName(text: string): any;
}
