# Requirements Document

## Project Description (Input)

meeting-minutes-stt-multi-input: 既存の meeting-minutes-stt (MVP1) に対し、**マイク音声＋内部音声（ループバック）など複数入力を同時に取得し、アプリ側でミックス**してSTTパイプラインへ流す拡張。OS側Aggregate Deviceに依存せず、アプリ内で「自分の声＋会議音声」を収音できるようにする。

## Introduction

現行実装は単一入力デバイス選択のみ対応しており、会議音声と自分の声を同時に収音するにはOS側で仮想デバイスを構成する必要がある。これはユーザー期待と乖離しやすく、設定コストも高い。本specは**複数入力を同時キャプチャしアプリ内でミックス**することで、期待動作を満たすことを目的とする。

**ビジネス価値**:
- 会議音声＋自分の発話を確実に同時収録できる
- OS設定依存を減らし、再現性の高い運用が可能
- 設定体験の簡素化によりオンボーディングが改善

**スコープ制限**:
- 話者分離・話者同定は対象外（混合音声のSTTのみ）
- ノイズ抑制やAEC等の高度DSPは対象外
- 既存のIPCプロトコル/ Python STT処理は変更しない

---

## Glossary

| 用語 | 英語表記 | 定義 |
|-----|---------|------|
| **Input Mixer** | Input Mixer | 複数の入力音声を同期・合成して単一のPCMストリームを生成する層 |
| **入力ロール** | Input Role | 入力デバイスの役割（Microphone / Loopback など） |
| **Mix Frame** | Mix Frame | 10ms単位のミックス対象フレーム（16kHz/160 samples） |
| **ドリフト補正** | Drift Correction | 入力デバイス間のクロック差を補正する処理 |
| **ゲイン** | Gain | 入力ごとの音量倍率（dB/倍率） |

---

## Requirements

### STTMIX-REQ-001: Multi-Input Selection & Persistence

**Objective**: ユーザーとして、複数の音声入力（マイク＋ループバックなど）を同時に選択して録音したい。

#### Acceptance Criteria

1. **STTMIX-REQ-001.1**: WHEN アプリが音声デバイス一覧を表示 THEN システム SHALL 複数デバイス選択を可能にする。
2. **STTMIX-REQ-001.2**: WHEN ユーザーが複数デバイスを選択 THEN システム SHALL 選択内容を保存し、次回起動時に復元する。
3. **STTMIX-REQ-001.3**: WHEN ループバックデバイスが選択される THEN システム SHALL ループバックであることをUI上で明示する。

---

### STTMIX-REQ-002: Parallel Capture

**Objective**: ソフトウェアエンジニアとして、複数の入力デバイスから同時に音声を取得したい。

#### Acceptance Criteria

1. **STTMIX-REQ-002.1**: WHEN 録音開始コマンドが実行 THEN システム SHALL 選択された全デバイスの入力ストリームを並行して開始する。
2. **STTMIX-REQ-002.2**: WHEN いずれかのデバイスの開始に失敗 THEN システム SHALL 失敗理由を通知し、残りのデバイスで継続可能か判定する。
3. **STTMIX-REQ-002.3**: WHEN 録音停止コマンドが実行 THEN システム SHALL 全入力ストリームを停止し、リソースを解放する。

---

### STTMIX-REQ-003: Per-Input Resampling & Downmix

**Objective**: 複数入力を統一フォーマット（16kHz mono）に正規化したい。

#### Acceptance Criteria

1. **STTMIX-REQ-003.1**: WHEN 各入力ストリームから音声が到着 THEN システム SHALL 入力ごとに16kHzへリサンプリングする。
2. **STTMIX-REQ-003.2**: WHEN 入力がステレオ以上 THEN システム SHALL モノラルへダウンミックスする。
3. **STTMIX-REQ-003.3**: WHEN 正規化が完了 THEN システム SHALL 10msフレーム単位で後段に渡す。

---

### STTMIX-REQ-004: Time Alignment & Mixing

**Objective**: 音声の時間ずれを抑え、安定したミックスを生成したい。

#### Acceptance Criteria

1. **STTMIX-REQ-004.1**: WHEN 複数入力をミックスする THEN システム SHALL 10msフレーム境界で時間整列を行う。
2. **STTMIX-REQ-004.2**: IF 入力間のドリフトが検出される THEN システム SHALL サンプルの間引き/補間で補正する。
3. **STTMIX-REQ-004.3**: WHEN ミックスされたフレームが生成される THEN システム SHALL 既存のIPC送信フォーマット（process_audio_stream）で後段へ流す。

---

### STTMIX-REQ-005: Gain & Clipping Control

**Objective**: 入力ごとの音量を調整し、クリッピングを防ぎたい。

#### Acceptance Criteria

1. **STTMIX-REQ-005.1**: WHEN ミックス処理を行う THEN システム SHALL 入力ごとにゲインを適用できる。
2. **STTMIX-REQ-005.2**: WHEN デフォルト設定が適用される THEN システム SHALL 2入力時に過大音量にならないゲイン（例: -6dB相当）を使用する。
3. **STTMIX-REQ-005.3**: IF ミックス後の振幅が上限を超える THEN システム SHALL クリップまたはリミッタで歪みを抑制する。

---

### STTMIX-REQ-006: Degradation & Recovery

**Objective**: 一部入力に問題が発生しても可能な限り録音を継続したい。

#### Acceptance Criteria

1. **STTMIX-REQ-006.1**: WHEN いずれかの入力デバイスが切断 THEN システム SHALL ユーザーに通知する。
2. **STTMIX-REQ-006.2**: IF 1つ以上の入力が継続利用可能 THEN システム SHALL 残存入力のみで録音を継続する（設定で無効化可能）。
3. **STTMIX-REQ-006.3**: IF 全入力が失われた THEN システム SHALL 録音を停止し、エラーを通知する。

---

### STTMIX-REQ-007: IPC Compatibility

**Objective**: 既存のPython側STT処理との互換性を維持したい。

#### Acceptance Criteria

1. **STTMIX-REQ-007.1**: WHEN ミックス済みフレームを送信 THEN システム SHALL 既存のIPCプロトコル（process_audio_stream）を変更しない。
2. **STTMIX-REQ-007.2**: WHEN Python側が受信する音声 THEN システム SHALL 16kHz mono 16-bit PCM である。

---

### STTMIX-REQ-008: Observability

**Objective**: 複数入力の状態とミックス品質を監視したい。

#### Acceptance Criteria

1. **STTMIX-REQ-008.1**: WHEN ミックス処理が稼働 THEN システム SHALL 入力ごとのバッファ占有率を計測できる。
2. **STTMIX-REQ-008.2**: WHEN ドリフト補正やクリップが発生 THEN システム SHALL その回数を記録する。

---

## Non-Functional Requirements (NFR)

### STTMIX-NFR-Perf-001 (Latency)
- **Response Measure**: Input Mixerの追加レイテンシは p95 ≤ 20ms。

### STTMIX-NFR-Perf-002 (CPU)
- **Response Measure**: 2入力時の追加CPU使用率は平均 +5%以内（目安）。

### STTMIX-NFR-Rel-001 (Frame Loss)
- **Response Measure**: 通常負荷下でのフレーム欠損率 ≤ 0.1%。

---

## Constraints

1. **STTMIX-CON-001**: 音声取得はRust側で行い、Python側での録音は行わない。
2. **STTMIX-CON-002**: Python側のIPCフォーマットおよびVAD/Whisper処理は変更しない。
3. **STTMIX-CON-003**: OS側の音声ルーティング設定を自動変更しない。
4. **STTMIX-CON-004**: 初期ターゲットはmacOSを優先し、Windows/Linux対応は後続フェーズで検討可能とする（TBD）。
5. **STTMIX-CON-005**: 初期リリースは**最大2入力**（マイク＋ループバック）をサポートする。3入力以上の対応は将来検討。

---

## Requirement Traceability Matrix

| Requirement ID | Parent Requirement | Design Section | Notes |
|---|---|---|---|
| STTMIX-REQ-001 | Extends: STT-REQ-001.3 | Design §3/§10 | UIと設定保存（単一→複数選択へ拡張） |
| STTMIX-REQ-002 | Extends: STT-REQ-001.4 | Design §4 | 並列キャプチャ（単一→複数ストリーム） |
| STTMIX-REQ-003 | Extends: STT-REQ-001.5 | Design §4/§5 | リサンプル/モノラル化（入力ごと） |
| STTMIX-REQ-004 | New | Design §5/§6 | 同期・ミックス（新規機能） |
| STTMIX-REQ-005 | New | Design §5 | ゲイン/クリップ（新規機能） |
| STTMIX-REQ-006 | Extends: STT-REQ-004.9-11 | Design §7 | 障害時挙動（単一→複数デバイス対応） |
| STTMIX-REQ-007 | Maintains: STT-REQ-001.6 | Design §8 | IPC互換（変更なし） |
| STTMIX-REQ-008 | New | Design §9 | 監視指標（新規機能） |

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-09 | 0.1 | Codex | 初版（複数入力ミックス要件定義） |
| 2026-01-09 | 0.2 | Claude | 親要件リンク追加、STTMIX-CON-005（入力数上限）追加 |
