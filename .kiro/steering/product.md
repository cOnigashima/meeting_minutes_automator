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

## Current Implementation Status（2025-10-19）

- ✅ **MVP0 Walking Skeleton**（完了: 2025-10-10）  
  Fake 録音により 3 プロセス疎通を確立し、Chrome 拡張スケルトンを動作させる基盤を構築。
- ✅ **MVP1 Real STT（コア）**（2025-10-14 アップデート）  
  AudioDeviceAdapter / VAD / Whisper パイプラインとイベントストリーム IPC を実装し、macOS で手動/自動テストを通過。
- 🔄 **MVP1 Real STT（UI/UX 拡張）**  
  デバイス選択 UI、セッション管理 UI、Docs 同期前提のストレージ可視化を Task 9.x / 10.x で開発中。
- 🔵 **MVP2 Google Docs Sync**  
  要件・設計（Design Generated）までは完了。MVP1 の安定化後に実装開始。
- ⚪ **MVP3 LLM Summary + Production UI**  
  要件定義待ち。STT + Docs 同期を前提条件とする。

### Sub-Specification Progress

| Spec | 現在のフェーズ | 直近の成果 / 次のステップ |
|------|---------------|---------------------------|
| meeting-minutes-core (MVP0) | Implementation Complete ✅ | Walking Skeleton 完了、今後はバグ修正のみ |
| meeting-minutes-stt (MVP1) | Implementation 🔄 | Task 2〜7 完了、Task 9.x（UI統合）・10.x（E2E自動化）実施中 |
| meeting-minutes-docs-sync (MVP2) | Design Generated 🔵 | OAuth / Docs API 設計済み、実装は MVP1 安定化待ち |
| meeting-minutes-ci | Spec Initialized 🔵 | クロスプラットフォームCI と smoke テストの設計を継続 |
| meeting-minutes-llm (MVP3) | Not Started ⚪ | MVP1/2 完了後に要件定義開始 |
| ui-hub | Design Generated 🔵 | 既存UI改善のためのトークン駆動開発環境設計完了、tasks生成待ち |

### Implementation Progress Snapshots

- ✅ **ADR / 仕様更新**: ADR-013/015/016 により IPC デッドロック、モデル切替、オフラインフォールバックの P0 問題を解消。
- ✅ **テストベッド**:
  - `cargo test --test stt_e2e_test`（Whisper モデルとフィクスチャ音声を用いた E2E）
  - `.venv/bin/python -m pytest tests/test_audio_integration.py`（部分/確定イベント、エラー配信を検証）
- ✅ **ドキュメント**: README / platform-verification.md / meeting-minutes-stt tasks を最新状態へ更新。
- 🔄 **CI 自動化**: GitHub Actions matrix は設計済み。現状は macOS 手動検証 + ローカルテストで代替（meeting-minutes-ci で追跡）。

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
