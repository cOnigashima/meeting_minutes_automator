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

## Core Features

### 🎤 リアルタイム音声処理
- **常時録音**: マイクまたはシステム音声のキャプチャ
- **音声活動検出**: webrtcvadによる高精度な発話境界検出
- **リアルタイム文字起こし**: faster-whisperによる0.5秒以内の応答
- **自動要約生成**: LLMを活用したキーポイント抽出と要約

### 🖥️ デスクトップアプリケーション
- **クロスプラットフォーム対応**: macOS、Windows、Linux
- **軽量アーキテクチャ**: Tauriによる高速起動と低メモリ使用量
- **直感的UI**: リアルタイムテキスト表示と録音制御
- **プライバシー重視**: ローカル処理による機密情報保護

### 🌐 Chrome拡張連携
- **Google Docs統合**: 自動的な議事録挿入と構造化
- **リアルタイム同期**: WebSocket通信による即座の反映
- **Named Range管理**: プログラマティックな文書構造制御
- **オフライン対応**: ネットワーク切断時のローカル保存

### ⚙️ 高度な設定と制御
- **音声デバイス選択**: 入力ソースの柔軟な切り替え
- **STTモデル選択**: 精度と速度のバランス調整
- **要約設定**: 生成頻度と詳細レベルのカスタマイズ
- **セッション管理**: 録音の保存、読み込み、エクスポート

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

## Current Implementation Status

**開発フェーズ**: 仕様検証完了・実装準備中（Specification Phase）

本プロジェクトは現在、Kiro仕様駆動開発手法に基づき、**実装前の仕様策定と設計検証**を完了した段階にあります。コードベースの実装はまだ開始していませんが、以下の4つのsub-specificationに分割された詳細な要件定義と技術設計が完了しています。

### Sub-Specifications Progress

#### 📦 meeting-minutes-core (MVP0 - Walking Skeleton)
- **目的**: Tauri + Python + Chrome拡張の最小疎通確認（Fake実装）
- **フェーズ**: Design Validated ✅
- **ステータス**: タスク生成済み、実装承認待ち
- **主要成果**:
  - 3プロセス間IPC通信プロトコルの確定
  - WebSocket message type設計（Tagged Union）
  - E2Eテスト自動化戦略の策定

#### 🎤 meeting-minutes-stt (MVP1 - Real STT)
- **目的**: faster-whisper統合、webrtcvad統合、音声デバイス管理
- **フェーズ**: Design Validated ✅
- **ステータス**: 設計検証完了、タスク生成準備完了
- **主要成果**:
  - ADR-001: 録音責務の一元化（Rust側のみ）
  - ADR-002: ハイブリッドモデル配布戦略
  - ADR-003: IPCバージョニング方針
  - Pre-commit hooks + 静的解析基盤の整備

#### 📄 meeting-minutes-docs-sync (MVP2 - Google Docs Sync)
- **目的**: OAuth 2.0認証、Google Docs API統合、Named Range管理
- **フェーズ**: Design Generated 🔵
- **ステータス**: 設計生成完了、レビュー待ち
- **主要成果**:
  - MV3 Service Worker対応設計（chrome.alarms + Offscreen Document）
  - Optimistic Locking戦略（writeControl.requiredRevisionId）
  - Token Bucket RateLimiter設計

#### 🤖 meeting-minutes-llm (MVP3 - LLM Summary + Production UI)
- **目的**: プロダクション準備、UI/UX完成度向上
- **フェーズ**: Not Started ⚪
- **ステータス**: 要件定義待ち（MVP0-2完了後）

### Implementation Readiness

現在のプロジェクト状態:
- ✅ **Steering Documents**: 製品方針、技術スタック、構造、設計原則が確定
- ✅ **Architecture**: 3プロセスアーキテクチャとIPC通信プロトコル策定完了
- ✅ **Quality Infrastructure**: CI/CD基盤（pre-commit hooks、静的解析）整備済み
- ✅ **Design Validation**: 主要な技術的課題（録音責務、モデル配布、IPC等）のADR作成完了
- 🔵 **Codebase**: 実装未開始（`src-tauri/`, `src/`, `chrome-extension/`, `python-stt/`未作成）

**次のステップ**: meeting-minutes-core (MVP0) のタスク承認後、Walking Skeleton実装を開始

---

## Product Roadmap

### v1.0 - Initial Release (Current Target)

**コア機能**:
- faster-whisperによるローカル音声認識
- OpenAI API（GPT-4o）を使用した要約生成（**ネットワーク必須**）
- Google Docs統合
- macOS、Windows、Linux対応

**制約事項**:
- 要約生成にはOpenAI APIキーが必須
- オンライン環境での利用を前提

### v2.0 - Offline-First (Future)

**拡張機能**:
- ローカルLLM統合（Llama.cpp）
- 完全オフライン動作モード
- 要約生成のローカル実行オプション
- より高度な多言語対応

**技術的投資**:
- LLMモデルのローカルバンドル（2〜4GB）
- GPU加速オプション
- モデル選択UI（OpenAI / ローカルLlama切り替え）

### v3.0+ - Advanced Features (Vision)

- リアルタイム翻訳機能
- Zoom、Teams等の会議ツール直接統合
- モバイルプラットフォーム対応（Android、iOS）
- アクションアイテムの自動抽出と通知