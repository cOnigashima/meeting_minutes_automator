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

- [ ] 2. 実音声デバイス管理機能の実装（Rust側）
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

- [ ] 3. faster-whisper統合とモデル管理機能（Python側）
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

- [ ] 4. 音声活動検出（VAD）機能の実装（Python側）
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

- [x] 4.3 部分テキストと確定テキストの生成連携
  - AudioPipelineのmain.py統合完了（AudioProcessorクラス作成）
  - IpcHandlerのmain.py統合完了（VAD/STTイベントのIPC配信）
  - VAD→AudioPipeline→STT本番経路構築完了（_handle_process_audio実装）
  - 実統合テスト追加（test_audio_integration.py: 5 passed, 1 skipped）
  - 全テスト: 101 passed, 1 skipped
  - _Note: test_partial_text_generation_during_speechはtime.time()モッキング未実装のためskipped_
  - _Requirements: STT-REQ-003.6, STT-REQ-003.7, STT-REQ-003.8, STT-REQ-003.9_

- [ ] 5. リソース監視と動的モデルダウングレード機能（Python側）
- [ ] 5.1 ResourceMonitorスケルトンと監視ループ
  - 失敗するユニットテストを作成（リソース監視、ダウングレード判定）
  - ResourceMonitorクラスの定義
  - 30秒間隔のリソース監視ループ（CPU、メモリ）
  - リソース使用量取得機能
  - _Requirements: STT-REQ-006.6, STT-REQ-006.7, STT-REQ-006.8_

- [ ] 5.2 動的モデルダウングレード機能
  - CPU 85%/60秒持続時の1段階ダウングレード判定
  - メモリ4GB到達時の即座baseモデルダウングレード判定
  - 音声セグメント境界での切り替えロジック（シームレス切り替え）
  - ダウングレード順序の実装（large→medium→small→base→tiny）
  - モデル切り替え履歴のログ記録
  - ユニットテストと統合テストの緑化
  - _Requirements: STT-REQ-006.6, STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9_

- [ ] 5.3 UI通知とアップグレード提案機能
  - トースト通知の送信機能（「モデル変更: {old}→{new}」）
  - リソース回復検出ロジック（メモリ2GB未満 AND CPU 50%未満が5分継続）
  - モデルアップグレード提案UI通知
  - ユーザー承認時のアップグレード試行
  - tinyモデルでもリソース不足時の録音一時停止
  - _Requirements: STT-REQ-006.9, STT-REQ-006.10, STT-REQ-006.11, STT-REQ-006.12_

- [ ] 6. ローカルストレージ機能の実装（Rust側）
- [ ] 6.1 LocalStorageServiceスケルトンとセッション管理
  - 失敗するユニットテストを作成（セッション作成、保存、読み込み）
  - LocalStorageServiceクラスの定義
  - セッションID生成機能（UUID）
  - セッションディレクトリ作成機能（[app_data_dir]/recordings/[session_id]/）
  - _Requirements: STT-REQ-005.1_

- [ ] 6.2 音声ファイル保存機能
  - 音声データのWAVファイル保存（16kHz、モノラル、16bit PCM）
  - リアルタイムストリーミング書き込み
  - ファイルクローズ処理
  - ユニットテストの緑化
  - _Requirements: STT-REQ-005.2_

- [ ] 6.3 文字起こし結果保存機能
  - 部分テキストと確定テキストのJSON Lines形式保存（transcription.jsonl）
  - 追記モードでのファイル書き込み
  - タイムスタンプとis_finalフラグの記録
  - ユニットテストの緑化
  - _Requirements: STT-REQ-005.3_

- [ ] 6.4 セッションメタデータ保存機能
  - session.json保存機能（session_id、start_time、end_time、duration_seconds等）
  - セッション統計情報の集計（total_segments、total_characters）
  - 録音終了時のメタデータ書き込み
  - ユニットテストの緑化
  - _Requirements: STT-REQ-005.4_

- [ ] 6.5 セッション一覧取得と再生機能
  - セッションメタデータ一覧読み込み機能
  - 日時降順ソート機能
  - 過去セッションの読み込み機能（session.json、transcription.jsonl、audio.wav）
  - 再生・表示UI連携
  - 統合テストの緑化
  - _Requirements: STT-REQ-005.5, STT-REQ-005.6_

- [ ] 6.6 ディスク容量監視と警告機能
  - ディスク容量監視ロジック
  - 1GB未満時の警告ログと通知
  - 500MB未満時の録音開始拒否
  - エラーメッセージ表示
  - _Requirements: STT-REQ-005.7, STT-REQ-005.8_

- [ ] 7. IPC通信プロトコル拡張と後方互換性（Rust/Python両側）
- [ ] 7.1 IPCメッセージ拡張とバージョニング
  - 失敗するユニットテストを作成（メッセージシリアライゼーション、バージョンチェック）
  - 新フィールドの追加（confidence、language、processing_time_ms、model_size）
  - バージョンフィールドの追加（"version": "1.0"）
  - 既存メッセージ形式の維持（text、is_final）
  - ユニットテストの緑化
  - _Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.4, STT-REQ-SEC-001_

- [ ] 7.2 後方互換性テストとエラー処理
  - meeting-minutes-core（Fake実装）との互換性テスト
  - 未知フィールドの無視動作確認
  - エラー応答フォーマットの実装（errorCode、errorMessage、recoverable）
  - バージョン不一致時の処理（メジャー拒否、マイナー警告、パッチ情報のみ）
  - 統合テストの緑化
  - _Requirements: STT-REQ-007.3, STT-REQ-007.5, STT-REQ-007.6, STT-REQ-IPC-004, STT-REQ-IPC-005_

- [ ] 8. WebSocketメッセージ拡張（Rust側）
- [ ] 8.1 WebSocketメッセージ拡張とChrome拡張連携
  - 失敗するユニットテストを作成（メッセージ配信、フィールド検証）
  - 拡張メッセージ形式の実装（confidence、language、processing_time_ms追加）
  - Chrome拡張への配信機能
  - 未知フィールド無視の検証
  - ユニットテストと統合テストの緑化
  - _Requirements: STT-REQ-008.1_

- [ ] 9. UI拡張とユーザー設定機能（Rust/React）
- [ ] 9.1 音声デバイス選択UI
  - 失敗するE2Eテストを作成（デバイス選択フロー）
  - デバイス一覧表示コンポーネント
  - デバイス選択ドロップダウン
  - 選択デバイスの設定保存機能
  - E2Eテストの緑化
  - _Requirements: STT-REQ-001.3_

- [ ] 9.2 Whisperモデル選択UI
  - モデルサイズ選択ドロップダウン（tiny、base、small、medium、large-v3）
  - 自動選択とカスタマイズのトグル
  - リソース超過警告の表示
  - E2Eテストの緑化
  - _Requirements: STT-REQ-006.4_

- [ ] 9.3 オフラインモード設定UI
  - オフラインモード強制のチェックボックス
  - バンドルモデル使用状態の表示
  - 設定保存機能
  - _Requirements: STT-REQ-002.6_

- [ ] 9.4 リソース監視とモデル切り替え通知UI
  - トースト通知コンポーネント
  - モデル切り替え通知の表示（「モデル変更: {old}→{new}」）
  - モデルアップグレード提案ダイアログ
  - _Requirements: STT-REQ-006.9, STT-REQ-006.10_

- [ ] 9.5 セッション管理UI
  - セッション一覧表示コンポーネント
  - セッション詳細表示（メタデータ、文字起こし結果）
  - 音声再生機能
  - セッション削除機能
  - _Requirements: STT-REQ-005.5, STT-REQ-005.6_

- [ ] 9.6 実装とのギャップ
  - /kiro:validate-gap　を行う


- [ ] 10. 統合とE2Eテスト
- [ ] 10.1 音声録音→VAD→STT→保存の完全フロー統合テスト
  - 失敗するE2Eテストを作成（全体フロー）
  - RealAudioDevice→AudioStreamBridge→PythonSidecar→VAD→WhisperSTTEngine→LocalStorageServiceの統合
  - 部分テキストと確定テキストの配信検証
  - ローカルストレージへの保存検証
  - E2Eテストの緑化
  - _Requirements: STT-REQ-001, STT-REQ-002, STT-REQ-003, STT-REQ-005_

- [ ] 10.2 オフラインモデルフォールバックE2Eテスト
  - ネットワーク切断シミュレーション
  - バンドルbaseモデルへのフォールバック検証
  - オフライン起動から文字起こし完了までのフロー検証
  - _Requirements: STT-REQ-002.4, STT-REQ-002.5, STT-REQ-002.6_

- [ ] 10.3 動的モデルダウングレードE2Eテスト
  - CPU/メモリ負荷シミュレーション
  - リアルタイムモデルダウングレード検証
  - 音声セグメント境界でのシームレス切り替え検証
  - UI通知の表示確認
  - _Requirements: STT-REQ-006.6, STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9_

- [ ] 10.4 デバイス切断/再接続E2Eテスト
  - デバイス切断シミュレーション
  - 自動再接続フロー検証（最大3回）
  - ユーザー通知の表示確認
  - _Requirements: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11_

- [ ] 10.5 クロスプラットフォームE2Eテスト
  - macOS、Windows、Linux各環境での音声録音検証
  - ループバックオーディオキャプチャ検証
  - OS固有API統合の動作確認
  - _Requirements: STT-REQ-004.3, STT-REQ-004.4, STT-REQ-004.5, STT-REQ-004.6, STT-REQ-004.7, STT-REQ-004.8_

- [ ] 10.6 IPC/WebSocket後方互換性E2Eテスト
  - meeting-minutes-core（Fake実装）との互換性検証
  - 拡張フィールド追加後の動作確認
  - Chrome拡張での表示確認
  - _Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.3, STT-REQ-008.1, STT-REQ-E2E-001_

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

- [ ] 11.4 ログ出力とエラーハンドリング検証
  - 構造化JSONログ出力確認（session_id、component、event、duration_ms）
  - PIIマスク検証（音声データバイナリ内容の非記録）
  - エラー分類とユーザー提示の適切性確認
  - _Requirements: STT-NFR-005_

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
