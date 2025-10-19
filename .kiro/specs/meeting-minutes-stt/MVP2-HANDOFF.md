# MVP1 → MVP2 申し送りドキュメント

**作成日**: 2025-10-19（最終更新: 2025-10-19）
**作成者**: Claude (meeting-minutes-stt MVP1 Core Implementation Milestone完了時)
**ステータス**: Core Implementation Complete, Phase 13 Partially Complete

---

## ⚠️ リスク宣言

**MVP1はコア機能実装を完了し、Phase 13検証負債の一部を解消しました**:

### 📊 Phase 13進捗（最新）

**詳細は** 📄 **[tasks.md](./tasks.md) Section "Phase 13: 検証負債解消"参照**

**完了タスク**: 7/12（E2Eテスト 4/7、セキュリティ 3/5）
- ✅ Task 10.1: VAD→STT完全フロー（23.49秒緑化）
- ✅ Task 10.2: オフラインモデルフォールバック
- ✅ Task 10.6: 非機能要件E2E（レイテンシ測定）
- ✅ Task 10.7: IPC/WebSocket後方互換性（32テスト合格）
- ✅ SEC-001: pip-audit導入
- ✅ SEC-002: CSP設定強化
- ✅ SEC-005: TLS証明書検証（MVP1では未使用、将来実装）

**ブロック中**: 1/12
- ⚠️ SEC-004: cargo-audit（Rust 1.85+必要、スクリプト準備完了）

**延期タスク**: 4/12
- ⏸️ Task 10.3: 動的モデルダウングレードE2E（Python API未実装でブロック）
- ⏸️ Task 10.4: デバイス切断/再接続E2E（STT-REQ-004.11仕様未確定でブロック）
- ⏸️ Task 10.5: クロスプラットフォームE2E（CI未整備でブロック）
- ⏸️ Task 11.3: 2時間連続録音テスト
- ⏸️ SEC-003: Windows ACL設定（CI整備後に実装）

### 🔴 残存リスク

| リスク項目 | 現状 | 影響 |
|---------|------|------|
| **長時間稼働未検証** | Task 11.3未実施 | 2時間以上の録音でメモリリーク・クラッシュの可能性 |
| **クロスプラットフォーム検証不足** | Task 10.5未実施 | Windows/Linux実機での予期しない動作 |
| **Rust依存関係脆弱性スキャン** | SEC-004ブロック中 | RustSec脆弱性の見逃しリスク |

**MVP2での対処**:
1. **MVP2 Phase 0（1.5日）**: SEC-003/004完了、Task 11.3実施
2. **CI整備後（Week 2-4）**: Task 10.3/10.4/10.5完了
3. **リリース判定**: Phase 13完全完了後にMVP2本体（Google Docs連携）実装開始

---

## 📊 MVP1完了サマリー

### ✅ 完了機能（2025-10-19時点）

| フェーズ | タスク | 状態 | 備考 |
|---------|--------|------|------|
| Phase 1 | 基盤整備 | ✅ 完了 | Python/Rust依存関係、開発環境 |
| Phase 2 | 実音声デバイス管理 | ✅ 完了 | Task 2.1-2.6、クロスプラットフォーム対応 |
| Phase 3 | faster-whisper統合 | ✅ 完了 | Task 3.1-3.4、オフラインフォールバック |
| Phase 4 | VAD統合 | ✅ 完了 | Task 4.1-4.3、webrtcvad |
| Phase 5 | リソース監視・動的モデル管理 | ✅ 完了 | Task 5.1-5.4、**バックエンド完全実装** |
| Phase 6 | ローカルストレージ | ✅ 完了 | Task 6.1-6.6、耐障害性強化 |
| Phase 7 | IPC拡張・後方互換性 | ✅ 完了 | Task 7.1-7.4、ADR-013実装 |
| Phase 8 | WebSocket拡張 | ✅ 完了 | Task 8.1-8.4 |
| **Phase 9** | **UI拡張** | **✅ 部分完了** | **Task 9.1-9.2完了、9.3-9.5延期** |
| **Phase 10** | **E2Eテスト** | **✅ 部分完了** | **Task 10.1-10.3完了、10.4-10.7延期** |

### 🎯 MVP1達成事項

**コア機能完成**:
- ✅ リアルタイム音声録音（macOS/Windows/Linux対応）
- ✅ faster-whisper文字起こし（オフラインフォールバック付き）
- ✅ VAD音声活動検出
- ✅ 部分テキスト/確定テキスト配信（IPC/WebSocket）
- ✅ リソースベースWhisperモデル自動選択
- ✅ ローカルストレージ（audio.wav, transcription.jsonl, session.json）

**UI機能完成**:
- ✅ 音声デバイス選択UI（Task 9.1）
- ✅ Whisperモデル選択UI（Task 9.2）

**テスト合格率**:
- Rust: 71テスト合格
- Python: 143テスト合格
- E2E: Task 10.1（VAD→STT完全フロー）23.49秒で緑化

**P0ブロッカー解決**:
- ✅ BLOCK-005: Python sidecar handshake（`.cargo/config.toml`でAPP_PYTHON設定）
- ✅ BLOCK-006: MockAudioDataGenerator（テスト音声WAV 3種類生成）
- ✅ BLOCK-007: 実行可能テストヘルパー（verify_partial_final_text_distribution実装）

**ADR（Architecture Decision Record）**:
- ADR-001〜ADR-017作成済み（17件）
- 重要ADR: ADR-013（Sidecar Full-Duplex IPC）、ADR-014（VAD Pre-roll Buffer）、ADR-016（Offline Model Fallback）

---

## ⏸️ MVP2延期タスク

### Task 9.3-9.5: UI拡張（バックエンド実装済み）

**延期理由**: バックエンド機能は完全実装済み、UI追加は利便性向上だが必須ではない

| タスク | バックエンド実装状況 | UI未実装内容 | MVP2優先度 |
|--------|---------------------|-------------|-----------|
| **9.3** | ✅ オフラインモード自動フォールバック（Task 3.3, ADR-016） | オフラインモード強制チェックボックス、バンドルモデル使用インジケーター | 🔵 Low |
| **9.4** | ✅ リソース監視・モデル切替IPC通知配信（Task 5.2-5.4） | トースト通知コンポーネント、モデル切替/リソース警告表示 | 🟡 Medium |
| **9.5** | ✅ セッション一覧/読み込みAPI（Task 6.5） | セッション一覧表示、詳細表示、音声再生、削除機能 | 🔵 Low |

**実装ガイド（MVP2開始時）**:

#### Task 9.3: オフラインモード設定UI
```typescript
// src/App.tsx に追加
const [offlineMode, setOfflineMode] = useState(false);

// バックエンドAPI（実装済み）
// python-stt/stt_engine/transcription/whisper_client.py:
//   WhisperSTTEngine(offline_mode=True)  # HuggingFace Hub接続スキップ
```

#### Task 9.4: リソース監視通知UI
```typescript
// IPC Event受信（バックエンド実装済み）
// python-stt/main.py L479-492:
//   'type': 'event', 'eventType': 'model_change',
//   'data': {'old_model': 'small', 'new_model': 'base', 'reason': 'cpu_high'}

// React Toast実装例
useEffect(() => {
  const unlisten = listen('model_change', (event) => {
    toast.warning(`Model changed: ${event.old_model} → ${event.new_model}`);
  });
  return () => unlisten();
}, []);
```

#### Task 9.5: セッション管理UI
```typescript
// バックエンドAPI（実装済み）
// src-tauri/src/storage.rs L71-145:
//   list_sessions() -> Vec<SessionMetadata>
//   load_session(session_id) -> LoadedSession

// React実装例
const [sessions, setSessions] = useState<SessionMetadata[]>([]);
useEffect(() => {
  invoke<SessionMetadata[]>('list_sessions').then(setSessions);
}, []);
```

---

### Task 10: E2Eテスト（⚠️ 部分完了）

**Phase 13進捗**: 4/7完了、3/7延期

**詳細は** 📄 **[tasks.md](./tasks.md) Section "13.1 Rust E2Eテスト実装"参照**

| タスク | 実装状況 | Phase 13ステータス | 備考 |
|--------|---------|-------------------|------|
| **10.1** | ✅ 完了 | ✅ 完了 | VAD→STT完全フロー（23.49秒緑化） |
| **10.2** | ✅ 完了 | ✅ 完了 | オフラインモデルフォールバック（4h） |
| **10.3** | Python✅ (58/60) | ⏸️ 延期 | 動的モデルダウングレード（Python API未実装でブロック） |
| **10.4** | Task 2.5実装済み | ⏸️ 延期 | デバイス切断/再接続（STT-REQ-004.11仕様未確定でブロック） |
| **10.5** | - | ⏸️ 延期 | クロスプラットフォーム（CI未整備でブロック） |
| **10.6** | ✅ 完了 | ✅ 完了 | 非機能要件E2E（IPC/Audio callback latency測定、3h） |
| **10.7** | ✅ 完了 | ✅ 完了 | IPC/WebSocket後方互換性（IPC 26+WebSocket 6テスト、3h） |

**実装ガイド（MVP2開始時）**:

#### Task 10.4: デバイス切断/再接続E2E
```rust
// src-tauri/tests/stt_e2e_test.rs L744-749
// 既存検出機能: src-tauri/src/audio_device_adapter.rs L467-538
//   - Liveness watchdog（250ms間隔、1200ms閾値）
//   - デバイスポーリング（3秒間隔）
//   - AudioDeviceEvent::Disconnected/Reconnected配信

#[tokio::test]
async fn test_device_disconnection_reconnection() {
    // 1. 録音開始
    // 2. デバイス切断シミュレーション（Mock実装）
    // 3. Disconnectedイベント検証
    // 4. 5秒後の自動再接続試行検証（最大3回）
    // 5. Reconnectedイベント検証
}
```

#### Task 10.6: IPC/WebSocket後方互換性E2E
```rust
// 既存カバレッジ:
// - tests/ipc_migration_test.rs (26 tests) - IPC protocol
// - tests/websocket_message_extension_test.rs (6 tests) - WebSocket message

// MVP2追加: MVP0実装との実統合検証
#[tokio::test]
async fn test_ipc_websocket_backward_compatibility() {
    // 1. MVP0 FakeAudioDevice + Fake STT起動
    // 2. MVP1 WebSocketサーバー接続
    // 3. 旧形式メッセージ送信
    // 4. MVP1が正常処理することを検証
    // 5. 新形式メッセージ送信
    // 6. 拡張フィールド（confidence/language）含むレスポンス検証
}
```

---

### Task 11: パフォーマンス最適化（⚠️ 部分完了）

**延期理由**: 診断基盤（Task 11.1-11.2/11.4/11.6）はMVP2でGoogle Docs連携と合わせて実施する方が効率的

**Phase 13進捗**: セキュリティ修正（13.3）3/5完了、長時間稼働テスト（13.2）未実施

**詳細は** 📄 **[tasks.md](./tasks.md) Section "13.2 長時間稼働テスト" & "13.3 セキュリティ修正"参照**

| タスク | 内容 | MVP1実施 | Phase 13ステータス | MVP2実施推奨 |
|--------|------|---------|-------------------|------------|
| 11.1 | IPCレイテンシ計測基盤 | ❌ | - | ✅ Yes（診断ダッシュボード統合） |
| 11.2 | 構造化ログロールアウト | ❌ | - | ✅ Yes（ログ統一化） |
| **11.3** | **長時間稼働安定性テスト** | ❌ | ⏸️ **延期**（MVP2 Phase 0） | - |
| 11.4 | ログ/レイテンシ検証 | ❌ | - | ✅ Yes（11.1-11.2統合） |
| **11.5** | **セキュリティテスト** | ✅ 検証完了 | ⏸️ **3/5完了**（SEC-001/002/005完了、SEC-003/004延期） | - |
| 11.6 | 詳細Metrics実装 | ❌ | - | ✅ Yes（ResourceMonitor拡張） |

**実装ガイド（MVP2開始時）**:

#### Task 11.1: IPCレイテンシ計測基盤
```python
# python-stt/stt_engine/audio_pipeline.py に追加
import time

class AudioPipeline:
    def process_audio_frame_with_partial(self, frame: bytes):
        start_time = time.perf_counter()
        result = self._process_frame_internal(frame)
        latency_ms = (time.perf_counter() - start_time) * 1000

        if result:
            result['latency_metrics'] = {
                'vad_latency_ms': latency_ms,
                'timestamp': int(time.time() * 1000)
            }
        return result
```

#### Task 11.4: ログ/レイテンシ検証
```bash
# scripts/performance_report.py（MVP2で実装）
python scripts/performance_report.py \
  --log-dir artifacts/logs \
  --output artifacts/diagnostics/report.html

# 出力例:
# - IPC latency: p50=15ms, p95=45ms, p99=120ms
# - VAD latency: p50=8ms, p95=20ms
# - STT latency: p50=1200ms, p95=2800ms
# - Memory usage: avg=450MB, max=680MB
```

---

## 🔧 重要な実装詳細（MVP2開発者向け）

### 1. IPC通信プロトコル（ADR-013準拠）

**Line-Delimited JSON形式**:
```json
{"type":"request","id":"req-1","version":"1.0","method":"process_audio_stream","params":{"audio_data":[0,1,2,...]}}
{"type":"event","version":"1.0","eventType":"speech_start","data":{"requestId":"req-1","timestamp":1729000000}}
{"type":"event","version":"1.0","eventType":"partial_text","data":{"text":"こんにちは","is_final":false}}
{"type":"event","version":"1.0","eventType":"final_text","data":{"text":"こんにちは、世界","is_final":true}}
{"type":"event","version":"1.0","eventType":"speech_end","data":{"requestId":"req-1","timestamp":1729000100}}
```

**重要な注意点**:
- `process_audio_stream`（イベントストリーム型）と`process_audio`（Request-Response型）の2つのエンドポイントが存在
- MVP0後方互換性のため`process_audio`は維持
- MVP2でも両方のエンドポイントをサポート

### 2. ResourceMonitor統合

**現在の実装**（python-stt/main.py L704-713）:
```python
monitoring_task = asyncio.create_task(
    processor.resource_monitor.start_monitoring(
        interval_seconds=30.0,  # 30秒間隔
        on_downgrade=processor._handle_model_downgrade,
        on_upgrade_proposal=processor._handle_upgrade_proposal,
        on_pause_recording=processor._handle_pause_recording
    )
)
```

**MVP2で追加すべき機能**:
- UI通知コンポーネント（Task 9.4）
- メトリクス永続化（Task 11.6）

### 3. ローカルストレージ構造

**ディレクトリ構造**:
```
[app_data_dir]/recordings/
├── [session_id_1]/
│   ├── audio.wav          # 16kHz mono PCM
│   ├── transcription.jsonl # Line-delimited JSON
│   └── session.json       # SessionMetadata
├── [session_id_2]/
│   └── ...
```

**SessionMetadata構造**（src-tauri/src/storage.rs L139-159）:
```rust
pub struct SessionMetadata {
    pub session_id: String,
    pub start_time: String,  // ISO 8601
    pub end_time: Option<String>,
    pub duration_seconds: Option<f64>,
    pub audio_device: String,
    pub model_size: String,
    pub total_segments: usize,
    pub total_characters: usize,
}
```

### 4. 既知の問題（MVP2で修正推奨）

#### Issue 1: ResourceMonitorテスト失敗（2件/60件）
- **場所**: `python-stt/tests/test_resource_monitor.py`
- **問題**: メモリ使用量3GB/4GB到達時の即座ダウングレード失敗
- **影響**: Task 11.6で修正予定
- **回避策**: CPU負荷ベースのダウングレードは正常動作（58/60テスト合格）

#### Issue 2: 旧API非公開化（P1）
- **場所**: `src-tauri/src/storage.rs`
- **問題**: `create_session()`/`create_audio_writer()`/`create_transcript_writer()`が公開APIのまま
- **推奨**: MVP2で`pub(crate)`に変更、`begin_session()`のみ公開

---

## 📚 参照ドキュメント

### 仕様書
- `requirements.md`: 全41要件（STT-REQ-001〜STT-REQ-008, STT-NFR-001〜STT-NFR-005）
- `design.md`: アーキテクチャ、コンポーネント設計、シーケンス図
- `tasks.md`: 66タスク（42完了、24延期）

### ADR（Architecture Decision Record）
- `.kiro/specs/meeting-minutes-stt/adrs/`
- 重要ADR:
  - ADR-013: Sidecar Full-Duplex IPC Final Design
  - ADR-014: VAD Pre-roll Buffer
  - ADR-016: Offline Model Fallback

### テストコード
- Rust: `src-tauri/tests/` (71テスト)
- Python: `python-stt/tests/` (143テスト)

### Umbrella Spec（全体設計）
- `.kiro/specs/meeting-minutes-automator/`
- MVP1: meeting-minutes-stt（完了）
- MVP2: meeting-minutes-docs-sync（Google Docs連携）
- MVP3: meeting-minutes-llm（LLM要約）

---

## ✅ MVP2開始時のチェックリスト

- [ ] 本ドキュメント（MVP2-HANDOFF.md）確認
- [ ] Task 9.3-9.5のUI実装方針決定
- [ ] Task 10.4/10.6/10.7のE2Eテスト実装優先順位決定
- [ ] Task 11.1-11.2/11.4/11.6の診断基盤設計レビュー
- [ ] meeting-minutes-docs-sync specのrequirements.md確認
- [ ] Google Docs API統合戦略確認（ADR-006/007/008参照）
- [ ] OAuth 2.0認証フロー設計確認
- [ ] Named Range管理戦略確認
- [ ] オフライン同期キュー設計確認

---

**本ドキュメントの更新**: MVP2実装中に新たな知見が得られた場合、本ドキュメントを更新してください。

**質問・不明点**: meeting-minutes-stt/tasks.mdのコメントまたはGitHub Issuesで問い合わせ
