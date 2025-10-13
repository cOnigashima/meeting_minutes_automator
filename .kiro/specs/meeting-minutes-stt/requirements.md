# Requirements Document

## Project Description (Input)

meeting-minutes-stt: MVP1 Real STT実装。meeting-minutes-core完了後に実装。faster-whisper統合（モデルダウンロード、推論）、webrtcvad統合（リアルタイムVAD）、リソースベースモデル選択、音声デバイス管理（マイク/ループバック）、ローカルストレージ（録音ファイル保存）。成果物: 実音声→文字起こし→ローカル保存

## Introduction

meeting-minutes-sttは、meeting-minutes-core（Walking Skeleton）で確立した3プロセスアーキテクチャ上に実際の音声処理機能を実装するMVP1フェーズです。このspecでは、Fake実装を実音声処理に置き換え、faster-whisperによる高精度文字起こしとwebrtcvadによる音声活動検出を実現します。

**ビジネス価値**:
- 実用可能な音声文字起こし機能の提供
- ローカル環境での高精度STT処理（faster-whisper）
- リアルタイム音声活動検出による効率的な処理
- オフライン動作によるプライバシー保護

**スコープ制限（MVP1の範囲）**:
- Google Docs連携は含まない（MVP2 meeting-minutes-docs-syncで実装）
- LLM要約生成は含まない（MVP3 meeting-minutes-llmで実装）
- UIは最小限の拡張のみ（本格的なUI洗練はMVP3で実施）
- リソース管理は基本的な3段階閾値のみ実装

**meeting-minutes-coreからの移行**:
- `FakeAudioDevice` → `RealAudioDevice`への置き換え
- `FakeProcessor` → `WhisperSTTEngine` + `VoiceActivityDetector`への置き換え
- IPC通信プロトコルは維持（互換性保証）
- WebSocketメッセージフォーマットの拡張（confidence, speakerSegment等）

---

## Glossary

| 用語 | 英語表記 | 定義 |
|-----|---------|------|
| **faster-whisper** | faster-whisper | OpenAI WhisperのCTranslate2最適化版。高速・低メモリでSTT処理を実行。 |
| **webrtcvad** | webrtcvad | WebRTCのVoice Activity Detection実装。リアルタイムで発話/無音を検出。 |
| **VAD** | Voice Activity Detection | 音声活動検出。発話区間と無音区間を識別する技術。 |
| **音声デバイス** | Audio Device | マイクまたはシステム音声ループバックデバイス。OS固有のAPI経由でアクセス。 |
| **音声セグメント** | Audio Segment | VADで検出された1つの発話区間。STT処理の単位。 |
| **部分テキスト** | Partial Transcription | STT処理中のリアルタイム結果。発話終了前の暫定的な文字起こし。 |
| **確定テキスト** | Final Transcription | 無音検出後に確定した文字起こし結果。 |
| **モデルサイズ** | Model Size | Whisperモデルの種類（tiny/base/small/medium/large）。精度とリソース使用量のトレードオフ。 |
| **リソースベース選択** | Resource-Based Selection | システムリソース（CPU/GPU/メモリ）に基づく最適モデルの自動選択。 |
| **ループバックオーディオ** | Loopback Audio | システム全体の音声出力をキャプチャする仮想デバイス（macOS: BlackHole、Windows: WASAPI loopback）。 |

---

## Requirements

### STT-REQ-001: Real Audio Device Management

**Objective**: ソフトウェアエンジニアとして、実際の音声デバイスから音声データを取得したい。これにより、マイクまたはシステム音声のリアルタイム録音が可能になる。

**※重要**: ADR-001（録音責務の一元化）に基づき、音声録音はRust側AudioDeviceAdapterのみが実行します。Python側での録音は静的解析（check_forbidden_imports.py）により禁止されています。

#### Acceptance Criteria

1. **STT-REQ-001.1**: WHEN Tauriアプリが起動 THEN RealAudioDevice (Rust) SHALL システム上の利用可能な音声入力デバイスを列挙する
2. **STT-REQ-001.2**: WHEN 音声デバイスリストが取得される THEN RealAudioDevice (Rust) SHALL 各デバイスの名前、ID、サンプルレート、チャンネル数を含むメタデータを返す
3. **STT-REQ-001.3**: WHEN ユーザーが設定画面で音声デバイスを選択 THEN Tauriアプリ SHALL 選択されたデバイスIDを設定に保存する
4. **STT-REQ-001.4**: WHEN 録音開始コマンドが実行される THEN RealAudioDevice (Rust) SHALL 選択された音声デバイスを初期化し、サンプルレート16kHzでモノラル音声ストリームを開始する [※Rust側AudioDeviceAdapterで実装]
5. **STT-REQ-001.5**: WHEN 音声ストリームが開始される THEN RealAudioDevice (Rust) SHALL 20ms間隔で320サンプル（16kHz * 0.02秒）の音声データを生成する [※Rust側AudioDeviceAdapterで実装]
6. **STT-REQ-001.6**: WHEN 音声データが生成される THEN RealAudioDevice (Rust) SHALL 音声データをPythonサイドカープロセスに送信する [※stdin/stdout IPC経由]
7. **STT-REQ-001.7**: WHEN 録音停止コマンドが実行される THEN RealAudioDevice (Rust) SHALL 音声ストリームを停止し、デバイスリソースを解放する [※Rust側AudioDeviceAdapterで実装]
8. **STT-REQ-001.8**: IF 音声デバイス初期化が失敗 THEN RealAudioDevice (Rust) SHALL エラーメッセージ「音声デバイスの初期化に失敗しました: [デバイス名]」をユーザーに通知する [※Rust側AudioDeviceAdapterで実装]
9. **STT-REQ-001.9**: IF 音声デバイス初期化が失敗 THEN RealAudioDevice (Rust) SHALL 3秒間隔で最大3回まで初期化を再試行する [※Rust側AudioDeviceAdapterで実装]

---

### STT-REQ-002: faster-whisper Integration (Offline-First)

**Objective**: ソフトウェアエンジニアとして、faster-whisperモデルを統合し、**オフライン環境でも動作可能な**高精度音声文字起こしを実行したい。これにより、企業ネットワークやネットワーク切断時でも実用可能なSTT機能が提供される。

#### Acceptance Criteria

1. **STT-REQ-002.1**: WHEN Pythonサイドカープロセスが起動 THEN WhisperSTTEngine SHALL 以下の優先順位でモデルを検出する:
   1. ユーザー設定で指定されたモデルパス（`~/.config/meeting-minutes-automator/whisper_model_path`）
   2. HuggingFace Hubキャッシュ（`~/.cache/huggingface/hub/models--Systran--faster-whisper-*`）
   3. インストーラーバンドルモデル（`[app_resources]/models/faster-whisper/base`）

2. **STT-REQ-002.2**: WHEN システムリソースが検出される THEN WhisperSTTEngine SHALL 以下のリソースベースモデル選択ルールに従ってWhisperモデルサイズを決定する:
   - GPU利用可能 AND メモリ8GB以上: large-v3モデル
   - GPU利用可能 AND メモリ4GB以上: mediumモデル
   - CPU AND メモリ4GB以上: small モデル
   - CPU AND メモリ2GB以上: base モデル
   - それ以外: tiny モデル

3. **STT-REQ-002.3**: WHEN Whisperモデルサイズが決定される THEN WhisperSTTEngine SHALL faster-whisperモデルをHuggingFace Hubからダウンロードする（初回のみ、タイムアウト10秒）

4. **STT-REQ-002.4**: IF HuggingFace Hubからのダウンロードが失敗（タイムアウト、ネットワークエラー、プロキシ認証エラー） THEN WhisperSTTEngine SHALL バンドルbaseモデルにフォールバックし、ログに「オフラインモードで起動: バンドルbaseモデル使用」を記録する

5. **STT-REQ-002.5**: IF バンドルモデルも存在しない THEN WhisperSTTEngine SHALL 起動失敗エラー「faster-whisperモデルが見つかりません。インストールを確認してください」を返す

6. **STT-REQ-002.6**: WHEN ユーザーが設定で「オフラインモード強制」を有効化 THEN WhisperSTTEngine SHALL HuggingFace Hub接続を完全にスキップし、ローカルモデル（キャッシュまたはバンドル）のみを使用する

7. **STT-REQ-002.7**: WHEN 企業プロキシ環境で起動 THEN WhisperSTTEngine SHALL 環境変数`HTTPS_PROXY`および`HTTP_PROXY`を認識し、HuggingFace Hub接続に適用する

8. **STT-REQ-002.8**: WHEN モデルダウンロード中 THEN WhisperSTTEngine SHALL ダウンロード進捗をログに記録し、Tauriアプリに通知する

9. **STT-REQ-002.9**: WHEN モデルダウンロードが完了 THEN WhisperSTTEngine SHALL モデルを`~/.cache/huggingface/hub/`にキャッシュする

10. **STT-REQ-002.10**: WHEN モデルがロード完了 THEN WhisperSTTEngine SHALL "whisper_model_ready"メッセージをstdoutに出力する

11. **STT-REQ-002.11**: WHEN `process_audio`メッセージを受信 THEN WhisperSTTEngine SHALL 音声データをBase64デコードし、faster-whisperモデルで推論を実行する

12. **STT-REQ-002.12**: WHEN faster-whisper推論が完了 THEN WhisperSTTEngine SHALL 以下の形式でJSON応答を返す:
   ```json
   {
     "id": "request-id",
     "type": "response",
     "result": {
       "text": "文字起こし結果",
       "confidence": 0.95,
       "language": "ja",
       "is_final": true
     }
   }
   ```

13. **STT-REQ-002.13**: IF faster-whisperモデルのロードが失敗 THEN WhisperSTTEngine SHALL エラーログを記録し、tinyモデルへのフォールバックを試行する

14. **STT-REQ-002.14**: IF 音声データが不正（空、サンプルレート不一致等） THEN WhisperSTTEngine SHALL エラー応答`{"type": "error", "errorCode": "INVALID_AUDIO"}`を返す

---

### STT-REQ-003: webrtcvad Integration

**Objective**: ソフトウェアエンジニアとして、音声活動検出により発話区間を識別したい。これにより、無音時の不要なSTT処理を削減し、効率的な文字起こしを実現する。

#### Acceptance Criteria

1. **STT-REQ-003.1**: WHEN Pythonサイドカープロセスが起動 THEN VoiceActivityDetector SHALL webrtcvadライブラリを初期化し、aggressiveness=2（中程度）を設定する
2. **STT-REQ-003.2**: WHEN 音声データを受信 THEN VoiceActivityDetector SHALL 音声データを10ms単位のフレームに分割する
3. **STT-REQ-003.3**: WHEN 各フレームが処理される THEN VoiceActivityDetector SHALL webrtcvadでフレームごとに音声/無音を判定する
4. **STT-REQ-003.4**: WHEN 音声フレームが連続して0.3秒以上検出される THEN VoiceActivityDetector SHALL 発話開始イベントを記録する
5. **STT-REQ-003.5**: WHEN 無音フレームが連続して0.5秒以上検出される THEN VoiceActivityDetector SHALL 発話終了イベントを記録し、発話セグメントを確定する
6. **STT-REQ-003.6**: WHEN 発話セグメントが確定される THEN VoiceActivityDetector SHALL セグメント音声データをWhisperSTTEngineに渡し、確定テキスト生成を要求する
7. **STT-REQ-003.7**: WHEN 発話継続中（発話開始後、終了前） THEN VoiceActivityDetector SHALL 累積音声データをWhisperSTTEngineに渡し、部分テキスト生成を要求する（1秒間隔）
8. **STT-REQ-003.8**: WHEN 部分テキストが生成される THEN VoiceActivityDetector SHALL `{"is_final": false}`フラグ付きでTauriアプリに送信する
9. **STT-REQ-003.9**: WHEN 確定テキストが生成される THEN VoiceActivityDetector SHALL `{"is_final": true}`フラグ付きでTauriアプリに送信する

---

### STT-REQ-004: Cross-Platform Audio Device Support

**Objective**: ソフトウェアエンジニアとして、macOS、Windows、Linuxで統一的に音声デバイスにアクセスしたい。これにより、クロスプラットフォーム動作を保証する。

#### Acceptance Criteria

1. **STT-REQ-004.1**: WHEN 録音開始前 THEN RealAudioDevice SHALL OSのマイクアクセス許可を確認し、許可されていない場合は明示的な許可ダイアログを表示する
2. **STT-REQ-004.2**: IF ユーザーがマイクアクセスを拒否 THEN RealAudioDevice SHALL エラーメッセージ「マイクアクセスが拒否されました。システム設定から許可してください」を表示し、録音を中断する
3. **STT-REQ-004.3**: WHEN macOS環境で起動 THEN RealAudioDevice SHALL Core Audioフレームワーク経由で音声デバイスにアクセスする
4. **STT-REQ-004.4**: WHEN Windows環境で起動 THEN RealAudioDevice SHALL WASAPI経由で音声デバイスにアクセスする
5. **STT-REQ-004.5**: WHEN Linux環境で起動 THEN RealAudioDevice SHALL ALSA/PulseAudio経由で音声デバイスにアクセスする
6. **STT-REQ-004.6**: WHEN macOSでループバック音声デバイスを選択 THEN RealAudioDevice SHALL BlackHole等の仮想デバイスを認識し、システム音声をキャプチャする
7. **STT-REQ-004.7**: WHEN Windowsでループバック音声デバイスを選択 THEN RealAudioDevice SHALL WASAPI loopback modeでシステム音声をキャプチャする
8. **STT-REQ-004.8**: WHEN Linuxでループバック音声デバイスを選択 THEN RealAudioDevice SHALL PulseAudio monitorデバイスでシステム音声をキャプチャする
9. **STT-REQ-004.9**: WHEN 音声デバイスが切断される THEN RealAudioDevice SHALL デバイス切断イベントを検出し、エラーログを記録する
10. **STT-REQ-004.10**: WHEN デバイス切断が検出される THEN RealAudioDevice SHALL ユーザーに「音声デバイスが切断されました」通知を表示し、録音を停止する
11. **STT-REQ-004.11**: IF デバイス切断が検出される THEN RealAudioDevice SHALL 5秒後に自動再接続を試行する（最大3回）

---

### STT-REQ-005: Local Storage (Recording File Management)

**Objective**: ソフトウェアエンジニアとして、録音セッションをローカルストレージに永続化したい。これにより、後から録音内容を再生・分析できる。

#### Acceptance Criteria

1. **STT-REQ-005.1**: WHEN 録音セッションが開始される THEN LocalStorageService SHALL セッションIDを生成し、`[app_data_dir]/recordings/[session_id]/`ディレクトリを作成する
2. **STT-REQ-005.2**: WHEN 音声データが録音される THEN LocalStorageService SHALL 音声データを`audio.wav`ファイルとして保存する（16kHz, モノラル, 16bit PCM）
3. **STT-REQ-005.3**: WHEN 文字起こし結果が生成される THEN LocalStorageService SHALL 部分テキストと確定テキストを`transcription.jsonl`ファイルに追記する（JSON Lines形式）
4. **STT-REQ-005.4**: WHEN 録音セッションが終了される THEN LocalStorageService SHALL セッションメタデータを`session.json`ファイルに保存する:
   ```json
   {
     "session_id": "uuid",
     "start_time": "2025-10-02T10:00:00Z",
     "end_time": "2025-10-02T11:30:00Z",
     "duration_seconds": 5400,
     "audio_device": "MacBook Pro Microphone",
     "model_size": "small",
     "total_segments": 150,
     "total_characters": 12000
   }
   ```
5. **STT-REQ-005.5**: WHEN 録音セッションリストを取得 THEN LocalStorageService SHALL `recordings/`ディレクトリ内の全セッションメタデータを読み込み、日時降順でソートしたリストを返す
6. **STT-REQ-005.6**: WHEN ユーザーが過去のセッションを選択 THEN LocalStorageService SHALL セッションディレクトリから`session.json`, `transcription.jsonl`, `audio.wav`を読み込み、再生・表示機能を提供する
7. **STT-REQ-005.7**: WHEN ディスク容量が1GB未満 THEN LocalStorageService SHALL 警告ログを記録し、ユーザーに「ディスク容量が不足しています」通知を表示する
8. **STT-REQ-005.8**: WHEN ディスク容量が500MB未満 THEN LocalStorageService SHALL 新規録音開始を拒否し、エラーメッセージ「ディスク容量が不足しているため録音できません」を表示する

---

### STT-REQ-006: Resource-Based Model Selection and Dynamic Downgrade

**Objective**: ソフトウェアエンジニアとして、システムリソースに応じて最適なWhisperモデルを**起動時に自動選択**し、**実行中のリソース制約時に動的にダウングレード**したい。これにより、リソース制約下でも安定動作を保証する。

#### Acceptance Criteria (起動時モデル選択)

1. **STT-REQ-006.1**: WHEN Pythonサイドカープロセス起動時 THEN ResourceMonitor SHALL CPUコア数、メモリ容量、GPU利用可否、GPUメモリ容量を検出する

2. **STT-REQ-006.2**: WHEN システムリソースが検出される THEN ResourceMonitor SHALL 以下のモデル選択ルールを適用する:

| 条件 | 選択モデル | 理由 |
|------|-----------|------|
| GPU利用可能 AND システムメモリ≥8GB AND GPUメモリ≥10GB | large-v3 | 最高精度優先 |
| GPU利用可能 AND システムメモリ≥4GB AND GPUメモリ≥5GB | medium | 精度とリソースのバランス |
| CPU AND メモリ≥4GB | small | CPU推論の現実的な上限 |
| CPU AND メモリ≥2GB | base | 低リソース環境対応 |
| メモリ<2GB | tiny | 最低限動作保証 |

3. **STT-REQ-006.3**: WHEN モデル選択が完了 THEN ResourceMonitor SHALL 選択されたモデルサイズをログに記録する

4. **STT-REQ-006.4**: WHEN ユーザーが設定画面で手動モデル選択 THEN ResourceMonitor SHALL 自動選択をオーバーライドし、選択されたモデルを使用する

5. **STT-REQ-006.5**: WHEN 手動選択されたモデルがシステムリソースを超過 THEN ResourceMonitor SHALL 警告ログ「選択されたモデルはシステムリソースを超過する可能性があります」を記録する

#### Acceptance Criteria (実行中の動的ダウングレード)

6. **STT-REQ-006.6**: WHEN システムメモリ使用量が3GB超過 THEN ResourceMonitor SHALL 以下のダウングレードシーケンスを実行する:
   - ダウングレード順序: large → medium → small → base → tiny

7. **STT-REQ-006.7**: WHEN システムCPU使用率が85%を60秒以上持続 THEN ResourceMonitor SHALL 現在のモデルを1段階ダウングレードし、UIに「CPU負荷軽減のためモデルを{old}→{new}に変更しました」通知を表示する

8. **STT-REQ-006.8**: WHEN メモリ使用量が4GB到達 THEN ResourceMonitor SHALL 即座にbaseモデルへダウングレードし、UIに「メモリ不足のためbaseモデルに変更しました」通知を表示する

9. **STT-REQ-006.9**: WHEN モデルダウングレードが実行される THEN ResourceMonitor SHALL 以下の動作を保証する:
   - **音声セグメント境界での切り替え**: 現在処理中の音声セグメントは既存モデルで完了し、次のセグメントから新モデルを適用
   - 処理中断時間: 0秒（シームレス切り替え）
   - 進行中の音声セグメント処理は現在のモデルで完了まで継続
   - 次の音声セグメントから新しいモデルを使用
   - UI通知: トースト通知で「モデル変更: {old} → {new}」を表示
   - モデル切り替え履歴をローカルログに記録（トラブルシューティング用）

10. **STT-REQ-006.10**: WHEN システムリソースが回復（メモリ2GB未満 AND CPU 50%未満が5分継続） THEN ResourceMonitor SHALL 元のモデルサイズへの自動アップグレードを提案するUI通知を表示する

11. **STT-REQ-006.11**: IF モデルダウングレードがtinyに到達してもリソース不足が継続 THEN ResourceMonitor SHALL 録音を一時停止し、「システムリソース不足のため録音を一時停止しました」エラーを表示する

12. **STT-REQ-006.12**: WHEN ユーザーがモデルアップグレード提案を承認 THEN ResourceMonitor SHALL 1段階上位のモデルへアップグレードを試行し、メモリ使用量を監視する

---

### STT-REQ-007: IPC Protocol Extension (Backward Compatible)

**Objective**: ソフトウェアエンジニアとして、meeting-minutes-coreで確立したIPC通信プロトコルを拡張したい。これにより、実音声処理の追加情報を伝達しつつ、既存のWalking Skeleton実装との互換性を保つ。

**前提条件**: meeting-minutes-core CORE-REQ-004で定義されたIPC通信プロトコルv1.0およびCORE-REQ-006のWebSocketMessage Tagged Union形式との互換性を維持します。

#### Acceptance Criteria

1. **STT-REQ-007.1**: WHEN Fake実装からReal実装への移行 THEN IPC通信プロトコル SHALL 既存のメッセージ形式を維持する

2. **STT-REQ-007.2**: WHEN 新しいフィールドを追加 THEN IPC通信プロトコル SHALL 以下の拡張フィールドを含む:
   ```json
   {
     "id": "uuid",
     "type": "response",
     "version": "1.0",              // 新規（バージョニング）
     "result": {
       "text": "文字起こし結果",
       "is_final": true,
       "confidence": 0.95,          // 新規
       "language": "ja",             // 新規
       "processing_time_ms": 450,    // 新規
       "model_size": "small"         // 新規
     }
   }
   ```

3. **STT-REQ-007.3**: WHEN meeting-minutes-core（Fake実装）がこのメッセージを受信 THEN Fake実装 SHALL 未知のフィールドを無視し、`text`フィールドのみを使用する（後方互換性）

4. **STT-REQ-007.4**: WHEN 全てのIPCメッセージを送信 THEN IPC通信プロトコル SHALL `"version": "1.0"`フィールドを必須とし、将来的なプロトコル変更時の互換性確認に使用する

5. **STT-REQ-007.5**: WHEN WhisperSTTEngineがエラーを返す THEN IPC通信プロトコル SHALL 以下のエラー応答形式を使用する:
   ```json
   {
     "id": "uuid",
     "type": "error",
     "version": "1.0",
     "errorCode": "STT_INFERENCE_ERROR",
     "errorMessage": "Whisper model inference failed: [details]",
     "recoverable": false
   }
   ```

6. **STT-REQ-007.6**: WHEN バージョン不一致が検出される THEN IPC通信プロトコル SHALL 以下の処理を実行する:
   - メジャーバージョン不一致（例: 1.x → 2.x）: エラー応答を返し、通信を拒否
   - マイナーバージョン不一致（例: 1.0 → 1.1）: 警告ログを記録し、後方互換モードで処理継続（ADR-003に基づく）
   - パッチバージョン不一致（例: 1.0.1 → 1.0.2）: 情報ログのみ記録し、通常処理継続

---

### STT-REQ-008: WebSocket Message Extension

**Objective**: ソフトウェアエンジニアとして、Chrome拡張に送信するWebSocketメッセージを拡張したい。これにより、リッチな文字起こし情報（confidence、言語等）をブラウザ側で利用可能にする。

#### Acceptance Criteria

1. **STT-REQ-008.1**: WHEN 文字起こし結果をWebSocket配信 THEN WebSocketServer SHALL 以下の拡張メッセージ形式を使用する:
   ```json
   {
     "messageId": 123,
     "sessionId": "session-uuid",
     "timestamp": 1696234567890,
     "type": "transcription",
     "isPartial": false,
     "text": "文字起こし結果",
     "confidence": 0.95,           // 新規
     "language": "ja",             // 新規
     "speakerSegment": 0,          // 新規（MVP1ではダミー値）
     "processingTimeMs": 450       // 新規
   }
   ```

2. **STT-REQ-008.2**: WHEN meeting-minutes-core（Fake実装）のChrome拡張がこのメッセージを受信 THEN Chrome拡張 SHALL 未知のフィールドを無視し、`text`フィールドのみを使用する（後方互換性）

3. **STT-REQ-008.3**: WHEN エラーが発生 THEN WebSocketServer SHALL 以下のエラーメッセージ形式を使用する:
   ```json
   {
     "messageId": 124,
     "sessionId": "session-uuid",
     "timestamp": 1696234567890,
     "type": "error",
     "errorCode": "AUDIO_DEVICE_ERROR",
     "errorMessage": "Audio device disconnected",
     "recoverable": true
   }
   ```

---

## Non-Functional Requirements

### STT-NFR-001: Performance

1. **STT-NFR-001.1**: WHEN faster-whisper推論を実行 THEN WhisperSTTEngine SHALL 以下の処理時間目標を達成する:
   - tiny/base: 0.2秒以内（1秒の音声データに対して）
   - small: 0.5秒以内
   - medium: 1秒以内
   - large: 2秒以内（GPU使用時）

2. **STT-NFR-001.2**: WHEN 音声ストリーミング中 THEN RealAudioDevice SHALL 20ms間隔で音声データを生成し、遅延を50ms以内に抑える

3. **STT-NFR-001.3**: WHEN VAD処理を実行 THEN VoiceActivityDetector SHALL 10msフレームごとの判定を1ms以内に完了する

4. **STT-NFR-001.4**: WHEN 録音セッション中 THEN システム全体のメモリ使用量 SHALL 選択されたモデルサイズに応じた上限を超えない:
   - tiny/base: 500MB以下
   - small: 1GB以下
   - medium: 2GB以下
   - large: 4GB以下

5. **STT-NFR-001.5**: WHEN モデルダウングレード実行中 THEN ResourceMonitor SHALL モデル切り替えを3秒以内に完了し、音声処理の中断時間を最小化する

6. **STT-NFR-001.6**: WHEN リソース監視を実行 THEN ResourceMonitor SHALL CPU/メモリ使用量を30秒間隔で監視し、オーバーヘッドを2%以内に抑える

### STT-NFR-002: Reliability

1. **STT-NFR-002.1**: WHEN faster-whisperモデルロードが失敗 THEN WhisperSTTEngine SHALL 自動的にtinyモデルへフォールバックし、ユーザーに通知する

2. **STT-NFR-002.2**: WHEN 音声デバイスが切断される THEN RealAudioDevice SHALL デバイス切断を検出し、5秒以内に録音を安全に停止する

3. **STT-NFR-002.3**: WHEN Pythonプロセスが異常終了 THEN Tauriアプリ SHALL プロセス異常終了を検出し、自動再起動を試行する（最大3回）

4. **STT-NFR-002.4**: WHEN ディスク容量が不足 THEN LocalStorageService SHALL 録音を安全に停止し、既存データの破損を防ぐ

### STT-NFR-003: Compatibility

1. **STT-NFR-003.1**: RealAudioDevice SHALL macOS 11以降、Windows 10以降、Ubuntu 20.04以降で動作する

2. **STT-NFR-003.2**: WhisperSTTEngine SHALL Python 3.9以降で動作する

3. **STT-NFR-003.3**: faster-whisper SHALL CTranslate2 3.0以降に依存する

4. **STT-NFR-003.4**: webrtcvad SHALL webrtcvad 2.0以降に依存する

### STT-NFR-004: Security

1. **STT-NFR-004.1**: WHEN faster-whisperモデルをダウンロード THEN WhisperSTTEngine SHALL HuggingFace HubのHTTPS接続を使用し、TLS 1.2以降で通信する

2. **STT-NFR-004.2**: WHEN faster-whisperモデルをインストーラーにバンドル THEN インストーラー SHALL モデルファイルのSHA256ハッシュを検証し、改ざんを検出する

3. **STT-NFR-004.3**: WHEN ローカルストレージに保存 THEN LocalStorageService SHALL アプリケーション専用ディレクトリ（ユーザーのホームディレクトリ配下）にのみ書き込む

4. **STT-NFR-004.4**: WHEN 音声デバイスにアクセス THEN RealAudioDevice SHALL OSのアクセス許可ダイアログでユーザーの明示的な許可を要求する（macOS/Windows）

### STT-NFR-005: Logging

1. **STT-NFR-005.1**: WHEN faster-whisperモデルロード中 THEN WhisperSTTEngine SHALL ダウンロード進捗を5秒間隔でログに記録する

2. **STT-NFR-005.2**: WHEN 音声処理エラーが発生 THEN 各コンポーネント SHALL エラーメッセージ、スタックトレース、コンテキストをERRORレベルでログに記録する

3. **STT-NFR-005.3**: WHEN VAD処理中 THEN VoiceActivityDetector SHALL 発話開始/終了イベントをINFOレベルでログに記録する

4. **STT-NFR-005.4**: WHEN リソース監視 THEN ResourceMonitor SHALL メモリ使用量、CPU使用率を30秒間隔でDEBUGレベルでログに記録する

---

## Out of Scope (明確な非スコープ)

以下の機能は、本MVP1実装には**含まれません**。後続のMVPで実装されます:

### MVP2 (meeting-minutes-docs-sync) で実装予定
- OAuth 2.0認証フロー
- Google Docs API統合（batchUpdate、Named Range管理）
- リアルタイムDocs同期
- オフライン時のキューイングと再同期

### MVP3 (meeting-minutes-llm) で実装予定
- LLM API統合（OpenAI/ローカルLLM）
- セグメント要約/ローリングサマリー生成
- Tauri UI洗練（設定画面、履歴表示、リアルタイム表示）
- リソース管理3段階閾値の完全実装
- エラーハンドリングの高度化

### 将来検討事項（スコープ外）
- 話者分離（speaker diarization）
- リアルタイム翻訳
- カスタムWhisperモデルのファインチューニング
- モバイルアプリ対応

---

## Success Criteria

本MVP1実装は、以下の条件を全て満たした場合に成功とみなされます:

1. ✅ **実音声処理**: マイクまたはループバックデバイスから実際の音声をキャプチャし、faster-whisperで文字起こしが動作する（STT-REQ-001, STT-REQ-002）
2. ✅ **VAD動作**: webrtcvadによる発話区間検出が機能し、部分テキストと確定テキストが正しく生成される（STT-REQ-003）
3. ✅ **リソースベース選択**: システムリソースに応じて最適なWhisperモデルが自動選択され、実行中の動的ダウングレードが機能する（STT-REQ-006）
4. ✅ **ローカルストレージ**: 録音セッションがローカルに保存され、後から再生・閲覧できる（STT-REQ-005）
5. ✅ **クロスプラットフォーム**: macOS、Windows、Linuxの3環境で動作確認が取れる（STT-REQ-004, STT-NFR-003）
6. ✅ **後方互換性**: meeting-minutes-core（Walking Skeleton）からの移行がスムーズに行える（STT-REQ-007, STT-REQ-008）
7. ✅ **meeting-minutes-core互換性**: CORE-REQ-004 (IPC) およびCORE-REQ-006 (WebSocket)との後方互換性が保証されている

---

## Dependencies

### Upstream Dependencies (Blocking)

本specの実装開始前に、以下の成果物が完了している必要があります:

- **meeting-minutes-core** (phase: design-validated以降):
  - **CORE-REQ-004**: IPC通信プロトコル v1.0 (stdin/stdout JSON) - `design.md` L1243-1279参照
  - **CORE-REQ-006**: WebSocketサーバー (ポート9001-9100) - `design.md` L1156-1241参照
  - **CORE-REQ-007**: Chrome拡張スケルトン (WebSocket接続機能) - `design.md` L1349-1550参照
- **前提**: meeting-minutes-core/design.md L1243-1279で定義されたWebSocketMessage Tagged Union形式を基準とする

### External Dependencies
- **faster-whisper**: ≥0.10.0
- **webrtcvad**: ≥2.0.0
- **numpy**: ≥1.24.0（音声データ処理用）
- **Python**: 3.9以降
- **CTranslate2**: 3.0以降（faster-whisperの依存）

### Internal Dependencies
- **meeting-minutes-core**: Walking Skeleton実装（IPC通信、WebSocketサーバー、Chrome拡張スケルトン）
- **Umbrella Spec**: `.kiro/specs/meeting-minutes-automator` - 全体アーキテクチャリファレンス
- **Steering Documents**:
  - `tech.md`: faster-whisper統合、webrtcvad統合の技術詳細
  - `structure.md`: Pythonモジュール構造（`audio/`, `transcription/`）
  - `principles.md`: プロセス境界の明確化原則、リソース管理原則

---

## MVP0 からの引き継ぎ要件

このセクションは `meeting-minutes-core` (Walking Skeleton) で未実装または部分実装だった機能要件を定義します。詳細は `docs/mvp0-known-issues.md` の「MVP1 Traceability」セクション参照。

### STT-REQ-IPC-004: IPC Latency Monitoring

**背景**: MVP0では基本的なIPC通信のみ実装。レイテンシ計測機能が未実装（`docs/mvp0-known-issues.md` Ask 9-1）

**要件**: The system shall measure and log IPC communication latency between Rust and Python processes.

**受け入れ条件**:
- **AC-IPC-004.1**: When an IPC message is sent, the system shall record a timestamp.
- **AC-IPC-004.2**: When an IPC response is received, the system shall calculate the round-trip latency.
- **AC-IPC-004.3**: The system shall log latency as `ipc_latency_ms` metric via structured logging (`logger.rs`).
- **AC-IPC-004.4**: The mean IPC latency shall remain below 50ms under normal operation.
- **AC-IPC-004.5**: The `scripts/performance_report.py` shall aggregate IPC latency metrics (min/max/mean/median/stdev).

**優先度**: High（Real STT処理では長時間処理があるため、IPC性能監視が重要）

**トレーサビリティ**:
- **MVP0**: `meeting-minutes-core/tasks.md` Task 4.2（未実装）
- **Known Issues**: `docs/mvp0-known-issues.md` Ask 9-1

---

### STT-REQ-IPC-005: IPC Health Check and Retry Logic

**背景**: MVP0では基本的なエラー処理のみ。ヘルスチェック・リトライ機構が未実装（`docs/mvp0-known-issues.md` Ask 9-1）

**要件**: The system shall implement health check and retry logic for IPC communication to handle transient failures.

**受け入れ条件**:
- **AC-IPC-005.1**: The system shall track consecutive IPC failures.
- **AC-IPC-005.2**: When 3 consecutive IPC failures occur, the system shall initiate a retry sequence with exponential backoff.
- **AC-IPC-005.3**: When 5 consecutive failures occur, the system shall notify the user via UI error message.
- **AC-IPC-005.4**: The retry sequence shall use exponential backoff: 1s, 2s, 4s, 8s, 16s (max).
- **AC-IPC-005.5**: The system shall reset the failure counter after a successful IPC response.

**優先度**: High（Real STT処理時の安定性確保に必須）

**トレーサビリティ**:
- **MVP0**: `meeting-minutes-core/tasks.md` Task 4.2, 4.3（未実装）
- **Known Issues**: `docs/mvp0-known-issues.md` Ask 9-1

---

### STT-REQ-LOG-001: Structured Logging Migration

**背景**: MVP0では `logger.rs` 実装済みだが未使用（`println!`/`eprintln!` のまま）（`docs/mvp0-known-issues.md` Ask 9-2）

**要件**: The system shall replace all `println!`/`eprintln!` calls with structured JSON logging using `logger.rs` macros.

**受け入れ条件**:
- **AC-LOG-001.1**: All Rust components shall use `log_info!`, `log_error!`, `log_warn!`, `log_debug!` macros.
- **AC-LOG-001.2**: Key events shall be logged: recording start/stop, IPC message send/receive, WebSocket broadcast, STT processing start/complete.
- **AC-LOG-001.3**: Error logs shall include context information (error type, component, event, details).
- **AC-LOG-001.4**: All logs shall output in JSON format with timestamp, level, component, event, message fields.

**優先度**: Medium（デバッグ効率・運用性向上）

**トレーサビリティ**:
- **MVP0**: `src-tauri/src/logger.rs` 実装済みだが未使用
- **Known Issues**: `docs/mvp0-known-issues.md` Ask 9-2

---

### STT-REQ-SEC-001: IPC JSON Message Validation

**背景**: MVP0では受信JSONを無条件でデシリアライズ。サイズ・フィールド検証なし（`docs/mvp0-known-issues.md` Ask 9-3）

**要件**: The system shall validate all incoming IPC JSON messages for size limits and required fields before processing.

**受け入れ条件**:
- **AC-SEC-001.1**: If an IPC message exceeds 1MB size, then the system shall reject it with an error response.
- **AC-SEC-001.2**: The system shall validate that all IPC messages contain required fields: `type` (string), `id` (string).
- **AC-SEC-001.3**: The system shall validate message-specific required fields (e.g., `process_audio` requires `audio_data` array).
- **AC-SEC-001.4**: If validation fails, then the system shall log a warning and send an error response to Python.
- **AC-SEC-001.5**: The system shall not crash on malformed JSON (invalid UTF-8, syntax errors).

**優先度**: High（Real STT実装前にセキュリティ要件を満たす必要がある）

**トレーサビリティ**:
- **MVP0**: `src-tauri/src/python_sidecar.rs:receive_message()` 基本的なJSONパースのみ
- **Known Issues**: `docs/mvp0-known-issues.md` Ask 9-3

---

### STT-REQ-E2E-001: Chrome Extension Automated E2E Testing

**背景**: MVP0ではChrome拡張部分は手動E2Eのみ（`docs/mvp0-known-issues.md` Ask 8-1）

**要件**: The system shall provide automated E2E tests that verify the full flow including Chrome extension behavior.

**受け入れ条件**:
- **AC-E2E-001.1**: E2E tests shall use Puppeteer or Playwright to automate Chrome browser.
- **AC-E2E-001.2**: E2E tests shall load the Chrome extension programmatically.
- **AC-E2E-001.3**: E2E tests shall verify that transcription messages appear in Chrome Console output.
- **AC-E2E-001.4**: E2E tests shall run in headless mode for CI/CD integration.
- **AC-E2E-001.5**: E2E tests shall verify WebSocket connection establishment from Chrome extension.

**優先度**: Medium（CI/CD自動化で有用だが、手動E2Eでも検証可能）

**トレーサビリティ**:
- **MVP0**: 手動E2Eテスト実施済み（`docs/platform-verification.md`）
- **Known Issues**: `docs/mvp0-known-issues.md` Ask 8-1
- **CI/CD Spec**: `meeting-minutes-ci` 要件 CI-REQ-E2E-001 と連携

---

## Requirement Traceability Matrix

本サブスペックとアンブレラ仕様（meeting-minutes-automator）の要件対応表。

| STT ID | 要件概要 | アンブレラID | 備考 |
|--------|---------|-------------|------|
| STT-REQ-001 | Real Audio Device Management | REQ-001.1 | マイク/ループバック録音 |
| STT-REQ-002 | faster-whisper Integration (Offline-First) | ARC-002.a, ARC-002.1 | オフライン対応、モデルバンドル含む |
| STT-REQ-003 | webrtcvad Integration | ARC-002.c | 音声活動検出 |
| STT-REQ-004 | Cross-Platform Audio Device Support | REQ-004, CON-001.c | OS別音声デバイスアクセス |
| STT-REQ-005 | Local Storage | REQ-001.1.e, REQ-001.1.f | 録音ファイル保存・ローテーション |
| STT-REQ-006 | Resource-Based Model Selection and Dynamic Downgrade | ARC-002.2, NFR-002.1 | 起動時選択+動的ダウングレード |
| STT-REQ-007 | IPC Protocol Extension | REQ-005 | Pythonサイドカー通信拡張 |
| STT-REQ-008 | WebSocket Message Extension | REQ-003.1 | Chrome拡張連携メッセージ拡張 |
| STT-NFR-001 | Performance | NFR-001 | リアルタイム性能要件 |
| STT-NFR-002 | Reliability | NFR-004 | 可用性・自動復旧 |
| STT-NFR-003 | Compatibility | REQ-004 | クロスプラットフォーム動作 |
| STT-NFR-004 | Security | NFR-003 | ローカル処理優先、改ざん検証 |
| STT-NFR-005 | Logging | - | MVP1固有ログ要件 |
| **STT-REQ-IPC-004** | **IPC Latency Monitoring** | **CORE Task 4.2** | **MVP0引き継ぎ** |
| **STT-REQ-IPC-005** | **IPC Health Check and Retry** | **CORE Task 4.2, 4.3** | **MVP0引き継ぎ** |
| **STT-REQ-LOG-001** | **Structured Logging Migration** | **CORE logger.rs** | **MVP0引き継ぎ** |
| **STT-REQ-SEC-001** | **IPC JSON Validation** | **NFR-003** | **MVP0引き継ぎ** |
| **STT-REQ-E2E-001** | **Chrome Extension Automated E2E** | **CORE Task 8.2** | **MVP0引き継ぎ** |

**上流依存**:
- **meeting-minutes-core**: CORE-REQ-004 (IPC通信プロトコルv1.0), CORE-REQ-006 (WebSocketサーバー), CORE-REQ-007 (Chrome拡張スケルトン)
- **MVP0 Known Issues**: `docs/mvp0-known-issues.md` Ask 8-1, 9-1, 9-2, 9-3（引き継ぎ要件5件）

**下流影響**:
- **meeting-minutes-docs-sync**: STT-REQ-008のWebSocketメッセージ形式を利用
- **meeting-minutes-llm**: STT-REQ-005のローカルストレージ（transcription.jsonl）を要約入力として利用

---

### STT-REQ-EXT-001: Chrome拡張WebSocket管理方式の決定

**ID**: STT-REQ-EXT-001

**Objective**: ソフトウェアエンジニアとして、MVP0で未解決の設計判断（Service Worker vs Content Script WebSocket管理）を実装検証して確定したい。これにより、Real STT実装前に最適なアーキテクチャを確立できる。

**Background**:
MVP0（Walking Skeleton）では、設計書（`.kiro/specs/meeting-minutes-core/design.md:1387-1446`）がService WorkerによるWebSocket管理を要求していたが、実装時にMV3の30秒アイドル制約を考慮し、Content ScriptでWebSocket管理を実装した（`chrome-extension/service-worker.js:6-7`参照）。

**設計書 vs 実装の不整合**:
- **設計書・要件**: Service WorkerがWebSocket接続・再接続・状態管理を担当
- **MVP0実装**: Content ScriptがWebSocket管理、Service Workerは軽量メッセージ中継のみ

この設計判断は**実装時の経験的判断**であり、技術的検証データに基づいていない。MVP1開始前に両方式を実装検証し、データに基づき設計を確定する必要がある。

**検証項目**:

| 項目 | Service Worker方式（非採用） | Content Script方式（採用済み - ADR-004） |
|------|----------------------------|----------------------------------------|
| **MV3対応** | chrome.alarms/WebSocket ping (20秒間隔) でkeepalive実装 | タブ表示中は永続（30秒制約なし） ✅ |
| **タブライフサイクル** | 独立（タブ閉じても接続維持） | ページリフレッシュ・ナビゲーションで切断 |
| **Google Meet SPA** | 影響なし | URL変更監視 + 重複注入防止で対応 ✅ |
| **バックグラウンドタブ** | 影響なし | visibilitychange APIで接続管理 ✅ |
| **複数タブ** | 単一接続を共有 | chrome.storage.localで状態同期 ✅ |
| **拡張UI（Popup/Options）** | Service Workerから直接状態クエリ可能 | chrome.storage.local経由で状態共有 ✅ |
| **リソース消費** | keepaliveによるCPU/メモリ負荷 | タブ表示時のみ負荷（実測済み: 5-10MB/タブ） |
| **レイテンシ影響** | AC-NFR-PERF.4（<50ms）への影響 | storage経由でも50ms以内達成可能 ✅ |

**解決済みの課題** (ADR-004参照):
- ✅ Content Script永続性: MutationObserverでSPA対応実装済み
- ✅ 状態管理: chrome.storage.localによる状態ブリッジ実装済み
- ✅ Popup UI連携: storage.onChangedリスナーによるリアルタイム同期
- 再接続チャーン: Content Script方式でのページ遷移時の文字起こしデータ損失率

#### Acceptance Criteria

1. **STT-REQ-EXT-001.1**: WHEN 両方式のSpike実装が完了 THEN 以下の実測データを記録すること:
   - Service Worker方式: WebSocket維持成功率、CPU/メモリ使用率、keepalive実装パターン
   - Content Script方式: Google Meetでのページ遷移回数、再接続発生回数、文字起こしデータ損失率

2. **STT-REQ-EXT-001.2**: WHEN 実測データ分析が完了 THEN 以下を含む技術判断レポートを作成すること:
   - AC-NFR-PERF.4（IPC latency < 50ms）への影響評価
   - 複数タブ・拡張UI対応の実現可能性
   - リソース消費の比較（CPU/メモリ）
   - 推奨方式の選定根拠

3. **STT-REQ-EXT-001.3**: WHEN 採用方式が決定 THEN 以下を更新すること:
   - `.kiro/specs/meeting-minutes-core/design.md`: Chrome Extension Layer設計の修正
   - `.kiro/specs/meeting-minutes-core/requirements.md`: AC-007の修正（Service Worker → Content Scriptまたは逆）
   - 不採用方式のコードを削除

4. **STT-REQ-EXT-001.4**: WHEN コード修正が完了 THEN E2Eテスト（`src-tauri/tests/e2e_test.rs`）を再実行し、全パス確認すること

**Priority**: HIGH（Real STT実装前の設計確定が必須）

**Blocking**: 本要件はMVP1設計フェーズをブロックする（`spec.json` BLOCK-001参照）

**Reference**:
- `docs/mvp0-known-issues.md#ask-10`
- `chrome-extension/service-worker.js:6-7` (現在の判断コメント)
- `.kiro/specs/meeting-minutes-core/design.md:1387-1446` (元の設計書)

**Traceability**:
- **Parent**: CORE-REQ-007 (Chrome拡張スケルトン)
- **Dependency**: AC-NFR-PERF.4 (IPC latency < 50ms)

---

---

## Requirement Traceability Matrix

| 要件ID | 要件名 | タスクID | ステータス | 実装ファイル | テストファイル |
|--------|--------|----------|-----------|-------------|--------------|
| STT-REQ-007.1 | IPC後方互換性維持 | Task 7.1, 7.1.5 | ✅ 完了 | src-tauri/src/ipc_protocol.rs, src-tauri/src/commands.rs, src-tauri/src/python_sidecar.rs | tests/ipc_migration_test.rs |
| STT-REQ-007.2 | IPC拡張フィールド追加 | Task 7.1, 7.1.5 | ✅ 完了 | src-tauri/src/ipc_protocol.rs (TranscriptionResult構造体) | tests/ipc_migration_test.rs, src-tauri/src/ipc_protocol.rs (L426-439) |
| STT-REQ-007.4 | IPCバージョンフィールド必須化 | Task 7.1, 7.1.5 | ✅ 完了 | src-tauri/src/ipc_protocol.rs (PROTOCOL_VERSION), src-tauri/src/commands.rs | tests/ipc_migration_test.rs, src-tauri/src/ipc_protocol.rs (L317-331) |
| STT-REQ-007.5 | IPCエラーフォーマット統一 | Task 7.1, 7.1.5 | ✅ 完了 | src-tauri/src/ipc_protocol.rs (IpcMessage::Error), src-tauri/src/commands.rs | tests/ipc_migration_test.rs, src-tauri/src/ipc_protocol.rs (L192-216) |
| ADR-003 | versionデフォルト値設定 | Task 7.1, 7.1.5 | ✅ 完了 | src-tauri/src/ipc_protocol.rs (default_version関数) | src-tauri/src/ipc_protocol.rs (L317-331) |

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成（MVP1 Real STT要件定義） |
| 2025-10-02 | 1.1 | Claude Code | 要件ID採番、Traceability Matrix追加、オフライン対応詳細化、動的ダウングレード統合、依存関係明示化 |
| 2025-10-06 | 1.2 | Claude Code | **MVP0引き継ぎ要件追加**: STT-REQ-IPC-004, IPC-005, LOG-001, SEC-001, E2E-001（`docs/known-issues.md` Traceability連携） |
| 2025-10-13 | 1.3 | Claude Code | **Task 7.1/7.1.5完了**: Requirement Traceability Matrix追加（STT-REQ-007シリーズ実装状況） |
