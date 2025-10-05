# Project Overview: Meeting Minutes Automator

## Purpose
Meeting Minutes Automatorは、会議や打ち合わせの音声をリアルタイムで自動的に文字起こしし、構造化された議事録を生成するソフトウェアです。ローカル環境での高精度音声処理とGoogle Docsとのシームレス連携により、手動での議事録作成から解放される革新的なツールです。

## Current Development Phase
**仕様検証完了・実装準備中（Specification Phase）**

- プロジェクトはKiro仕様駆動開発手法に基づき、実装前の仕様策定と設計検証を完了した段階
- コードベースの実装はまだ開始していない（`src-tauri/`, `src/`, `chrome-extension/`, `python-stt/`は未作成）
- 詳細な要件定義と技術設計が4つのsub-specificationに分割されて完了

## Sub-Specifications Progress

### meeting-minutes-core (MVP0 - Walking Skeleton)
- **目的**: Tauri + Python + Chrome拡張の最小疎通確認（Fake実装）
- **フェーズ**: Design Validated ✅
- **ステータス**: タスク生成済み、実装承認待ち

### meeting-minutes-stt (MVP1 - Real STT)
- **目的**: faster-whisper統合、webrtcvad統合、音声デバイス管理
- **フェーズ**: Design Validated ✅
- **ステータス**: 設計検証完了、タスク生成準備完了

### meeting-minutes-docs-sync (MVP2 - Google Docs Sync)
- **目的**: OAuth 2.0認証、Google Docs API統合、Named Range管理
- **フェーズ**: Design Generated 🔵
- **ステータス**: 設計生成完了、レビュー待ち

### meeting-minutes-llm (MVP3 - LLM Summary)
- **目的**: プロダクション準備、UI/UX完成度向上
- **フェーズ**: Not Started ⚪
- **ステータス**: 要件定義待ち（MVP0-2完了後）

## Key Features
- **リアルタイム音声処理**: faster-whisperによる0.5秒以内の応答
- **音声活動検出**: webrtcvadによる高精度な発話境界検出
- **Google Docs統合**: WebSocket通信による即座の反映
- **オフライン対応**: ネットワーク切断時のローカル保存
- **プライバシー重視**: ローカル処理による機密情報保護
