# Technical Design Document — Multi-Input Audio Mixing

## 1. Overview

本設計は meeting-minutes-stt の拡張として、**複数入力デバイス（マイク＋ループバック等）の同時録音とアプリ内ミックス**を実現する。Python側STT処理やIPCプロトコルは変更せず、Rust側の入力層に **Input Mixer** を追加することで互換性を保つ。

**関連要件**: STTMIX-REQ-001〜008, STTMIX-NFR-Perf/Rel, STTMIX-CON-001〜004

---

## 2. Goals / Non-Goals

**Goals**
- マイク＋ループバック等の複数入力を同時取得
- 16kHz mono に正規化したミックス出力を既存パイプラインへ供給
- OS側Aggregate Deviceに依存しない

**Non-Goals**
- 話者分離・話者同定
- AEC/NR等の高度DSP
- Python側のIPC/Whisper処理変更

---

## 3. Architecture Summary

```
Input Devices (N)
   ├─ cpal Stream (per device)
   ├─ Per-Input Buffer
   └─ Resampler + Downmixer
                │
                ▼
           Input Mixer (10ms frame align)
                │
                ▼
        Existing Ring Buffer (5s)
                │
                ▼
        Batch Sender → Python STT
```

**変更点**: 既存の AudioDeviceAdapter を拡張し、**複数ストリームの並列取得**と**Input Mixer**の追加を行う。  
**影響範囲**: Rust側の入力レイヤー/UI設定のみ。Python側は変更なし。

**関連要件**: STTMIX-REQ-002/003/004/007

---

## 4. Component Design

### 4.0 AudioDeviceRecorder (Facade)

- **役割**: 単一入力 / 複数入力の統一インターフェースを提供するFacade
- **設計意図**: 既存 `AudioDeviceAdapter` trait を変更せず、上位層で入力モードを切り替え
- **構成（ファクトリパターン）**:

```rust
/// Factory for creating independent adapter instances
pub type AdapterFactory = Arc<dyn Fn() -> Result<Box<dyn AudioDeviceAdapter>> + Send + Sync>;

pub struct AudioDeviceRecorder {
    adapter_factory: AdapterFactory,  // Factory pattern for multi-instance support
    single_adapter: Option<Box<dyn AudioDeviceAdapter>>,  // Active adapter for single mode
    mode: Option<RecordingMode>,
    is_recording: bool,
}

pub enum RecordingMode {
    Single { device_id: String },
    Multi { device_ids: Vec<String>, mixer_config: MixerConfig },
}

impl AudioDeviceRecorder {
    pub fn new(adapter_factory: AdapterFactory) -> Self { ... }
    pub fn start(&mut self, mode: RecordingMode, callback: AudioChunkCallback) -> Result<()> {
        match &mode {
            RecordingMode::Single { device_id } => {
                let mut adapter = (self.adapter_factory)()?;
                adapter.start_recording_with_callback(device_id, callback)?;
                self.single_adapter = Some(adapter);
            }
            RecordingMode::Multi { device_ids, mixer_config } => {
                // Task 2.x: MultiInputManager uses adapter_factory
                // to create independent adapters for each device
            }
        }
    }
    pub fn adapter_factory(&self) -> &AdapterFactory { ... }  // For MultiInputManager
}
```

- **なぜファクトリパターン？**:
  - 既存 AudioDeviceAdapter は単一ストリーム設計（device_id, stream_thread等を保持）
  - 同一インスタンスで複数デバイスの並列録音は不可能
  - ファクトリにより独立したアダプタインスタンスを動的生成可能
- **既存コードへの影響**: なし（AudioDeviceAdapter trait は変更しない）
- **AppState 拡張**: `audio_device_recorder: Mutex<Option<AudioDeviceRecorder>>` を追加

**関連要件**: STTMIX-CON-001 (後方互換)

### 4.1 MultiInputManager (Rust)
- 役割: 複数デバイスの開始/停止、入力ストリームのライフサイクル管理
- 入力: `Vec<DeviceId>`、各デバイスの `InputRole` / `Gain` / `Mute`
- 出力: Mixerへの per-input buffer 参照

**関連要件**: STTMIX-REQ-001/002/006

### 4.2 InputCapture (per-device)
- 役割: `cpal` input stream から f32 PCM を取得
- 変換: `f32` → `i16`、チャンネル数に応じた downmix
- 出力: per-input buffer (ring buffer)

**関連要件**: STTMIX-REQ-002/003

### 4.3 Resampler + Downmixer

- **役割**: 入力サンプルレートを 16kHz へ正規化
- **選択**: 既存実装と同様、cpalコールバック内での**平均化ダウンサンプリング**を採用
- **実装方式**:
  - ネイティブサンプルレート（例: 48kHz）から16kHzへの変換
  - N サンプル（N = native_rate / 16000）を平均化してダウンサンプル
  - 平均化は簡易的なローパスフィルタとして機能し、エイリアシングを軽減
- **理由**:
  1. 既存 `audio_device_adapter.rs` の平均化ダウンサンプリングは音声認識品質に十分
  2. 追加クレート（rubato等）導入によるビルド複雑化を回避
  3. NFR-Perf-001（p95 ≤ 20ms）は既存手法で達成可能
- **ダウンミックス**: ステレオ入力はL/R平均でモノラル化
- **代替案**: 高品質リサンプリングが必要な場合は `rubato` を検討（ADRで判断）

**関連要件**: STTMIX-REQ-003

### 4.4 Input Mixer
- 役割: per-input buffer から 10ms単位でフレームを取り出し、**時間整列**してミックス
- 出力: 16kHz mono 10msフレーム（160 samples）

**関連要件**: STTMIX-REQ-004/005

---

## 5. Mixing Algorithm

1. 各入力フレーム（10ms）を f32 に変換
2. 入力ごとに `gain` を適用（デフォルト -6dB 相当）
3. すべての入力を加算
4. `[-1.0, 1.0]` にクランプ → i16 PCMへ変換

**クリップ対策**
- 事前にゲインを抑える
- クリップ発生回数をメトリクス化
- 必要なら簡易リミッタ（TBD）

**関連要件**: STTMIX-REQ-005

---

## 6. Synchronization & Drift Correction

**同期方針**
- Mixerは**10ms cadence**でフレームを生成
- 各入力から 10ms 分のサンプルが揃っていない場合は、短期的に待機または無音補完

**ドリフト補正**
- **検出しきい値**: ±10サンプル（±0.625ms @ 16kHz）
- **補正方式**: しきい値超過時に1サンプルの間引き（遅れ入力）または複製（進み入力）
- **補正頻度上限**: 100ms あたり最大1回（過剰補正防止）
- **初期値の根拠**:
  - 10サンプル = 0.625ms はVADのフレーム境界（10ms）の約6%
  - 人間の聴覚では知覚不能なレベル
  - 実機測定で調整可能（タスクで実験枠を設ける）
- 補正回数をメトリクスとして記録（drift_correction_count）

**関連要件**: STTMIX-REQ-004/008

---

## 7. Error Handling & Degradation

- **単一入力の喪失**: UI通知 → 残存入力で継続（設定で停止も可）
- **全入力喪失**: 録音停止 + エラー通知
- **バッファ枯渇**: 無音フレームで補完し、欠損をメトリクス化

**関連要件**: STTMIX-REQ-006

---

## 8. IPC Compatibility

- 送信フォーマットは既存 `process_audio_stream` を継続
- Python側のVAD/Whisperは変更なし
- 出力は **16kHz mono 16-bit PCM**

**関連要件**: STTMIX-REQ-007, STTMIX-CON-002

---

## 9. Observability & Metrics

**メトリクス例**
- input_buffer_level{device}
- mix_drop_frames_count
- drift_correction_count
- clip_count
- mix_latency_ms

**関連要件**: STTMIX-REQ-008

---

## 10. Configuration & Persistence

**設定項目（案）**
- `multi_input.enabled`
- `multi_input.selected_device_ids`
- `multi_input.input_roles`
- `multi_input.gains`
- `multi_input.mute`
- `multi_input.degradation_policy`（continue/stop）

**保存先**: 既存の設定保存方式（localStorage / config）に合わせる

**関連要件**: STTMIX-REQ-001/005/006

---

## 11. Security / Privacy

- 音声はローカル処理のみ
- OS設定の自動変更は行わない
- 追加の外部通信は発生しない

**関連要件**: STTMIX-CON-001/003

---

## 12. Testing Strategy

1. **Unit**: Mixerアルゴリズム（ゲイン・クリップ・ドリフト補正）
2. **Integration**: 2入力の同時キャプチャ + ミックス → 16kHz mono出力
3. **E2E**: マイク＋ループバックでの実機確認
4. **Regression**: 既存の単一入力動作が維持されること

---

## 13. Open Questions

- 初期ターゲットOSは macOS のみで良いか（STTMIX-CON-004）
- ~~ドリフト補正のしきい値と補正方式の最終決定~~ → §6で初期値を確定（±10サンプル、実機測定で調整）
- ゲイン/UIのデフォルト値とUX設計

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-09 | 0.1 | Codex | 初版（Multi-Input設計） |
| 2026-01-09 | 0.2 | Claude | GO条件対応: §4.0 Facade追加, §4.3 Resampler確定, §6 ドリフト補正しきい値明記 |
| 2026-01-09 | 0.3 | Claude | 構造修正: §4.0 ファクトリパターン採用, §4.3 「線形補間」→「平均化」に修正 |

