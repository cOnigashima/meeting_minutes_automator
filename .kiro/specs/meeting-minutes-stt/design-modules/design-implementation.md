## Implementation Plan

### 実装フェーズ

#### Phase 1: 音声デバイス統合 (STT-REQ-001, STT-REQ-004)
**期間**: 2週間
**成果物**:
- `RealAudioDevice` trait実装 (Rust)
- OS別Adapter実装 (WasapiAdapter, CoreAudioAdapter, AlsaAdapter)
- デバイス検出・選択機能
- クロスプラットフォーム音声録音の疎通確認

**タスク**:
1. cpal統合とOS固有音声API抽象化
2. デバイス切断/再接続機構実装
3. クロスプラットフォームテスト (macOS/Windows/Linux)

---

#### Phase 2: VAD統合 (STT-REQ-003)
**期間**: 1週間
**成果物**:
- `VoiceActivityDetector` 実装 (Python)
- webrtcvad統合 (aggressiveness=2)
- 発話開始/終了イベント検出
- Golden Audioでの精度検証

**タスク**:
1. webrtcvad統合とフレーム分割ロジック
2. 発話境界検出ロジック (0.3秒開始、0.5秒終了)
3. 部分テキスト/確定テキスト生成タイミング制御

---

#### Phase 3: faster-whisper統合 (STT-REQ-002)
**期間**: 2週間
**成果物**:
- `WhisperSTTEngine` 実装 (Python)
- オフラインファーストモデルロード戦略
- モデルダウンロード/フォールバック機構
- プロキシ環境対応

**タスク**:
1. faster-whisper統合と推論実装
2. モデル検出優先順位実装 (ユーザー設定→キャッシュ→システム共有パス→バンドル)
3. HuggingFace Hubダウンロードとタイムアウト処理 (10秒)
4. バンドルbaseモデルフォールバック実装
5. プロキシ環境 (HTTPS_PROXY) 対応

---

#### Phase 3.5: モデル配布UI実装 (新規)
**期間**: 0.5週間
**成果物**:
- 初回起動時のモデル選択ダイアログ
- バックグラウンドダウンロードUI (進捗バー + 一時停止/再開)
- システム共有パス検索機能

**タスク**:
1. Tauri UIにモデルダウンロードダイアログ追加
2. ダウンロード進捗表示 (WebSocket経由でPython→Rust→UI)
3. 一時停止/再開/キャンセル機能実装
4. システム共有パス検索ロジック (`/usr/local/share/faster-whisper/` 等)
5. ユーザー選択肢UI: 「今すぐダウンロード」「後でダウンロード」「ローカルモデルを指定」

---

#### Phase 4: リソース管理 (STT-REQ-006)
**期間**: 1.5週間
**成果物**:
- `ResourceMonitor` 実装 (Python)
- 起動時モデル選択ロジック
- 動的ダウングレード機構
- UI通知統合

**タスク**:
1. 起動時システムリソース検出とモデル選択
2. リソース監視ループ (30秒間隔)
3. ダウングレードトリガー実装 (CPU 85%/60s, メモリ 4GB)
4. アップグレード提案ロジック
5. UI通知統合 (WebSocket経由)

---

#### Phase 5: ローカルストレージ (STT-REQ-005)
**期間**: 1週間
**成果物**:
- `LocalStorageService` 実装 (Rust)
- セッション管理機能
- transcription.jsonl形式での保存
- ディスク容量監視

**タスク**:
1. セッションディレクトリ作成とメタデータ保存
2. audio.wav保存 (16kHz mono 16bit PCM)
3. transcription.jsonl保存 (JSON Lines形式)
4. ディスク容量監視とエラーハンドリング (1GB警告、500MB制限)

---

#### Phase 6: IPC/WebSocket拡張 (STT-REQ-007, STT-REQ-008)
**期間**: 1週間
**成果物**:
- IPC通信プロトコル拡張 (version, confidence, language等)
- WebSocketメッセージ拡張
- 後方互換性検証
 - ADR-013に準拠したSidecar Facade・Backpressure実装

**タスク**:
1. IPC通信プロトコルv1.0拡張フィールド追加
2. WebSocketメッセージ拡張フィールド追加
3. meeting-minutes-core (Fake実装) との後方互換性テスト
 4. Sidecar Facade (ADR-013) とリングバッファP0修正の適用確認 (`ADR-013-P0-bug-fixes.md`)

> 参照: `.kiro/specs/meeting-minutes-stt/adrs/ADR-013-sidecar-fullدuplex-final-design.md`（ADR-011/012を統合した最終設計）  
> フォローアップ: `.kiro/specs/meeting-minutes-stt/adrs/ADR-013-P0-bug-fixes.md`

---

#### Phase 7: 統合テスト・E2Eテスト
**期間**: 1.5週間
**成果物**:
- 統合テストスイート
- E2Eテストスイート
- パフォーマンステスト

**タスク**:
1. 音声録音→VAD→STT→保存の完全フロー検証
2. クロスプラットフォーム動作確認 (macOS/Windows/Linux)
3. オフライン動作検証 (バンドルモデル使用)
4. 動的ダウングレードシナリオテスト
5. パフォーマンス目標検証 (処理時間、メモリ使用量)

---

### 総見積もり
- **実装期間**: 約10週間 (2.5ヶ月)
- **並行作業**: Phase 1-2 (音声デバイス + VAD) は並行可能
- **クリティカルパス**: Phase 3 (faster-whisper統合) → Phase 4 (リソース管理) → Phase 7 (統合テスト)

---

## Next Actions

### 直近の実装タスク (優先順位順)

1. **Phase 1開始**: `RealAudioDevice` trait実装とcpal統合 (担当者: Rust開発者、期日: 2週間以内)
2. **バンドルモデル準備**: faster-whisper baseモデルのインストーラーバンドル準備 (担当者: DevOps、期日: Phase 3開始前)
3. **Golden Audio準備**: VAD/STT精度検証用の音声サンプル準備 (担当者: QA、期日: Phase 2開始前)
4. **meeting-minutes-core依存関係確認**: CORE-REQ-004/006/007の完了確認 (担当者: プロジェクトマネージャー、期日: 即座)

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成 (MVP1 Real STT技術設計) |
