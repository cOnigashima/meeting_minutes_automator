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

**開発フェーズ**: MVP0完了、MVP1準備中（2025-10-10更新）

本プロジェクトは、Kiro仕様駆動開発手法に基づき、**MVP0（Walking Skeleton）が完成**しました。3プロセス間（Tauri + Python + Chrome拡張）のE2E疎通確認が完了し、後続MVP（STT、Docs同期、LLM要約）の実装基盤が確立されました。

### Sub-Specifications Progress

#### 📦 meeting-minutes-core (MVP0 - Walking Skeleton) ✅ 完成
- **目的**: Tauri + Python + Chrome拡張の最小疎通確認（Fake実装）
- **フェーズ**: Implementation Complete ✅
- **完了日**: 2025-10-10
- **完了した実装**:
  - ✅ 3プロセスアーキテクチャE2E疎通確認（録音→処理→配信→表示）
  - ✅ Fake音声録音（100ms間隔でダミーデータ生成）
  - ✅ Pythonサイドカープロセス管理（起動/終了/ヘルスチェック）
  - ✅ JSON IPC通信（Rust ↔ Python）
  - ✅ WebSocketサーバー（Rust ↔ Chrome拡張）
  - ✅ Chrome拡張スケルトン（Google Meetページで動作）
- **主要ADR**:
  - ADR-004: Chrome拡張WebSocket管理（Content Script方式採用）
  - ADR-005: chrome.storage.local状態管理メカニズム
- **ドキュメント**:
  - chrome-storage-best-practices.md
  - mvp0-known-issues.md

#### 🎤 meeting-minutes-stt (MVP1 - Real STT)
- **目的**: faster-whisper統合、webrtcvad統合、音声デバイス管理
- **フェーズ**: Design Validated ✅
- **ステータス**: 設計検証完了、タスク生成準備中（MVP0完了）
- **主要成果**:
  - ADR-001: 録音責務の一元化（Rust側のみ）
  - ADR-002: ハイブリッドモデル配布戦略
  - ADR-003: IPCバージョニング方針
  - Pre-commit hooks + 静的解析基盤の整備
- **次のステップ**: `/kiro:spec-tasks meeting-minutes-stt` でタスク生成

#### 📄 meeting-minutes-docs-sync (MVP2 - Google Docs Sync)
- **目的**: OAuth 2.0認証、Google Docs API統合、Named Range管理
- **フェーズ**: Design Generated 🔵
- **ステータス**: 設計生成完了、検証待ち（MVP1完了後）
- **主要成果**:
  - MV3 Service Worker対応設計（chrome.alarms + Offscreen Document）
  - Optimistic Locking戦略（writeControl.requiredRevisionId）
  - Token Bucket RateLimiter設計
  - ADR-004/005: Chrome拡張アーキテクチャ基盤（MVP0で完成）

#### 🔧 meeting-minutes-ci (Infrastructure - CI/CD)
- **目的**: GitHub Actions CI/CD、クロスプラットフォームテスト、自動リリース
- **フェーズ**: Spec Initialized ✅
- **ステータス**: 要件定義完了、設計待ち
- **主要成果**:
  - クロスプラットフォームテストマトリックス設計
  - コスト最適化戦略（平行度制御、キャッシュ）
  - セキュリティ/パフォーマンステスト基盤

#### 🤖 meeting-minutes-llm (MVP3 - LLM Summary + Production UI)
- **目的**: プロダクション準備、UI/UX完成度向上
- **フェーズ**: Not Started ⚪
- **ステータス**: 要件定義待ち（MVP0-2完了後）

### Implementation Progress

現在のプロジェクト状態（2025-10-10更新）:
- ✅ **MVP0 Walking Skeleton**: E2E疎通確認完了
  - 3プロセス間通信（Tauri ↔ Python ↔ Chrome拡張）動作確認
  - Fake音声録音・処理・配信フロー検証
  - WebSocket通信とchrome.storage.local状態管理実装
- ✅ **Architecture Decision Records**: 主要技術決定を文書化
  - ADR-001〜003（MVP1: STT関連）
  - ADR-004〜005（MVP0: Chrome拡張アーキテクチャ）
- ✅ **Development Infrastructure**: 開発基盤整備完了
  - Pre-commit hooks + 静的解析（forbidden imports check）
  - ドキュメント整備（chrome-storage-best-practices.md等）
  - CI/CD spec初期化（meeting-minutes-ci）
- ✅ **Quality Foundation**: コーディング規約とテスト基準確立
  - Rust: `cargo clippy -D warnings`
  - Python: `mypy --strict`
  - TypeScript: `eslint`

**次のステップ**: `/kiro:spec-tasks meeting-minutes-stt` でMVP1タスク生成

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