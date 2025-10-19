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

#### Phase 7: 統合テスト・E2Eテスト（コア機能検証）
**期間**: 2-3日
**優先度**: 🔴 P0 Critical（Gap分析：Option B検証優先アプローチ）
**成果物**:
- 統合テストスイート（Task 10.1-10.7）
- クロスプラットフォームE2Eテスト
- パフォーマンステスト基盤

**タスク**:
1. **Task 10.1**: 音声録音→VAD→STT→保存の完全フロー検証
   - RealAudioDevice→Sidecar→VAD→WhisperSTTEngine→LocalStorageの統合
   - 部分テキスト/確定テキスト配信検証
   - ローカルストレージ保存検証
2. **Task 10.2-10.7**: ⚠️ **Phase 13に移管**（検証負債解消）
   - **Task 10.1完了**: VAD→STT完全フローE2E（23.49秒で緑化）✅
   - **Task 10.2-10.7延期**: Rust E2Eテスト7件は`#[ignore]` + `unimplemented!()`
   - **Phase 13.1で実装**:
     - 13.1.1: オフラインモデルフォールバックE2E（Task 10.2）
     - 13.1.2: 動的モデルダウングレードE2E（Task 10.3）
     - 13.1.3: デバイス切断/再接続E2E（Task 10.4）
     - 13.1.4: クロスプラットフォームE2E（Task 10.5）
     - 13.1.5: 非機能要件E2E（Task 10.7）
     - 13.1.6: IPC/WebSocket後方互換性E2E（Task 10.6）
   - **詳細**: `tasks/phase-13-verification.md` 参照

**受け入れ基準**:
- ✅ Task 10.1完了（VAD→STT完全フロー、23.49秒緑化）
- ⏸️ Task 10.2-10.7は**Phase 13で完了**（検証負債解消）

**検証対象要件**:
- ✅ STT-REQ-001/002/003/004/005/006 (コア機能、Python単体テスト完了)
- ⏸️ STT-REQ-007/008 (IPC/WebSocket拡張、**Phase 13で統合検証**)
- ⏸️ NFR-STT-001/002/003 (非機能要件、**Phase 13.1.5/13.2で検証**)

---

#### Phase 8: リソース監視統合テスト修正
**期間**: 1日
**優先度**: 🟡 P1 High（Gap分析：Pythonテスト4件失敗対応）
**成果物**:
- リソース監視統合テスト安定化（test_audio_integration.py）
- Task 11.6 詳細Metrics実装

**タスク**:
1. 失敗テスト修正（4件）:
   - `test_model_downgrade_on_high_cpu`
   - `test_model_downgrade_on_high_memory`
   - `test_upgrade_proposal_on_recovery`
   - `test_recording_pause_notification`
2. 詳細Metricsログ実装:
   - `mutex_contention_count`: Mutex競合発生回数
   - `callback_duration_us`: Audio callback実行時間
   - `ipc_latency_ms`: IPC通信レイテンシ

**検証対象要件**:
- STT-REQ-006 (リソースベースモデル選択)
- STT-REQ-IPC-004/005 (IPC監視)

---

#### Phase 9: UI拡張とユーザー設定機能
**期間**: 5-7日
**優先度**: 🔴 P0 Critical（Gap分析：100% Gap、MVP1必須）
**成果物**:
- React UIコンポーネント（Task 9.1-9.5）
- 設定保存機能統合
- UIテスト

**タスク**:
1. **Task 9.1**: 音声デバイス選択UI
   - デバイス一覧表示コンポーネント
   - デバイス選択ドロップダウン
   - 設定保存機能
2. **Task 9.2**: Whisperモデル選択UI
   - モデルサイズ選択ドロップダウン（tiny〜large-v3）
   - 自動選択/カスタマイズトグル
   - リソース超過警告表示
3. **Task 9.3**: オフラインモード設定UI（オプション、MVP1スコープ外候補）
   - オフラインモード強制チェックボックス
   - バンドルモデル使用状態表示
4. **Task 9.4**: リソース監視通知UI（オプション、MVP1スコープ外候補）
   - トースト通知コンポーネント
   - モデル切り替え通知表示
5. **Task 9.5**: セッション管理UI（オプション、MVP1スコープ外候補）
   - セッション一覧表示
   - 音声再生/削除機能

**実装優先度**:
- **MVP1必須**: Task 9.1, 9.2（デバイス/モデル選択）
- **MVP1オプション**: Task 9.3, 9.4, 9.5（設定/通知/履歴）

**検証対象要件**:
- STT-REQ-001.3 (デバイス選択)
- STT-REQ-006.4 (モデル選択)

---

### 実装戦略: Option B（検証優先アプローチ）

**Gap分析に基づく推奨実装順序**:

```
Phase 1-6（既存実装完了） ✅
  ↓
Phase 7: E2Eテスト（Task 10）      ← 統合問題早期発見
  ↓
Phase 8: リソース監視修正           ← テスト安定化
  ↓
Phase 9: UI拡張（Task 9）          ← 安定基盤上でUI実装
  ↓
Phase 10: 品質保証・ドキュメント    ← 最終仕上げ
```

**理由**:
- 🟢 **リスク低減**: E2Eテスト先行により統合問題を早期発見
- 🟢 **品質優先**: TDD思想に準拠、UI実装時は安定基盤
- 🟡 **デモ遅延**: UI完成まで手動テスト困難（トレードオフ許容）

**Alternative（Option A: UI優先）**:
- Phase 9（UI） → Phase 7（E2E） → Phase 8（修正）
- メリット: デモ準備が最速
- デメリット: 統合問題発見が遅れる（高リスク）

---

### 総見積もり（更新版）
- **Phase 1-6**: 完了済み（実装完了度85%）
- **Phase 7-9**: 8-11日（E2Eテスト3日 + 修正1日 + UI 5-7日）
- **Phase 10**: 2-3日（ドキュメント、UML図、最終検証）
- **総実装期間**: 10-14日（MVP1完成）

**クリティカルパス（Option B）**:
- Phase 7（E2Eテスト）→ Phase 8（修正）→ Phase 9（UI）→ Phase 10（品質保証）

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
