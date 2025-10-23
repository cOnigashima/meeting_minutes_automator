# Product Overview

## 用語集

プロジェクト全体で使用する主要な用語の定義は、以下のドキュメントを参照してください:

📖 **[用語集 (Glossary)](../.kiro/specs/meeting-minutes-automator/requirements.md#用語集-glossary)**

主要な用語:
- **セグメント要約**: 単一の発話から生成される要約
- **ローリングサマリー**: セッション全体の累積要約（動的更新）
- **セッションサマリー**: 録音セッション終了時の最終全体要約

## Product Overview

Meeting Minutes Automator は、会議や打ち合わせの音声をリアルタイムで自動的に文字起こしし、構造化された議事録を生成するソフトウェアです。ローカル環境での高精度音声処理とGoogle Docsとのシームレス連携により、手動での議事録作成から解放される革新的なツールです。

## Core Features（ロードマップ別）

### 🎤 リアルタイム音声処理（MVP1 コア機能）
- ✅ **webrtcvad + faster-whisper** によるリアルタイム文字起こし（部分/確定テキストをイベント配信）
- ✅ **ResourceMonitor** によるモデル自動ダウングレード・アップグレード提案
- ✅ **AudioDeviceAdapter**（CoreAudio / WASAPI / ALSA）で物理/仮想デバイスを扱い、切断検知を実装
- ⏳ **要約生成**・**トピック抽出** は MVP3（LLM統合）で提供予定

### 🖥️ デスクトップアプリケーション（MVP0 完了 → MVP1 拡張中）
- ✅ Tauri + Rust + React による軽量なクロスプラットフォーム基盤
- ✅ Pythonサイドカーのライフサイクル管理（起動/終了/ヘルスチェック、自動復旧）
- ✅ WebSocket サーバーで Chrome 拡張へストリーム配信
- ⏳ UI 強化（デバイス選択・セッション履歴・リソース警告表示）は Task 9.x で実装予定

### 🌐 Chrome拡張連携（MVP0 完了 → MVP2 で機能拡張）
- ✅ WebSocket クライアント + 再接続ロジック + `chrome.storage.local` へのストリーム同期
- ⏳ Google Docs API 連携・Named Range 管理は MVP2 のスコープ

### ⚙️ 設定と制御
- ✅ モデル監視・ダウングレード提案・録音一時停止の IPC 通知
- ⏳ デバイス選択 UI とセッション管理 UI（バックエンド API は実装済み）
- ⏳ LLM 設定・Docs 同期設定は MVP2/3 で段階的に追加予定

## Target Use Case

### Primary Use Cases

**📋 会議議事録の自動化**
- 社内会議、顧客との打ち合わせ、プロジェクトレビュー
- リアルタイムでの発言内容記録と要点整理
- 構造化された議事録のGoogle Docs自動生成

**🎯 コンテンツ制作支援**
- インタビュー、ポッドキャスト、ウェビナーの文字起こし
- 講演やプレゼンテーションの記録と要約
- 研究インタビューや調査内容の構造化

**👥 教育・研修環境**
- オンライン授業やセミナーの記録
- 研修内容の要点抽出と共有
- 学習内容の復習用資料作成

### Target Users

**ビジネスプロフェッショナル**
- プロジェクトマネージャー、チームリーダー
- 営業担当者、コンサルタント
- 人事担当者、経営陣

**コンテンツ制作者**
- ジャーナリスト、ライター
- ポッドキャスター、YouTuber
- 研究者、アナリスト

**教育関係者**
- 教師、講師、トレーナー
- 学生、研究者
- 企業研修担当者

## Key Value Proposition

### 🚀 効率性の飛躍的向上
- **時間削減**: 手動議事録作成から90%以上の時間短縮
- **品質向上**: 人的ミスの削減と一貫した記録品質
- **同時作業**: 会議参加に集中しながら自動で記録

### 🔒 プライバシーとセキュリティ
- **ローカル処理**: 機密情報がクラウドに送信されない
- **データ制御**: 録音データの保存・削除の完全制御
- **企業対応**: 厳格なセキュリティ要件への適合

### 🌍 アクセシビリティと包括性
- **多言語対応**: 日本語・英語での高精度認識
- **聴覚支援**: リアルタイム字幕による会議参加支援
- **クロスプラットフォーム**: どの環境でも一貫した体験

### 💡 革新的な統合体験
- **ハイブリッドアーキテクチャ**: デスクトップアプリとブラウザ拡張の最適組み合わせ
- **リアルタイム同期**: 音声からドキュメントまでの瞬時反映
- **構造化出力**: 単なる文字起こしではない知的な文書生成

## Unique Differentiators

### vs. 既存のクラウドサービス
- **プライバシー優先**: ローカル処理による完全なデータ制御
- **レスポンス速度**: クラウドAPI遅延なしの即座処理
- **コスト効率**: 従量課金なしの一度購入での継続利用

### vs. 単機能文字起こしツール
- **包括的ワークフロー**: 録音から最終文書まで一気通貫
- **知的処理**: 単純文字起こしを超えた要約と構造化
- **活用環境統合**: Google Docsとのネイティブ連携

### vs. 手動議事録作成
- **精度と網羅性**: 人間の記憶や筆記速度の限界を超越
- **客観性**: 発言者バイアスや記録者の主観を排除
- **再現性**: 録音データからの正確な振り返りと検証

## Current Implementation Status（2025-10-21）

- ✅ **MVP0 Walking Skeleton**（完了: 2025-10-10）
  Fake 録音により 3 プロセス疎通を確立し、Chrome 拡張スケルトンを動作させる基盤を構築。
- ✅ **MVP1 Real STT**（完了: 2025-10-21、Phase 13+14完了）
  - Phase 13: 検証負債解消（E2Eテスト、長時間稼働、セキュリティ修正）
  - Phase 14: Post-MVP1 Cleanup（LegacyIpcMessage削除、0 warnings達成）
  - テスト: 267/285合格（18件失敗は優先度P2、コア機能動作に影響なし）
- 📋 **次: MVP2 Google Docs Sync** - OAuth 2.0 + Google Docs API統合
  - MVP2 Phase 0でテスト修正を検討（優先度P2、18件失敗）
- 🔵 **並行: meeting-minutes-ci** - クロスプラットフォームCI整備
- ⚪ **MVP3 LLM Summary + Production UI**
  MVP2完了後に要件定義開始。STT + Docs 同期を前提条件とする。

### Sub-Specification Progress

| Spec | 現在のフェーズ | 直近の成果 / 次のステップ |
|------|---------------|---------------------------|
| meeting-minutes-core (MVP0) | Implementation Complete ✅ | Walking Skeleton 完了、今後はバグ修正のみ |
| meeting-minutes-stt (MVP1) | Completed ✅ | Phase 13+14完了（2025-10-21）、267/285テスト合格、18件失敗はP2で順次対応 |
| meeting-minutes-docs-sync (MVP2) | Design Generated 🔵 | OAuth/Docs API設計済み、tasks生成待ち、Phase 0でテスト修正検討 |
| meeting-minutes-ci | Spec Initialized 🔵 | CI依存タスク受入完了（CI-INTAKE-001/002/003）、tasks生成待ち |
| meeting-minutes-llm (MVP3) | Not Started ⚪ | MVP2完了後に要件定義開始 |
| ui-hub | Ready for Implementation 🔵 | 全承認完了、22タスク実装準備完了 |

### Implementation Progress Snapshots

- ✅ **MVP1完了（2025-10-21）**: Phase 13+14完了
  - Phase 13: Task 10.1/10.2/10.3/10.4/10.6/10.7 E2Eテスト + Task 11.3長時間稼働 + SEC-001/002/005セキュリティ修正
  - Phase 14: LegacyIpcMessage完全削除、P0バグ修正、Rust 0 warnings達成
  - テスト: Rust 106/107 + Python 161/178 = 267/285合格
  - Known Limitations: ADR-018で文書化（LIMIT-001〜003、優先度P2）
- 🔵 **CI依存タスク移行**: meeting-minutes-ci specへ移管（Task 10.5, SEC-003, SEC-004）
- 📋 **MVP2 Phase 0検討事項**: Python 17件 + Rust 1件のテスト失敗解消（優先度P2）
- ✅ **ドキュメント**: README / platform-verification.md / MVP2-HANDOFF.md を最新状態へ更新

---

## Product Roadmap

### v1.0 - Local STT Release（開発中）
- faster-whisper + webrtcvad によるリアルタイム文字起こし
- AudioDeviceAdapter でのマイク / ループバック入力とリソース監視
- Chrome 拡張でのリアルタイム表示（Docs 連携は手動手順）
- セッションデータ（音声/文字起こし/メタデータ）のローカル保存

### v1.1 - Google Docs Sync
- OAuth 2.0 認可フロー（Desktop ↔ Chrome 拡張連携）
- Named Range を用いた Docs 更新とオフラインキュー
- エラー通知 + リトライポリシーの実装

### v1.2 - Summaries & Insights
- LLM 要約（当初はクラウドAPI、オプションでローカルLLM）
- アクションアイテム抽出、ハイライト生成
- Chrome 拡張 / Tauri UI での概要表示

### v2.0+ - 追加ビジョン
- 完全オフライン動作（ローカルLLM + Docs 同期の後読み）
- Zoom / Teams 等の他会議ツール統合
- モバイルクライアント（iOS / Android）
- 多言語翻訳・アクセシビリティ機能の拡張
