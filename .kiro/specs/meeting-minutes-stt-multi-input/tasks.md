# Implementation Plan

## meeting-minutes-stt-multi-input

複数入力デバイス（マイク＋ループバック）の同時録音とアプリ内ミックス機能の実装タスク。

---

- [x] 1. 基盤構造とFacadeパターンの実装
- [x] 1.1 AudioDeviceRecorder Facade の骨格実装
  - 単一入力と複数入力を切り替える統一インターフェースを作成
  - RecordingMode 列挙型（Single / Multi）を定義
  - AdapterFactory パターンで複数アダプタインスタンス生成に対応
  - 単一入力モードでは既存動作を完全に維持することを検証
  - _Requirements: STTMIX-CON-001_ (後方互換の骨格のみ、並列キャプチャは Task 2.x)

- [x] 1.2 AppState の拡張
  - 複数デバイスID保持用のフィールドを追加
  - multi_input_enabled フラグを追加
  - AudioDeviceRecorder への参照を状態管理に組み込み
  - 既存の単一デバイス選択との後方互換性を維持
  - _Requirements: STTMIX-REQ-001.2_

- [x] 1.3 Tauri コマンド層の拡張
  - start_recording_multi コマンドを追加（複数デバイスID対応）
  - 単一ID指定時は既存動作を維持（後方互換）
  - 複数ID指定時の入力バリデーション（最大2入力、空配列チェック）
  - フロントエンドから呼び出し可能なインターフェースを確保
  - _Requirements: STTMIX-CON-001, STTMIX-CON-005_ (APIの骨格のみ、実際の並列キャプチャは Task 2.x)

---

- [x] 2. 複数入力の並列キャプチャ機能
- [x] 2.1 MultiInputManager の実装
  - 複数デバイスのライフサイクル管理機能を構築
  - 各入力に対する役割（Microphone / Loopback）とゲイン設定を保持
  - 録音開始時に全デバイスの入力ストリームを並行して開始
  - 録音停止時に全ストリームを確実に停止しリソースを解放
  - _Requirements: STTMIX-REQ-002.1, STTMIX-REQ-002.3_

- [x] 2.2 InputCapture（入力ごとのキャプチャ）実装
  - cpal input stream から f32 PCM を取得する処理を実装
  - 入力ごとの per-input buffer（リングバッファ）を作成
  - ネイティブサンプルレートでのデータ受信とバッファ投入
  - 各入力のコールバック処理でスレッドセーフなバッファ書き込み
  - _Requirements: STTMIX-REQ-002.1, STTMIX-REQ-003.3_

- [x] 2.3 部分失敗時の継続動作
  - いずれかのデバイス開始失敗時のエラー通知と判定ロジック
  - 残りのデバイスで継続可能かの判定（設定による制御）
  - 失敗理由のログ記録とUI通知イベント発行
  - _Requirements: STTMIX-REQ-002.2, STTMIX-REQ-006.1_

---

- [x] 3. リサンプリングとダウンミックス処理
- [x] 3.1 入力ごとのリサンプリング実装
  - 各入力のネイティブサンプルレートから16kHzへの変換処理
  - 既存の平均化ダウンサンプリング手法を再利用（N サンプル平均、簡易ローパスフィルタ）
  - ステレオ入力をモノラルへダウンミックス（L/R平均）
  - 正規化完了後に10msフレーム単位で後段へ渡す
  - _Requirements: STTMIX-REQ-003.1, STTMIX-REQ-003.2, STTMIX-REQ-003.3_

---

- [x] 4. Input Mixer（時間整列とミックス）の実装
- [x] 4.1 10ms フレーム境界での時間整列
  - Mixer が10ms cadence でフレームを生成する仕組みを構築
  - 各入力から10ms分のサンプルを取り出すロジック
  - サンプルが揃っていない場合の短期待機または無音補完
  - _Requirements: STTMIX-REQ-004.1_

- [x] 4.2 ドリフト補正の実装
  - 入力間のサンプル数差を監視する機構
  - ±10サンプル（±0.625ms）のしきい値超過時に補正実行
  - 1サンプルの間引き（遅れ入力）または複製（進み入力）
  - 100msあたり最大1回の補正頻度制限
  - 補正回数のメトリクス記録
  - _Requirements: STTMIX-REQ-004.2, STTMIX-REQ-008.2_

- [x] 4.3 ミックスアルゴリズムの実装
  - 各入力フレームを f32 に変換
  - 入力ごとにゲインを適用（デフォルト -6dB）
  - すべての入力を加算
  - [-1.0, 1.0] にクランプし i16 PCM へ変換
  - ミックス済みフレームを既存リングバッファへ投入
  - _Requirements: STTMIX-REQ-004.3, STTMIX-REQ-005.1, STTMIX-REQ-005.2_

- [x] 4.4 クリッピング検出と抑制
  - ミックス後の振幅が上限を超えた場合の検出
  - クランプによる歪み抑制
  - クリップ発生回数のメトリクス記録
  - 将来的な簡易リミッタ導入の拡張ポイント確保
  - _Requirements: STTMIX-REQ-005.3, STTMIX-REQ-008.2_

---

- [x] 5. IPC互換性とバッチ送信の統合
- [x] 5.1 既存パイプラインへの統合
  - ミックス済みフレームを既存の5秒リングバッファへ投入
  - 既存のBatch Sender（250ms周期）との連携確認
  - process_audio_stream IPC フォーマットが変更されていないことを検証
  - Python側が16kHz mono 16-bit PCMを受信することを確認
  - _Requirements: STTMIX-REQ-007.1, STTMIX-REQ-007.2, STTMIX-CON-002_

---

- [x] 6. エラーハンドリングと段階的縮退
- [x] 6.1 単一入力喪失時の継続動作
  - デバイス切断イベントの検出と通知
  - 残存入力のみで録音を継続する機能
  - 設定による停止/継続の切り替え（degradation_policy）
  - _Requirements: STTMIX-REQ-006.1, STTMIX-REQ-006.2_

- [x] 6.2 全入力喪失時の停止処理
  - 全入力が失われた場合の録音停止
  - エラー通知のUI表示
  - リソースの確実な解放
  - _Requirements: STTMIX-REQ-006.3_

- [x] 6.3 バッファ枯渇時の無音補完
  - 入力バッファが空の場合の無音フレーム生成
  - フレーム欠損のメトリクス記録
  - NFR（フレーム欠損率 ≤ 0.1%）の検証準備
  - _Requirements: STTMIX-NFR-Rel-001_

---

- [x] 7. 設定保存と復元機能
- [x] 7.1 マルチ入力設定の永続化
  - selected_device_ids の保存と復元
  - input_roles（Microphone/Loopback）の保存
  - gains（入力ごとのゲイン値）の保存
  - mute 状態の保存
  - degradation_policy の保存
  - _Requirements: STTMIX-REQ-001.2, STTMIX-REQ-005.1_

- [x] 7.2 起動時の設定復元
  - アプリ起動時に保存された設定を読み込み
  - 選択されたデバイスが利用可能かの確認
  - 利用不可デバイスの警告表示
  - _Requirements: STTMIX-REQ-001.2_

---

- [x] 8. UI拡張（複数デバイス選択）
- [x] 8.1 デバイス選択UIの複数選択対応
  - デバイス一覧で複数選択を可能にするUI変更
  - ループバックデバイスの視覚的識別（アイコン/ラベル）
  - 選択されたデバイスの役割表示
  - **最大2入力の選択制限バリデーション**（3入力以上は選択不可）
  - _Requirements: STTMIX-REQ-001.1, STTMIX-REQ-001.3, STTMIX-CON-005_

- [x] 8.2 OS判定と機能ゲート
  - macOS以外の環境ではマルチ入力機能を非表示または無効化
  - Windows/Linux環境では「macOSのみ対応」の説明を表示
  - OS判定ロジックをRust側で実装しフロントエンドへ通知
  - _Requirements: STTMIX-CON-004_

- [x] 8.3 入力状態の表示
  - 各入力のバッファ占有率の視覚的表示
  - 入力喪失時の警告表示
  - 録音中のミックス状態インジケーター
  - _Requirements: STTMIX-REQ-006.1, STTMIX-REQ-008.1_

---

- [x] 9. 監視機能とメトリクス収集
- [x] 9.1 入力ごとのメトリクス収集
  - input_buffer_level{device} の計測
  - drift_correction_count の記録
  - clip_count の記録
  - mix_latency_ms の計測
  - _Requirements: STTMIX-REQ-008.1, STTMIX-REQ-008.2_

- [x] 9.2 メトリクス公開とログ出力
  - 構造化ログへのメトリクス出力
  - デバッグ用の詳細ログモード
  - パフォーマンス監視用のサマリーログ
  - _Requirements: STTMIX-REQ-008_

---

- [x] 10. テスト実装
- [x] 10.1 ユニットテスト
  - Mixerアルゴリズムのテスト（ゲイン・クリップ・ドリフト補正）
  - リサンプリング精度のテスト
  - バッファ管理のエッジケーステスト
  - _Requirements: All functional requirements_

- [x] 10.2 統合テスト
  - 2入力の同時キャプチャ + ミックス → 16kHz mono出力
  - 部分失敗シナリオのテスト
  - 設定保存/復元のテスト
  - _Requirements: STTMIX-REQ-002, STTMIX-REQ-006_

- [x] 10.3 リグレッションテスト
  - 既存の単一入力動作が維持されることの確認
  - IPC互換性の検証
  - 既存テストスイートの全パス確認
  - _Requirements: STTMIX-REQ-007, STTMIX-CON-002_

- [x] 10.4 NFR検証テスト
  - レイテンシ計測（p95 ≤ 20ms）
  - CPU使用率計測（+5%以内）
  - フレーム欠損率計測（≤ 0.1%）
  - _Requirements: STTMIX-NFR-Perf-001, STTMIX-NFR-Perf-002, STTMIX-NFR-Rel-001_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| STTMIX-REQ-001 | 1.2, 7.1, 7.2, 8.1 |
| STTMIX-REQ-002 | 2.1, 2.2, 2.3 |
| STTMIX-REQ-003 | 3.1, 2.2 |
| STTMIX-REQ-004 | 4.1, 4.2, 4.3 |
| STTMIX-REQ-005 | 4.3, 4.4, 7.1 |
| STTMIX-REQ-006 | 2.3, 6.1, 6.2, 6.3 |
| STTMIX-REQ-007 | 5.1 |
| STTMIX-REQ-008 | 4.2, 4.4, 8.3, 9.1, 9.2 |
| STTMIX-NFR-Perf-001 | 10.4 |
| STTMIX-NFR-Perf-002 | 10.4 |
| STTMIX-NFR-Rel-001 | 6.3, 10.4 |
| STTMIX-CON-001 | 1.1, 1.3 |
| STTMIX-CON-002 | 5.1, 10.3 |
| STTMIX-CON-004 | 8.2 |
| STTMIX-CON-005 | 1.3, 2.1, 8.1 |

**注記**: Task 1.x は骨格実装（後方互換性確保）のみ。並列キャプチャ要件 (STTMIX-REQ-002) の完全な実装は Task 2.x で達成。

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-09 | 1.0 | Claude | 初版（10メジャータスク、28サブタスク） |
| 2026-01-09 | 1.1 | Claude | レビュー指摘対応: 1.3（コマンド層拡張）、8.2（OS判定）追加、8.1に最大2入力制限追加 |
| 2026-01-09 | 1.2 | Claude | 構造修正: Task 1.xの要件マッピング修正（骨格のみ明記）、Task 3.1「線形補間」→「平均化」修正 |
