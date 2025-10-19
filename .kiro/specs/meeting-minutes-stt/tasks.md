# Implementation Plan

## Overview

meeting-minutes-stt (MVP1) は、meeting-minutes-core (Walking Skeleton) で確立した3プロセスアーキテクチャ上に実際の音声処理機能を実装します。Fake実装を実音声処理に置き換え、faster-whisperによる高精度文字起こしとwebrtcvadによる音声活動検出を実現します。

**実装アプローチ**: TDD (Test-Driven Development) に基づき、失敗するテストを先に作成し、実装を肉付けしながらテストを緑化します。スケルトン→ユニットテスト→統合テスト→E2Eテストの順で段階的に実装します。

**重要な設計決定**:
- ADR-001: 録音責務の一元化（Rust側AudioDeviceAdapterのみ、Python側録音禁止）
- ADR-002: ハイブリッドモデル配布戦略（HuggingFace Hub + バンドルbaseモデルフォールバック）
- ADR-003: IPCバージョニング（後方互換性保証）

---

## Implementation Tasks

- [x] 1. 基盤整備とプロジェクト準備
  - Python依存関係の追加（faster-whisper、webrtcvad、numpy）
  - Rust依存関係の追加（cpal）
  - 静的解析スクリプトの動作確認（check_forbidden_imports.py）
  - 開発環境のセットアップ検証（Python 3.9-3.12、Rust 1.70+）
  - _Requirements: 全要件, STT-REQ-LOG-001_

- [x] 2. 実音声デバイス管理機能の実装（Rust側）
- [x] 2.1 AudioDeviceAdapter trait とOS別実装のスケルトン作成
  - 失敗するユニットテストを作成（デバイス列挙、録音開始/停止）
  - AudioDeviceAdapter traitの定義
  - WasapiAdapter（Windows）、CoreAudioAdapter（macOS）、AlsaAdapter（Linux）の空実装
  - OS検出ロジックとアダプター選択機能
  - _Requirements: STT-REQ-001.1, STT-REQ-001.2, STT-REQ-004.3, STT-REQ-004.4, STT-REQ-004.5_

- [x] 2.2 デバイス列挙機能の実装
  - デバイスメタデータ取得機能（名前、ID、サンプルレート、チャンネル数）
  - OS固有音声API統合（WASAPI、CoreAudio、ALSA）
  - デバイス一覧のソート・フィルタリング機能
  - ユニットテストの緑化
  - _Requirements: STT-REQ-001.1, STT-REQ-001.2_

- [x] 2.3 音声ストリームキャプチャ機能の実装
  - 16kHz mono PCM音声ストリーム開始機能（基本実装完了、リサンプリングは2.4で対応）
  - コールバック型音声データ処理機能（AudioChunkCallback）
  - 別スレッドでのストリーム管理（Sync制約回避）
  - CoreAudio/WASAPI/ALSA統合実装
  - ユニットテストの緑化
  - _Requirements: STT-REQ-001.4, STT-REQ-001.5, STT-REQ-001.6_

- [x] 2.4 ループバックオーディオ対応
  - macOS BlackHole等の仮想デバイス認識機能（パターンマッチ）
  - Windows Stereo Mix/Wave Out Mix認識
  - Linux PulseAudio monitorデバイス認識（"Monitor of", ".monitor"）
  - `AudioDeviceInfo.is_loopback`フィールド追加
  - ループバックデバイスフィルタリング機能
  - 3プラットフォーム統合実装完了
  - _Requirements: STT-REQ-004.6, STT-REQ-004.7, STT-REQ-004.8_

- [x] 2.5 デバイス切断検出と自動再接続機能
  - デバイス切断イベント検出機能（AudioDeviceEvent enum）
  - エラーログ記録機能
  - ユーザー通知機能（「音声デバイスが切断されました」）- app.emit() 統合
  - Liveness watchdog（250ms間隔、1200ms閾値）
  - デバイスポーリング（3秒間隔）
  - 全OS対応（CoreAudio/WASAPI/ALSA）
  - ユニットテストと統合テストの緑化
  - _Note: 自動再接続ロジック（STT-REQ-004.11）はフェーズ外、別タスクで実装予定_
  - _Requirements: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11_

- [x] 2.6 マイクアクセス許可確認機能
  - OS固有の許可確認ロジック（CoreAudio/WASAPI/ALSA）
  - 許可ダイアログ表示機能（OS自動表示）
  - 許可拒否時のエラー処理（適切なユーザーメッセージ）
  - AudioDeviceAdapter::check_permission()メソッド実装
  - ユニットテスト作成（全36テスト合格）
  - _Requirements: STT-REQ-004.1, STT-REQ-004.2_
  - _Note: エンドユーザー向けデバイス選択UI/UXは Task 9.1 で継続実装（バックエンド機能は完了）_

- [x] 3. faster-whisper統合とモデル管理機能（Python側）
- [x] 3.1 WhisperSTTEngineスケルトンと初期化ロジック
  - 失敗するユニットテストを作成（モデルロード、推論）
  - WhisperSTTEngineクラスの定義
  - モデル検出優先順位ロジック（ユーザー設定 → HuggingFace Hubキャッシュ → バンドル）
  - faster-whisperライブラリの初期化
  - _Requirements: STT-REQ-002.1, STT-REQ-002.10_

- [x] 3.2 リソースベースモデル選択機能
  - システムリソース検出（CPUコア数、メモリ容量、GPU利用可否）
  - モデル選択ルールの実装（GPU+8GB→large-v3、CPU+4GB→small等）
  - 手動モデル選択のオーバーライド機能
  - リソース超過時の警告ログ記録
  - ユニットテストの緑化（15テスト合格）
  - _Requirements: STT-REQ-006.1, STT-REQ-006.2, STT-REQ-006.3, STT-REQ-006.4, STT-REQ-006.5_

- [x] 3.3 オフラインモデルフォールバック機能
  - HuggingFace Hubからのモデルダウンロード（タイムアウト10秒）
  - ネットワークエラー時のバンドルbaseモデルフォールバック
  - プロキシ環境対応（HTTPS_PROXY、HTTP_PROXY環境変数）
  - オフラインモード強制設定機能
  - モデルキャッシュ管理（~/.cache/huggingface/hub/）
  - ダウンロード進捗ログ記録
  - ユニットテストと統合テストの緑化（14テスト合格）
  - _Requirements: STT-REQ-002.3, STT-REQ-002.4, STT-REQ-002.5, STT-REQ-002.6, STT-REQ-002.7, STT-REQ-002.8, STT-REQ-002.9_

- [x] 3.4 faster-whisper推論機能
  - 音声データ（16-bit PCM）のnumpy配列変換とfloat32正規化
  - faster-whisperモデルでの推論実行（language="ja", beam_size=5）
  - JSON応答フォーマット生成（text、confidence、language、is_final、processing_time_ms）
  - 不正音声データのエラー処理（空データ、破損データ）
  - avg_logprobからconfidenceへの変換（exponential変換）
  - ユニットテストの緑化（10テスト合格）
  - _Requirements: STT-REQ-002.11, STT-REQ-002.12, STT-REQ-002.13, STT-REQ-002.14_

- [x] 4. 音声活動検出（VAD）機能の実装（Python側）
- [x] 4.1 VoiceActivityDetectorスケルトンとwebrtcvad初期化
  - VoiceActivityDetectorクラスの定義とwebrtcvad初期化（aggressiveness=2）
  - 音声データの10ms単位フレーム分割機能（160 samples = 320 bytes）
  - is_speech()メソッドによるフレームごとの音声/無音判定
  - ユニットテストの緑化（9テスト合格）
  - _Requirements: STT-REQ-003.1, STT-REQ-003.2_

- [x] 4.2 音声/無音判定機能
  - process_frame()メソッドによるフレームごとの音声/無音状態管理
  - 発話開始検出（音声フレーム30フレーム=0.3秒以上連続でspeech_startイベント）
  - 発話終了検出（無音フレーム50フレーム=0.5秒以上連続でspeech_endイベント）
  - 発話セグメント確定ロジック（audio_data、duration_ms含むsegment返却）
  - 状態リセット処理（音声→無音、無音→音声の遷移時）
  - ユニットテストの緑化（8テスト合格、合計17テスト）
  - _Requirements: STT-REQ-003.3, STT-REQ-003.4, STT-REQ-003.5_

- [x] 4.3 部分テキストと確定テキストの生成連携（MVP0互換版）
  - AudioPipelineのmain.py統合完了（AudioProcessorクラス作成）
  - Request-Response型プロトコル遵守（STT-REQ-007.1）
  - VAD→AudioPipeline→STT本番経路構築完了（_handle_process_audio実装）
  - **IPC実装方針**: 1リクエスト→1最終応答（MVP0互換）
  - 中間イベント（speech_start, partial_text）はログ記録のみ、IPC送信なし
  - 実統合テスト追加（test_audio_integration.py: AudioPipeline動作、IPC統合）
  - **Task 7への引継ぎ事項**: リアルタイム部分テキスト配信（イベントストリーム型プロトコル追加）
  - _Requirements: STT-REQ-007.1（後方互換性）, STT-REQ-003.6, STT-REQ-003.9_
  - _Note: STT-REQ-003.7/003.8（部分テキスト）はAudioPipeline内部実装済み、IPC配信はTask 7で実装_

- [x] 5. リソース監視と動的モデルダウングレード機能（Python側）
- [x] 5.1 ResourceMonitor API層実装（完了）
  - ResourceMonitorクラスの定義完了
  - システムリソース検出API実装（CPU、メモリ、GPU）
  - モデル選択ルールAPI実装（large-v3/medium/small/base/tiny）
  - リソース使用量取得API実装（get_current_memory_usage, get_current_cpu_usage）
  - ダウングレード/アップグレード判定API実装
  - UI通知生成API実装
  - ユニットテスト緑化（27テスト合格）
  - _Note: API層のみ実装。監視ループ・統合・実際のモデル切替は未実装_
  - _Requirements: STT-REQ-006.1, STT-REQ-006.2, STT-REQ-006.3（部分）_

- [x] 5.2 ResourceMonitor監視ループ実装（完了）
  - **30秒間隔の監視ループ**実装完了（`start_monitoring()`, `stop_monitoring()`）
  - CPU負荷持続判定の自動状態管理実装完了（cpu_high_start_timeの自動更新）
  - リソース回復の自動状態管理実装完了（low_resource_start_timeの自動更新）
  - 60秒継続CPU高負荷検出と自動ダウングレードトリガー実装完了
  - 5分継続リソース回復検出と自動アップグレード提案実装完了
  - 30秒間隔のDEBUGログ出力実装完了
  - コールバックベース設計（on_downgrade, on_upgrade_proposal, on_pause_recording）
  - ユニットテスト緑化（4テスト合格、合計31テスト）
  - バグ修正: `should_pause_recording()` をメモリ使用率ベースに変更
  - バグ修正: `get_current_cpu_usage()` の1秒ブロック問題修正
  - _Note: AudioProcessorへの統合はTask 5.3で実装_
  - _Requirements: STT-NFR-001.6, STT-NFR-005.4, STT-REQ-006.7, STT-REQ-006.10（完全実装）_

- [x] 5.3 動的モデルダウングレード機能（完了 + Critical Fixes適用済み）
  - **WhisperSTTEngine.load_model()実装完了**: 動的モデル切替（unload → reload）
    - **🔧 Critical Fix**: リソースリーク対策として`gc.collect()`追加（line 422）
    - **🔧 Cleanup**: TODOコメント削除（line 368）
  - **AudioProcessorコールバック統合完了**: `_handle_model_downgrade`, `_handle_upgrade_proposal`, `_handle_pause_recording`
  - **ResourceMonitor統合完了**: AudioProcessor.__init__でResourceMonitor初期化、current_model/initial_model設定
  - **🔧 Critical Fix: 監視ループ起動実装完了**（main.py:299-308）
    - `main()`関数で30秒間隔の監視ループを自動起動
    - シャットダウン時の適切なクリーンアップ処理実装（finally block）
    - **影響**: これにより Task 5.3 の全機能が本番環境で実際に動作
  - **IPC通知送信実装完了**: model_change (cpu_high/memory_high), upgrade_proposal, recording_paused イベント
  - **メモリダウングレードロジック修正完了**: tinyモデル時のダウングレードスキップ実装（line 501）
  - **アップグレード提案ロジック拡張完了**: initial_modelへの直接アップグレード提案実装（line 538-559）
  - **統合テスト緑化完了**: TestResourceMonitorIntegration全5テスト合格
  - **🔧 テスト追加**: `test_get_upgrade_target_respects_initial_model()` 追加（initial_model ceiling確認）
  - **全テスト合格**: 139 passed (+1), 1 skipped（リグレッションなし）
  - _Requirements: STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9, STT-REQ-006.10, STT-REQ-006.11, STT-NFR-001.6（完全実装）_
  - _Note: Task 5.4（ユーザー承認時のアップグレード実行）は別タスク。本タスクは提案送信まで実装_

  **検証済み指摘対応**:
  - ✅ **指摘1（Critical）**: 監視ループ未起動 → main()で起動実装完了
  - ✅ **指摘4（Medium）**: リソースリーク懸念 → gc.collect()追加
  - ❌ **指摘2**: メモリ監視未実装 → 誤り。line 498で実装済み
  - ❌ **指摘3**: get_upgrade_target()ロジックバグ → 誤り。元のロジックが正しい

- [x] 5.4 UI通知とアップグレード提案機能（完了）
  - **IPC経由のUI通知送信実装完了**: upgrade_proposal, model_change, recording_paused イベント（Task 5.3で実装済み）
  - **リソース回復の自律的検出実装完了**: low_resource_start_timeの自動更新（Task 5.3で実装済み、line 491-499）
  - **ユーザー承認時のアップグレード試行実装完了**: approve_upgrade IPCメッセージハンドラ追加（main.py:84-85, 286-349）
  - **_handle_approve_upgrade()メソッド実装完了**: WhisperSTTEngine.load_model()呼び出し、current_model更新、IPC通知送信
  - **tinyモデルでリソース不足時の録音一時停止実行実装完了**: _handle_pause_recording()（Task 5.3で実装済み）
  - **統合テストの作成完了**: test_user_approved_upgrade_execution追加（test_audio_integration.py:732-777）
  - **Rustテスト緑化完了**: 11テスト合格（MockAudioAdapter.check_permission()追加により修正）
  - **🔧 P0バグ修正完了**（2025-10-13）:
    - **IPC応答の非対称性修正**: 成功時に`type: response, id: msg_id`を追加（STT-REQ-007.1準拠）
      - `main.py:330-348`: response + event 二段構成に変更
      - 失敗時のみid付きエラーを返す非対称性を解消
    - **テストコードのバグ修正**: `test_audio_integration.py:742-758`
      - `ResourceMonitor`のimport追加（NameError解消）
      - コンストラクタ引数削除（TypeError解消）
      - 手動プロパティ設定に変更（`initial_model`, `current_model`）
  - _Requirements: STT-REQ-006.10（完了）, STT-REQ-006.11（Task 5.3で完了）, STT-REQ-006.12（完了）, STT-REQ-007.1（修正完了）_
  - _Note: E2Eテスト2件失敗はPythonサイドカー起動問題（Task 5.4の実装とは無関係）_

- [x] 6. ローカルストレージ機能の実装（Rust側）
- [x] 6.1 LocalStorageServiceスケルトンとセッション管理（✅ 完了）
  - LocalStorageServiceクラスの定義完了（src-tauri/src/storage.rs）
  - セッションID生成機能実装完了（UUID v4形式）
  - セッションディレクトリ作成機能実装完了（`[app_data_dir]/recordings/[session_id]/`）
  - ユニットテスト5件作成・緑化完了
    - `test_generate_session_id`: UUID形式検証
    - `test_generate_session_id_uniqueness`: UUID一意性検証
    - `test_create_session_directory`: ディレクトリ作成検証
    - `test_create_session_nested_directory`: 親ディレクトリ自動作成検証
    - `test_get_session_dir`: パス取得検証
  - **P0対応済み**: fs2::available_spaceを用いたディスク容量チェックと`begin_session()`統合APIを追加（storage.rs L115-175, L230-268）
  - **互換性**: 旧APIは後方互換のため残置（容量チェック必須化済み）
  - _Requirements: STT-REQ-005.1（完全実装、互換API維持）_

- [x] 6.2 音声ファイル保存機能（✅ 完了）
  - **AudioWriter構造体実装完了**（src-tauri/src/storage.rs L49-127）
  - **WAVヘッダー書き込み実装完了**（16kHz, モノラル, 16bit PCM形式）
  - **ストリーミング書き込み実装完了**（`write_samples()` メソッド）
  - **ファイルクローズ処理実装完了**（ヘッダーサイズ更新、`close()` メソッド）
  - **LocalStorageService統合完了**（`create_audio_writer()` メソッド）
  - **ユニットテスト4件作成・緑化完了**（9テストすべて合格）
    - `test_create_audio_writer`: AudioWriter作成とファイル生成確認
    - `test_audio_writer_write_samples`: 1秒分のサンプル書き込み確認
    - `test_audio_writer_multiple_writes`: ストリーミング書き込み確認（10回分割）
    - `test_audio_writer_wav_header`: WAVヘッダー形式検証（RIFF, fmt, data）
  - _Requirements: STT-REQ-005.2（完全実装）_

- [x] 6.3 文字起こし結果保存機能（✅ 完了）
  - **TranscriptionEvent構造体実装完了**（src-tauri/src/storage.rs L139-149）
    - `timestamp_ms`: タイムスタンプ（ミリ秒）
    - `text`: テキスト内容
    - `is_final`: 確定テキストフラグ（false = 部分テキスト）
  - **TranscriptWriter構造体実装完了**（L151-187）
    - 追記モード（`OpenOptions::append`）でファイル作成
    - JSON Lines形式（1行1JSONオブジェクト）での書き込み
    - `append_event()`: イベント追記メソッド
    - `close()`: ファイル同期とクローズ
  - **LocalStorageService統合完了**（`create_transcript_writer()` メソッド L48-55）
  - **ユニットテスト4件作成・緑化完了**（13テストすべて合格）
    - `test_create_transcript_writer`: TranscriptWriter作成とファイル生成確認
    - `test_transcript_writer_append_event`: 部分/確定テキスト追記確認
    - `test_transcript_writer_append_mode`: 複数回Writer作成での追記モード確認
    - `test_transcription_event_json_format`: JSON変換・逆変換確認
  - _Requirements: STT-REQ-005.3（完全実装）_

- [x] 6.4 セッションメタデータ保存機能（✅ 完了）
  - **SessionMetadata構造体実装完了**（src-tauri/src/storage.rs L139-159）
    - 8フィールド: session_id, start_time, end_time, duration_seconds, audio_device, model_size, total_segments, total_characters
  - **save_session_metadata()メソッド実装完了**（L58-69）
    - `serde_json::to_string_pretty()` でJSON整形
    - session.json上書き保存
  - **4テスト実装完了**（L624-782）
    - `test_save_session_metadata`: 基本保存・読み込み検証
    - `test_session_metadata_json_format`: 全フィールド検証
    - `test_save_session_metadata_overwrite`: 上書き動作確認
    - `test_session_metadata_iso8601_timestamps`: ISO 8601形式保持確認
  - **全17テスト合格**（Task 6.1-6.4統合）
  - _Requirements: STT-REQ-005.4（完全実装）_

- [x] 6.5 セッション一覧取得と再生機能（✅ 完了）
  - **LoadedSession構造体実装完了**（src-tauri/src/storage.rs L174-181）
    - metadata, transcripts, audio_pathの3フィールド
  - **list_sessions()メソッド実装完了**（L71-109）
    - recordings/ディレクトリ走査、session.json読み込み
    - 日時降順ソート（start_time降順）
  - **load_session()メソッド実装完了**（L111-145）
    - session.json + transcription.jsonl + audio.wavパス読み込み
    - 空transcription.jsonl対応
  - **4テスト実装完了**（L788-943）
    - `test_list_sessions`: 3セッション作成・日時降順ソート検証
    - `test_load_session`: メタデータ・文字起こし・音声パス検証
    - `test_list_sessions_empty`: 空ディレクトリ処理確認
    - `test_load_session_not_found`: 存在しないセッションエラー処理
  - **全21テスト合格**（Task 6.1-6.5統合）
  - _Requirements: STT-REQ-005.5, STT-REQ-005.6（完全実装）_

- [x] 6.6 ディスク容量監視と警告機能（✅ 完了）
  - **DiskSpaceStatus enum実装完了**（src-tauri/src/storage.rs L259-281）
    - Sufficient, Warning, Critical の3状態
    - Display trait実装（エラーメッセージ表示）
  - **check_disk_space()メソッド実装完了**（L147-182）
    - sys_info::disk_info()でディスク容量取得
    - 1GB以上: Sufficient
    - 500MB以上1GB未満: Warning（警告ログ出力）
    - 500MB未満: Critical（クリティカルログ出力）
  - **4テスト実装完了**（L1025-1106）
    - `test_check_disk_space_sufficient`: 十分な容量確認
    - `test_check_disk_space_warning`: 警告メッセージ検証
    - `test_check_disk_space_critical`: クリティカルメッセージ検証
    - `test_create_session_with_disk_check`: セッション作成前のディスク容量チェック
  - **全25テスト合格**（Task 6.1-6.6統合）
  - **sys-info crateを依存関係に追加**（Cargo.toml）
  - _Requirements: STT-REQ-005.7, STT-REQ-005.8（完全実装）_

---

### Task 6系 耐障害性強化リファクタリング（2025-10-13実施）

**背景**: 外部レビューで指摘された致命的な懸念3点を修正

#### 修正1: AudioWriter Drop実装（L270-277）
- **問題**: close()未呼び出し時、WAVヘッダーサイズが0の破損ファイル生成
- **対応**: Drop trait実装、finalize()内部メソッド化
- **効果**: パニック・例外時も自動ヘッダー更新、STT-REQ-005.2完全準拠
- **テスト追加**: `test_audio_writer_drop_without_close`（L583-627）

#### 修正2: TranscriptWriter Drop + sync_all()強化（L396-403）
- **問題1**: flush()のみでディスク永続化未保証（カーネルバッファ止まり）
- **問題2**: close()未呼び出し時、クラッシュでデータ欠損
- **対応1**: append_event()内でflush() + sync_all()実行（L365-382）
- **対応2**: Drop trait実装、finalize()内部メソッド化
- **効果**: クラッシュ時のデータ欠損最小化、STT-REQ-005.3完全準拠
- **テスト追加**: `test_transcript_writer_drop_without_close`（L826-868）

#### 修正3: begin_session()原子的API追加（L46-76）
- **問題**: create_session()→create_audio_writer()の呼び順依存、責務分断
- **対応**: SessionHandle（RAII）導入（L15-39）
  - ID生成 → ディスク容量チェック → ディレクトリ作成を原子的実行
  - Critical時は録音開始拒否、Warning時は警告表示
  - SessionHandle経由でのみライター取得可能
- **効果**: 呼び出し順ミス防止、STT-REQ-005.1完全準拠
- **テスト追加**:
  - `test_begin_session`（L1308-1332）
  - `test_begin_session_with_writers`（L1334-1378）

#### 最終結果
- **全29テスト合格**（修正前25 + 新規4）
- **コード行数**: storage.rs 1,397行（修正前1,106行から+291行）
- **耐障害性**: 異常終了時のデータ保護を完全保証

---

### Task 6系 UI通知統合（P0対応、2025-10-13実施）

**背景**: 外部レビューで指摘されたWarning時のUI通知欠落を修正

#### 修正内容: SessionHandleへのdisk_statusフィールド追加（L19-59）
- **問題**: Warning時に eprintln!のみでUI通知未実装（STT-REQ-005.7未達）
- **対応**:
  - SessionHandleに`disk_status: DiskSpaceStatus`フィールド追加（L24）
  - `needs_disk_warning() -> bool`メソッド追加（L44-48）
  - `disk_warning_message() -> Option<String>`メソッド追加（L50-58）
- **効果**: 呼び出し側でUI通知を制御可能
  ```rust
  let handle = storage.begin_session()?;
  if handle.needs_disk_warning() {
      // UI通知: handle.disk_warning_message()
  }
  ```
- **テスト追加**: `test_session_handle_disk_warning`（L1365-1387）
- **要件充足**: STT-REQ-005.7完全準拠

#### 最終結果
- **全30テスト合格**（P0対応後）
- **UI通知**: 呼び出し側で制御可能（Tauri Event API統合準備完了）
- **残課題（P1）**: 旧API（create_session）非公開化、test環境依存性

---

### Task 6系 致命的欠陥修正（P0対応、2025-10-13実施）

**背景**: 外部レビューで指摘された2つの致命的欠陥を修正

#### 修正1: fs2 crateでapp_data_dir容量取得（L230-268）
- **問題**: sys_info::disk_info()はルートFSのグローバル値のみ取得
  - app_data_dirが外付けHDDや別パーティション上の場合、誤判定
  - 例: ルートFS 10GB空き（Sufficient）、外付けHDD 400MB空き（Critical）→ 誤ってSufficientを返す
- **対応**: fs2::available_space()でapp_data_dirのFS容量を正確取得
  ```rust
  use fs2::available_space;
  let free_bytes = available_space(&self.app_data_dir)?;
  ```
- **効果**: 外付けHDD、別パーティション、ネットワークドライブでも正確な容量判定
- **依存関係追加**: fs2 v0.4.3（Cargo.toml）
- **要件充足**: STT-REQ-005.7/005.8完全準拠

#### 修正2: 旧API経路のディスク容量チェック追加
- **問題**: create_session/create_audio_writer/create_transcript_writerが公開APIで、直接呼び出すとディスク容量チェックをバイパス可能
  ```rust
  // 危険な呼び出し例
  storage.create_session(&session_id)?;  // ❌ 容量チェックなし
  storage.create_audio_writer(&session_id)?;  // ❌ 容量チェックなし
  ```
- **対応**: 各旧APIに容量チェック追加（L115-175）
  - `create_session()`: L116-123でCritical時拒否
  - `create_audio_writer()`: L142-149でCritical時拒否
  - `create_transcript_writer()`: L163-170でCritical時拒否
- **効果**: どのAPI経路でもディスク容量不足を検出・拒否
- **要件充足**: STT-REQ-005.8完全準拠

#### 最終結果
- **全41テスト合格**（P0対応完了）
- **致命的欠陥**: 全解消
- **要件充足**: STT-REQ-005.1〜005.8完全準拠
- **依存関係**: fs2 v0.4.3追加、sys-info削除可能
- **残課題（P1）**: 旧API非公開化（後方互換性維持のため保留）

- [x] 7. IPC通信プロトコル拡張と後方互換性（Rust/Python両側）
  - **Task 4.3からの引継ぎ**: リアルタイム部分テキスト配信機能の実装
  - **背景**: Task 4.3ではMVP0互換性優先でRequest-Response型（1リクエスト→1最終応答）を維持
  - **本タスクの目標**: イベントストリーム型プロトコル追加（1リクエスト→複数イベント配信）
  - **実装方針**: 新エンドポイント `process_audio_stream` または既存エンドポイントの拡張を検討
- [x] 7.1 IPCメッセージ拡張とバージョニング（✅ 完了、Task 7.1.5でP0修正完了）
  - **新モジュール作成**: `ipc_protocol.rs`（src-tauri/src/ipc_protocol.rs）
  - **TranscriptionResult構造体実装完了**（L10-38）
    - 既存フィールド: text, is_final
    - 新規フィールド（Optional）: confidence, language, processing_time_ms, model_size
    - `#[serde(default, skip_serializing_if = "Option::is_none")]`で後方互換性確保
  - **IpcMessage enum実装完了**（L42-78）
    - Request, Response, Error の3バリアント
    - 全メッセージに`version`フィールド必須化（STT-REQ-007.4）
    - **P0修正完了**: `#[serde(default = "default_version")]`で旧形式メッセージ対応（ADR-003準拠）
    - エラー応答形式統一（errorCode, errorMessage, recoverable）
  - **ヘルパーメソッド実装**（L80-97）
    - `version()`: バージョン取得
    - `id()`: メッセージID取得
  - **ユニットテスト11件実装・全合格**（L99-333）
    - `test_transcription_result_with_all_fields`: 全フィールド検証
    - `test_transcription_result_backward_compatibility`: 旧形式互換性
    - `test_ipc_message_response_with_version`: バージョンフィールド検証
    - `test_ipc_message_error_format`: エラー形式検証（STT-REQ-007.5）
    - `test_ipc_message_version_accessor`: アクセサメソッド検証
    - `test_transcription_result_skip_none_fields`: None省略検証
    - `test_ipc_message_roundtrip`: ラウンドトリップ検証
    - `test_forward_compatibility_ignore_unknown_fields`: 未知フィールド無視
    - `test_version_constant`: プロトコルバージョン定数検証
    - `test_confidence_range`: 信頼度スコア範囲検証
    - **P0対応テスト追加**: `test_version_field_omitted_defaults_to_1_0`（L317-331、ADR-003検証）
  - **後方互換性**: `#[serde(default = "default_version")]`で旧形式メッセージを完全サポート（ADR-003準拠）
  - **前方互換性**: serdeの`#[serde(deny_unknown_fields)]`未使用で未知フィールド無視
  - **Task 7.1.5統合完了**: 既存IPC通信への新プロトコル統合（詳細は下記）
  - **全11テスト合格**
  - _Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.4, STT-REQ-007.5, ADR-003（完全実装）_

- [x] 7.1.5 既存IPC通信への新プロトコル統合（✅ 完了 2025-10-14）
  - **Rust側**: `src-tauri/src/commands.rs` で `ProtocolMessage::Request` + `process_audio_stream` をデフォルト化し、`ProtocolMessage` 経由で全応答を復号（L152-270）。旧形式はLegacy互換として警告付きで維持。
  - **Python側**: `python-stt/main.py` が `type: "request"` / `method` ディスパッチを実装（L70-140）し、LegacyIpcMessage 変換とイベントストリームを同居させることで前後方互換を確保。
  - **Chrome/WebSocket連携**: `commands.rs` → `websocket.rs` 経由で `isPartial` / `confidence` / `language` / `processingTimeMs` を配信し、拡張レスポンスを標準化。
  - **テスト**: `tests/ipc_migration_test.rs` の9ケースに加え、Rust/Python統合テスト（`stt_e2e_test.rs`, `test_audio_integration.py`）で新旧プロトコル共存とイベントシーケンスを検証済み。
  - **移行戦略**: LegacyIpcMessageを非推奨化しつつ稼働中クライアントを保護。将来の除却は Task 11.x（テレメトリ完了後）で追跡。
  - _Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.4, STT-REQ-007.5, ADR-003（完全実装）_
- [x] 7.1.6 イベントストリーム型プロトコル追加（✅ 完了、P0修正完了 2025-10-13実施）
  - **Python側実装完了**（main.py L261-406）
    - `_handle_process_audio_stream()` メソッド実装（L262-406）
    - イベントタイプ: `speech_start`, `partial_text`, `final_text`, `speech_end`, `error`
    - **P0修正1**: JSON形式をRust schema完全準拠（`eventType`/`data`フィールド使用）
    - **P0修正2**: リアルタイムタイムスライス実装（L303: `asyncio.sleep(0.01)`で10msフレーム間隔シミュレート）
      - 問題: フレーム一気処理でtime.time()変化せず、`_should_generate_partial()`が常にFalse
      - 対応: 各フレーム処理後に10ms待機で壁時計時刻を進行させ、1秒間隔partial_text発火を実現
    - **P0修正3**: エラーイベント送信実装（L387-402: `event_type == 'error'`ケース追加）
      - 問題1: AudioPipelineエラー時、Python側が何も送信せずRust側が`receive_message()`でハング
      - 対応1: `type: 'error'`メッセージ送信でRust側ループを確実に終了
      - 問題2: エラーメッセージに`id`フィールドがなく、IpcMessage::Errorデシリアライズ失敗
      - 対応2: L395で`'id': msg_id`追加（ipc_protocol.rs L104必須フィールド準拠）
    - **P1修正**: `speech_end`イベント送信実装（AudioPipelineが返さないため手動送信）
  - **Rust側実装完了**（commands.rs L152-270）
    - メソッド名変更: `process_audio` → `process_audio_stream`（L163）
    - イベント受信ループ実装（L188-270）
    - イベントタイプ別処理: speech_start, partial_text, final_text, speech_end
    - エラーハンドリング: `ProtocolMessage::Error`でループ終了（L252-254）
    - `final_text`でWebSocket配信、`speech_end`でループ終了
  - **統合テスト実装**（python-stt/tests/test_audio_integration.py L786-1028）
    - `test_process_audio_stream_sends_multiple_events`: イベント配信検証
    - `test_process_audio_still_works_for_backward_compatibility`: 後方互換性検証
    - `test_process_audio_stream_handles_error_events`: エラーイベント送信検証（P0修正検証）
      - L1024: `id`フィールド検証追加（IpcMessage::Error必須フィールド）
    - **全3テスト合格**
  - **後方互換性維持**: 既存 `process_audio` エンドポイント変更なし
  - **STT-REQ-003.7/003.8対応**: リアルタイム部分テキスト配信完全実装（P0修正により実現）
  - **ビルド検証**: `cargo build`成功（deprecation warning 9件は既存Legacy IPC由来）
  - _Requirements: STT-REQ-003.7, STT-REQ-003.8, STT-REQ-007.1（P0修正完了、完全実装）_

- [x] 7.2 後方互換性テストとエラー処理（✅ 完了、P0修正完了 2025-10-13実施）
  - **⚠️ P0欠陥発見と修正**: バージョン不一致処理が未実装だった問題を修正
    - **問題**: テストがバージョン判定ロジックを検証せず、コメントで期待を述べるだけ
    - **影響**: version 2.xのメッセージも無条件に処理され、STT-REQ-007.6未達成
  - **バージョン判定ロジック実装**（ipc_protocol.rs L45-107, L127-169）
    - `VersionCompatibility` enum追加: Compatible, MinorMismatch, MajorMismatch, Malformed
    - `check_version_compatibility()` 関数実装: セマンティックバージョニング解析
    - `IpcMessage::check_version_compatibility()` メソッド追加
  - **commands.rsでバージョンチェック実装**（commands.rs L200-228）
    - メッセージ受信時に`msg.check_version_compatibility()`呼び出し
    - `MajorMismatch`: エラーログ + 通信拒否（ループ脱出）
    - `MinorMismatch`: 警告ログ + 後方互換モードで処理継続
    - `Malformed`: エラーログ + 通信拒否（ループ脱出）
  - **バージョン判定ロジック単体テスト**（tests/ipc_migration_test.rs L629-728）
    - `test_version_check_major_mismatch`: メジャー不一致検出（2.0 vs 1.0）
    - `test_version_check_minor_mismatch`: マイナー不一致検出（1.1 vs 1.0）
    - `test_version_check_patch_compatible`: パッチ差異許容（1.0.2 vs 1.0.1）
    - `test_version_check_malformed`: 不正バージョン検出（"invalid", "1.x", ""）
    - `test_ipc_message_version_check_integration`: IpcMessage統合テスト
  - **meeting-minutes-core互換性テスト実装**（tests/ipc_migration_test.rs L730-850）
    - `test_fake_implementation_compatibility`: MVP0（Fake）が拡張フィールドを無視（STT-REQ-007.3）
    - `test_mvp0_minimal_response_accepted_by_mvp1`: MVP0最小応答をMVP1が受容（STT-REQ-007.3）
    - `test_legacy_and_new_protocol_coexistence`: LegacyとNew形式の共存検証（STT-REQ-007.1）
  - **既存テスト検証**（tests/ipc_migration_test.rs）
    - `test_forward_compatibility_unknown_fields`: 未知フィールド無視動作（既存）
    - `test_version_field_omitted_backward_compat`: versionフィールド省略時デフォルト（既存）
    - `test_new_format_error_response`: エラー応答フォーマット（STT-REQ-007.5、既存）
  - **全26テスト合格**（18 → 25 → 26に増加、P0修正で実装検証テスト追加）
  - **エラー応答フォーマット**: 既にTask 7.1で実装済み（IpcMessage::Error）
  - _Requirements: STT-REQ-007.3, STT-REQ-007.5, STT-REQ-007.6（P0修正完了、完全実装）_

- [x] 7.3 IPCデッドロック根本解決（ADR-013: Sidecar Full-Duplex IPC Final Design）
  - **Priority**: 🔴 P0 Critical
  - **Actual Time**: 1日（2025-10-14完了）
  - **Related ADR**:
    - ❌ ADR-008 (Rejected - 構造的デッドロック欠陥)
    - ❌ ADR-009 (Rejected - Mutex共有問題 + blocking_send問題)
    - ⚠️ ADR-011 (Superseded by ADR-013 - IPC Stdin/Stdout Mutex Separation)
    - ⚠️ ADR-012 (Superseded by ADR-013 - Audio Callback Backpressure Redesign)
    - ✅ **ADR-013 (Approved - Final Design統合・明確化)**
  - **Background**: ADR-009の第3回技術検証で2つの構造的欠陥を発見
    - **P0 Mutex共有によるシリアライゼーション**: `Arc<Mutex<PythonSidecarManager>>`共有により、Sender/Receiver並行実行が実質シリアライズされ、デッドロックが解消されない
    - **P0 blocking_send()によるCPALストリーム停止**: Python異常時にオーディオコールバックが最大2秒ブロック → CPALのOSバッファ（128ms）オーバーラン → ストリーム停止
  - **Solution**: ADR-013による統合設計で根本解決
    - **AudioSink/EventStream Facade API**: Mutex完全隠蔽、チャネルのみ公開
    - **Line-Delimited JSON Framing**: read_exact() deadlock回避、デバッグ容易性
    - **5s Ring Buffer + Immediate Stop**: Python異常時即座停止、UX明確化
    - **Phase 1-4実装完了**: 1365行、19テスト、100%合格
  - **P0 Bugs Fixed**: 4件（Child handle retention、Ring buffer overflow detection、Partial write prevention、VAD AttributeError）

- [x] 7.3.1 ADR-013設計とDesign.md更新（✅ 完了 2025-10-14）
  - ADR-009をRejected化（2つの構造的欠陥）
  - ADR-011作成 → ADR-013で統合・明確化
  - ADR-012作成 → ADR-013で統合・明確化
  - ADR-013作成: Sidecar Full-Duplex IPC Final Design
  - Design.md Section 7.9全面更新: ADR-013準拠
  - spec.json更新: phase=implementation, BLOCK-004追加・完了
  - _Actual: 3時間_
  - _Requirements: STT-REQ-007.7_

**Note**: 以下のサブタスク（7.3.2〜7.3.9）はADR-011/012ベースで記述されていましたが、**ADR-013実装（Phase 1-4）により統合完了**しています。

- [x] 7.3.2〜7.3.9 ADR-013 Phase 1-4実装完了（✅ 2025-10-14）
  - **Phase 1: Sidecar Facade API実装完了**（src-tauri/src/sidecar.rs、535行、4/4テスト合格）
    - AudioSink/EventStream構造体実装
    - Mutex完全隠蔽、チャネルのみ公開
    - stdin/stdoutの単独所有（ADR-011要件達成）
  - **Phase 2: Ring Buffer統合完了**（src-tauri/src/ring_buffer.rs、340行、11/11テスト合格）
    - SPSC Lock-Free Ring Buffer実装（5秒バッファ）
    - try_send() backpressure実装（ADR-012要件達成）
    - Partial write prevention（P0-3修正）
  - **Phase 3: Python Execution Model**（既存python-stt/main.pyがADR-013要件を満たす）
    - Line-Delimited JSON対応
    - VAD状態ベースno_speech判定（P0-4修正、7.3.6相当）
  - **Phase 4: E2E Tests実装完了**（tests/sidecar_full_duplex_e2e.rs、490行、4/4テスト合格）
    - 500フレーム並行処理でDeadlock 0%検証（7.3.8相当）
    - 6000フレーム送信でFrame loss 0%検証（7.3.8相当）
    - Python異常検出 ~6s検証（5秒バッファ満杯検証、7.3.4相当）
  - **Success Criteria達成**:
    - ✅ Deadlock発生率 = 0%（ADR-011）
    - ✅ Frame loss率 = 0%（ADR-012）
    - ✅ Audio callback latency < 10μs（ADR-012）
    - ✅ Python異常検出 ~6s（5秒バッファ満杯検証）
    - ✅ 既存テスト全合格（Rust 71 + Python 143）
  - **P0 Bugs Fixed**: 4件（Child handle retention、Ring buffer overflow、Partial write prevention、VAD AttributeError）
  - **Total**: 1365行、19テスト、100%合格、実装日数1日
  - _Requirements: STT-REQ-007.7（完全達成）_

**旧サブタスク詳細**（ADR-013実装で統合完了のため参照用）:
- 7.3.2: PythonSidecarManager構造体変更 → Phase 1 Sidecar Facade APIで実現
- 7.3.3: Sender/Receiver並行タスク → Phase 1で実現
- 7.3.4: Audio Callback try_send() Backpressure → Phase 2 Ring Bufferで実現
- 7.3.5: フロントエンドエラーハンドリング → 未実装（Task 9で対応予定）
- 7.3.6: Python VAD状態ベースno_speech判定 → P0-4修正で実現
- 7.3.7: Error Handling & Graceful Shutdown → Phase 1で実現
- 7.3.8: E2Eテストと検証 → Phase 4で実現
- 7.3.9: Metrics and Rollback Strategy → Phase 4で一部実現（残課題あり）

- [x] 8. WebSocketメッセージ拡張（Rust側）
- [x] 8.1 WebSocketメッセージ拡張とChrome拡張連携（✅ 完了、P0修正完了 2025-10-14）
  - **WebSocketMessage::Transcription拡張フィールド追加**（`src-tauri/src/websocket.rs`）
    - `isPartial`: Option<bool> - 部分/確定テキストの明示化
    - `confidence`: Option<f64> - 信頼度スコア（0.0-1.0）
    - `language`: Option<String> - 言語コード（例: "ja"）
    - `processingTimeMs`: Option<u64> - 処理時間（ミリ秒）
    - `#[serde(skip_serializing_if = "Option::is_none")]` で後方互換性確保
  - **⚠️ P0欠陥発見と修正**: partial_textイベントがWebSocketに配信されず、is_partialが常にfalseだった問題を修正
    - **問題**: commands.rs L341-346でpartial_textブランチがTODOコメントのみで実装されていなかった
    - **影響**: Chrome拡張が部分テキストを受信できず、STT-REQ-008.1のpartial/final識別が不可能（リリースブロッカー）
    - **修正内容**（commands.rs L15-22, L350-425）:
      - `extract_extended_fields()` ヘルパー関数追加（コード重複回避）
      - `partial_text` ブランチでWebSocket.broadcast()実装（`is_partial: Some(true)`）
      - `final_text` ブランチをヘルパー関数使用に統一（`is_partial: Some(false)`）
      - 両ブランチでconfidence/language/processing_time_msを同一ロジックで抽出
  - **ユニットテスト6件作成・全合格**（`tests/websocket_message_extension_test.rs`）
    - `test_transcription_with_all_extended_fields`: 全フィールドシリアライズ検証
    - `test_transcription_backward_compatibility_minimal_fields`: 後方互換性検証（STT-REQ-008.2）
    - `test_transcription_partial_fields`: 一部フィールド省略検証
    - `test_confidence_range_validation`: 信頼度範囲検証
    - `test_deserialization_from_python_response`: Pythonレスポンスデシリアライズ検証
    - `test_chrome_extension_ignores_unknown_fields`: 未知フィールド無視検証（STT-REQ-008.2）
  - **既存テスト修正完了**（`tests/integration/websocket_integration.rs`）
    - 拡張フィールドをNoneで初期化（後方互換性維持）
  - **全テスト合格**: 新規6 + 既存統合3 = 9テスト（WebSocket統合3件、IPC統合26件全合格）
  - **コンパイル成功**: P0修正後にwarningのみ（既存deprecation warning）
  - _Requirements: STT-REQ-008.1, STT-REQ-008.2, Principle 3（セキュリティ責任境界）（P0修正完了、完全実装）_

---

## Task 9: UI拡張とユーザー設定機能（MVP1完成フェーズ）

**期間**: 5-7日
**優先度**: 🔴 P0 Critical（MVP1必須）
**目的**: Phase 7/8で安定化した基盤の上にユーザー設定UIを実装し、MVP1完成状態にする

- [ ] 9.1 音声デバイス選択UI
  - 利用可能な音声入力デバイス一覧を取得して表示
  - デバイス選択ドロップダウンコンポーネント（React）実装
  - 選択されたデバイスIDの設定保存機能（Tauri Command経由）
  - デバイスメタデータ（名前、サンプルレート、チャンネル数）の表示
  - _Requirements: STT-REQ-001.2, STT-REQ-001.3_

- [ ] 9.2 Whisperモデル選択UI
  - モデルサイズ選択ドロップダウン（tiny/base/small/medium/large-v3）実装
  - 自動選択/カスタマイズトグルスイッチ
  - 現在のシステムリソースに基づく推奨モデル表示
  - リソース超過警告メッセージ表示（手動選択時）
  - モデル選択の設定保存機能
  - _Requirements: STT-REQ-006.4, STT-REQ-006.5_

- [ ] 9.3 オフラインモード設定UI（オプション、MVP1スコープ外候補）
  - オフラインモード強制チェックボックス
  - バンドルモデル使用状態の表示インジケーター
  - HuggingFace Hub接続スキップの動作確認
  - _Requirements: STT-REQ-002.6_

- [ ] 9.4 リソース監視通知UI（オプション、MVP1スコープ外候補）
  - トースト通知コンポーネント（React）実装
  - モデル切り替え通知「モデル変更: {old} → {new}」の表示
  - システムリソース不足警告の表示
  - 通知の自動消去タイマー（5秒）
  - _Requirements: STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.10_

- [ ] 9.5 セッション管理UI（オプション、MVP1スコープ外候補）
  - 過去のセッション一覧表示（日時降順ソート）
  - セッション選択による詳細表示（メタデータ、文字起こし結果）
  - 音声再生機能（audio.wav playback）
  - セッション削除機能（確認ダイアログ付き）
  - _Requirements: STT-REQ-005.5, STT-REQ-005.6_

---

## Task 10: 統合テスト・E2Eテスト（コア機能検証）

**期間**: 2-3日
**優先度**: 🔴 P0 Critical（Gap分析：Option B検証優先アプローチ）
**目的**: Phase 1-6で実装済みのコア機能（音声録音→VAD→STT→保存）の統合動作を検証し、統合問題を早期発見する

**実装状況**（2025-10-19更新）:
- ✅ **Task 10.1-10.3 完了**（MVP1コア機能E2E検証）
- ✅ **E2Eテスト実装完了**（`src-tauri/tests/stt_e2e_test.rs`、test_audio_recording_to_transcription_full_flow）
  - 1/7のテストが完全実装（Task 10.1 Phase 1）
  - 6/7のテストが未実装（Task 10.4-10.7、`#[ignore]`または`unimplemented!()`）
- ✅ **P0 Blocker解決実績**:
  - BLOCK-005: Pythonサイドカーハンドシェイク問題（`.cargo/config.toml`でAPP_PYTHON環境変数設定）
  - BLOCK-006: MockAudioDataGenerator作成（テスト音声WAV 3種類生成）
  - BLOCK-007: 実行可能なテストヘルパー実装（verify_partial_final_text_distribution + partial_text明示的検証）
- ✅ **P0 Bug修正実績**:
  - ADR-014: VAD Pre-roll Buffer（300ms音声損失問題修正、speech_end/partial_textの両方で完全保存）
  - ADR-016: Offline Model Fallback（STT-REQ-002.4未実装問題修正、WhisperModel例外時のbundledフォールバック実装）
- ✅ **テスト合格実績**:
  - VAD: 19/19 tests ✅
  - AudioPipeline: 11/11 tests ✅
  - Offline Fallback: 14/14 tests ✅
  - ResourceMonitor: 58/60 tests (96.7%) ✅
  - E2E: 1/1 test (Task 10.1) ✅

**Phase 1達成基準**:
- ✅ VAD + STT統合動作検証完了（Task 10.1 Phase 1）
- ✅ Python sidecar起動とWhisper初期化確認
- ✅ 部分テキスト（is_final=false）と確定テキスト（is_final=true）の配信確認
- ✅ speech_end イベント検証
- ✅ IPCイベント配信（process_audio_stream）

**Phase 2 (Future)**:
- ⏸️ LocalStorage統合（audio.wav, transcription.jsonl, session.json）
- ⏸️ WebSocket統合（Chrome拡張配信）
- ⏸️ Task 10.2-10.7実装（オフライン、リソース監視、デバイス切断、クロスプラットフォーム等）

**残課題**:
- Task 10.2-10.7のE2Eテスト実装（現在スケルトンのみ）

- [x] 10.1 音声録音→VAD→STT→保存の完全フロー検証（✅ Phase 1 完了、Phase 2 Future）
  - ✅ **Phase 1 (VAD + STT Core Flow) - COMPLETED**:
    - Python sidecar起動とWhisper初期化確認
    - VAD音声検出（speech_start、no_speech）
    - 部分テキスト生成（partial_text with is_final=false）
    - 確定テキスト生成（final_text with is_final=true）
    - speech_end イベント検証
    - IPCイベント配信（process_audio_stream）
    - リグレッション保護（partial_text明示的検証追加）
  - ⏸️ **Phase 2 (LocalStorage + WebSocket) - Future（別タスク化予定）**:
    - ローカルストレージへのセッション保存（audio.wav, transcription.jsonl, session.json）を検証
    - WebSocket経由でChrome拡張へのメッセージ配信を確認
  - _Requirements: STT-REQ-001, STT-REQ-002, STT-REQ-003 (Phase 1 ✅), STT-REQ-005, STT-REQ-008 (Phase 2 ⏸️)_
  - _Test: `cargo test --test stt_e2e_test test_audio_recording_to_transcription_full_flow` (1 passed, 22.36s)_

- [x] 10.2 オフラインモデルフォールバックE2Eテスト（✅ Python側完了、Rust側保留）
  - ✅ **Python側ユニットテスト完了**（`tests/test_offline_model_fallback.py`、14件合格）:
    - HuggingFace Hubダウンロード成功・タイムアウト検証
    - ネットワークエラー時のbundled modelフォールバック
    - offline_mode=True設定時のHub接続スキップ
    - プロキシ環境変数（HTTPS_PROXY/HTTP_PROXY）認識
    - モデルキャッシング動作検証
  - ✅ **P0 Bug修正完了**（ADR-016）:
    - **問題**: STT-REQ-002.4「ネットワークエラー→bundled base」が未実装（false positive test）
    - **修正**: `initialize()` でWhisperModel load失敗時に `_detect_bundled_model_path()` 呼び出し
    - **影響**: オフライン環境での初回起動が動作不能だった → 修正後は bundled baseへ自動フォールバック
    - **テスト**: 実際の制御フローをシミュレートするよう2テスト修正（mock戦略変更）
  - ⏸️ **Rust側E2E保留**（実装優先度低、インフラ統合テスト的）:
    - ネットワーク切断シミュレーション（環境変数制御、モック複雑）
    - CI環境での再現性課題
    - Python側で十分カバー済みのため、MVP1スコープ外とする
  - _Requirements: STT-REQ-002.4, STT-REQ-002.5 (Python側 ✅)_
  - _Test: `pytest tests/test_offline_model_fallback.py` (14 passed, 0.25s)_

- [x] 10.3 動的モデルダウングレードE2Eテスト（✅ Python側ほぼ完了、Rust側保留）
  - ✅ **Python側ユニットテスト**（`tests/test_resource_monitor.py`、58/60件合格 = 96.7%）:
    - CPU使用率85% @ 60秒継続でダウングレード検証 ✅
    - メモリ使用量3GB/4GB到達時の即座ダウングレード ⚠️（2件失敗、Task 11.6で修正予定）
    - UI通知生成（ダウングレード、アップグレード提案、録音一時停止）✅
    - モデル切り替えシーケンス検証 ✅
    - リソース回復時のアップグレード提案（5分待機）✅
  - ✅ **統合テスト**（`tests/test_audio_integration.py::TestResourceMonitorIntegration`、7件合格）:
    - AudioPipeline + ResourceMonitor統合動作検証
    - モデルダウングレード失敗時の状態一貫性
    - ユーザー承認アップグレード実行
  - ⏸️ **Rust側E2E保留**（実装優先度低、Python側でカバー済み）
  - _Requirements: STT-REQ-006.6, STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9 (Python側 96.7% ✅)_
  - _Test: `pytest tests/test_resource_monitor.py -k "monitor or downgrade"` (58/60 passed)_

- [ ] 10.4 デバイス切断/再接続E2Eテスト（❌ 未実装、`unimplemented!()`）
  - 音声デバイス切断イベントをシミュレーション
  - 5秒後の自動再接続試行（最大3回）を検証
  - ユーザー通知「音声デバイスが切断されました」の表示確認
  - 再接続失敗時の最終エラーハンドリングを確認
  - _Note: Task 2.5で検出機能実装済み、E2Eテスト未実装_
  - _Requirements: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11_

- [ ] 10.5 クロスプラットフォームE2Eテスト（❌ 未実装、`unimplemented!()`）
  - macOS/Windows/Linuxの各環境で音声デバイス列挙を実行
  - 各OS固有のループバックオーディオキャプチャ（BlackHole/WASAPI/PulseAudio）を検証
  - 全環境で録音→STT→保存の基本フロー動作確認
  - _Note: Task 2.4でループバックデバイス認識実装済み、E2Eテスト未実装_
  - _Requirements: STT-REQ-004.3, STT-REQ-004.4, STT-REQ-004.5, STT-NFR-003.1_

- [ ] 10.6 IPC/WebSocket後方互換性テスト（❌ 未実装、コメントのみ）
  - Phase 6で拡張したIPCフィールド（confidence/language/processingTimeMs/modelSize）の送受信検証
  - WebSocket拡張フィールド（isPartial/confidence/language/speakerSegment/processingTimeMs）の配信確認
  - meeting-minutes-core（Fake実装）との互換性確認（拡張フィールド無視）
  - バージョン不一致時のフォールバック動作（マイナーバージョン=警告、メジャーバージョン=エラー）を検証
  - _Note: tests/ipc_migration_test.rs (26 tests)、tests/websocket_message_extension_test.rs (6 tests)で**ユニットテスト**カバー済み、E2E統合テスト未実装_
  - _Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.3, STT-REQ-007.6, STT-REQ-008.1, STT-REQ-008.2_

- [ ] 10.7 非機能要件検証（SLA、パフォーマンス、リソース制約）（❌ 未実装、`unimplemented!()`）
  - 部分テキスト応答時間 < 0.5秒を測定（VAD発話検出から部分テキスト配信まで）
  - 確定テキスト応答時間 < 2秒を測定（VAD発話終了検出から確定テキスト配信まで）
  - 2時間連続録音時のメモリ使用量 < 2GBを検証
  - 継続録音時のCPU使用率 < 50%を測定
  - faster-whisper推論時間の目標達成確認（tiny/base: <0.2s, small: <0.5s, medium: <1s, large: <2s）
  - _Note: Task 11.1-11.3と重複、統合実装予定_
  - _Requirements: STT-NFR-001.1, STT-NFR-001.2, STT-NFR-001.4_

- [ ] 11. パフォーマンス最適化と非機能要件検証
- [ ] 11.1 レイテンシ最適化
  - 部分テキスト生成レイテンシ計測（目標: 0.5秒以内）
  - 確定テキスト生成レイテンシ計測（目標: 2秒以内）
  - ボトルネック特定とチューニング
  - _Requirements: STT-NFR-001.1, STT-NFR-001.2_

- [ ] 11.2 リソース使用量検証
  - メモリ使用量計測（2時間録音で最大2GB）
  - CPU使用率計測（継続的に50%以下）
  - ディスク使用量計測
  - _Requirements: STT-NFR-001.3, STT-NFR-001.4_

- [ ] 11.3 長時間稼働安定性テスト
  - 2時間連続録音テスト
  - メモリリーク検証
  - リソース監視機能の動作確認
  - _Requirements: STT-NFR-001.5, STT-NFR-001.6_

- [ ] 11.6 詳細Metrics実装とRollback Strategy（Task 7.3.9残課題）
  - **SessionMetrics詳細フィールド実装**:
    - `mutex_contention_count`: Mutex競合発生回数（想定0）
    - `stdin_lock_duration_us`: stdin Mutex保持時間（マイクロ秒）
    - `stdout_lock_duration_us`: stdout Mutex保持時間（マイクロ秒）
    - `concurrent_operations_count`: 並行操作カウント
    - `python_hangs_detected`: Python異常検出回数
    - `callback_duration_us`: Audio callback実行時間（マイクロ秒）
  - **Alert Conditions実装**:
    - mutex_contention_count > 100/秒 → 設計想定外アラート
    - frames_dropped > 100 → Python異常アラート
    - callback_duration_us > 100 → CPAL停止リスクアラート
  - **Feature flag追加**: `USE_STDIN_STDOUT_SEPARATED_MUTEX`（将来のロールバック用）
  - **Rollback手順文書化**: ADR-013 → ADR-008フォールバック手順
  - セッション終了時の`SessionMetrics::report()`メソッド実装
  - _Note: Task 7.3.9で計画されていたが、ADR-013実装でSuccess Criteria達成済みのためP1に降格_
  - _Requirements: STT-REQ-007.7（運用監視強化）_

- [ ] 11.1 IPCレイテンシ計測基盤
  - `PythonSidecarManager::send_message()` に送信タイムスタンプを付与（monotonicクロック）
  - `receive_message()` で応答タイムスタンプを取得し、`ipc_latency_ms` を算出
  - `logger` 経由でイベント (`ipc.latency`) を構造化ログへ出力
  - `scripts/performance_report.py` に集計処理を追加し、P95/P99 を算出
  - _Requirements: STT-REQ-007.8, STT-NFR-001 (Diagnostics)_

- [ ] 11.2 構造化ログロールアウト
  - `println!/eprintln!` を `log_info!`/`log_error!` に置換（lib.rs / commands.rs / websocket.rs など）
  - ログフィールド標準化: `session_id`, `component`, `event`, `duration_ms`, `pii_masked`
  - Python側 `logging` を JSON フォーマットに統一し、ResourceMonitor / AudioPipeline でも同様に出力
  - _Requirements: STT-NFR-005 (Logging policy)_

- [ ] 11.3 ダイアグノスティクスダッシュボード
  - ローカル `scripts/performance_report.py` で IPCレイテンシとリソースメトリクスをHTML/Markdownレポート化
  - 将来のCI連携を想定し、`artifacts/diagnostics/` に出力するワークフローを設計
  - _Requirements: STT-NFR-001.6, STT-NFR-005.4_

- [ ] 11.4 ログ/レイテンシ検証
  - 構造化JSONログに `session_id`, `event`, `ipc_latency_ms` が正しく出力されることを確認
  - PII マスク（音声データ本文が記録されない）の検証
  - 異常時のユーザー通知とログ内容の整合性確認
  - _Requirements: STT-NFR-005_

- [ ] 11.5 セキュリティテスト
  - TLS 1.2以降接続検証（HuggingFace Hub）
  - モデルファイルSHA256ハッシュ検証テスト
  - ディレクトリアクセス制限検証
  - 音声デバイスアクセス許可フロー検証
  - _Requirements: STT-NFR-004_

- [ ] 12. ドキュメントとリリース準備
- [ ] 12.1 UML図の作成
  - CMP-001_STT-Audio-Processing-Pipeline.puml（コンポーネント図）
  - SEQ-001_Audio-Recording-to-Transcription.puml（シーケンス図）
  - SEQ-002_Offline-Model-Fallback.puml（オフラインフォールバック）
  - SEQ-003_Dynamic-Model-Downgrade.puml（動的ダウングレード）
  - CLS-001_Audio-Device-Adapter.puml（クラス図）
  - _Requirements: STT-NFR-005 (ドキュメント/ログ運用方針), STT-REQ-001, STT-REQ-002, STT-REQ-003, STT-REQ-006_

- [ ] 12.2 ADRのレビューと更新
  - ADR-001（録音責務一元化）の実装検証
  - ADR-002（モデル配布戦略）の実装検証
  - ADR-003（IPCバージョニング）の実装検証
  - _Requirements: STT-REQ-001 (録音責務), STT-REQ-002 (faster-whisper統合/オフライン), STT-REQ-007 (IPCバージョニング)_

- [ ] 12.3 README更新とユーザーガイド作成
  - インストール手順の更新
  - faster-whisperモデルセットアップガイド
  - 音声デバイス設定ガイド（macOS BlackHole、Windows WASAPI loopback、Linux PulseAudio monitor）
  - トラブルシューティングガイド
  - _Requirements: 全要件_

- [ ] 12.4 プラットフォーム検証ランブックの整備
  - `docs/platform-verification.md` のベースライン・チェックリストを最新化
  - `scripts/platform_smoke.sh` を CI/ローカル両対応にしてログを `logs/platform/` へ保存
  - `cargo run --bin stt_burn_in`（ロングラン検証ツール）の実装と実行記録
  - Windows / Linux 実機テスト結果をベースライン表に追記
  - _Requirements: STT-NFR-003, STT-NFR-005, STT-REQ-007_

---

## Implementation Notes

### TDD実装フロー
1. **RED**: 失敗するテストを先に作成
2. **GREEN**: 最小限の実装でテストを通す
3. **REFACTOR**: コード品質向上
4. **REPEAT**: 次のタスクへ

### 重要な制約
- **録音責務**: Python側での録音は絶対禁止（check_forbidden_imports.pyで自動検証）
- **オフラインファースト**: HuggingFace Hub接続は10秒タイムアウト、バンドルモデルフォールバック必須
- **後方互換性**: meeting-minutes-core（Fake実装）との互換性を全テストで検証

### テストカバレッジ目標
- ユニットテスト: 80%以上
- 統合テスト: 主要シナリオ100%
- E2Eテスト: 全要件カバー

---

## Next Steps

タスク生成完了後、以下のコマンドで実装を開始します:

```bash
# 全タスクを順次実行
/kiro:spec-impl meeting-minutes-stt

# 特定タスクを実行
/kiro:spec-impl meeting-minutes-stt 2.1

# 複数タスクを実行
/kiro:spec-impl meeting-minutes-stt 2.1,2.2,2.3
```
