## Testing Strategy

### Unit Tests

**音声処理コアモジュール**:

1. **RealAudioDevice**:
   - デバイス検出: OS固有API経由でデバイス列挙
   - 音声フォーマット変換: 16kHz mono PCM変換精度
   - エラーハンドリング: デバイス切断検出、再接続ロジック

2. **VoiceActivityDetector**:
   - VAD精度検証: Golden Audioサンプルでの発話/無音判定精度
   - セグメンテーション品質: 発話開始 (0.3秒) / 終了 (0.5秒) 閾値検証
   - エッジケース: ノイズ環境、低音量音声、高速発話

3. **WhisperSTTEngine**:
   - モックオーディオでの認識精度: 既知テキストとの一致率
   - レスポンス時間: 処理時間目標検証 (tiny 0.2s, small 0.5s, medium 1s, large 2s)
   - オフラインフォールバック: HuggingFace Hubタイムアウト時のバンドルモデル使用

4. **ResourceMonitor**:
   - モデル選択ロジック: リソース条件ごとの正しいモデル選択
   - ダウングレードトリガー: CPU 85%/60秒、メモリ 4GB閾値検証
   - アップグレード提案: リソース回復条件 (メモリ2GB未満 AND CPU 50%未満が5分継続)

5. **LocalStorageService**:
   - ファイル保存: audio.wav, transcription.jsonl, session.json の正しい形式
   - ディスク容量監視: 1GB警告、500MB制限閾値検証
   - セッションリスト取得: 日時降順ソートの正確性

**テストフレームワーク**:
- Rust: `cargo test` + `cargo nextest`
- Python: `pytest --asyncio`

---

### Integration Tests

**跨コンポーネント連携**:

1. **音声キャプチャ→VAD→STT→保存のエンドツーエンドパイプライン**:
   - Golden Audioサンプルを使用し、録音開始→VAD→STT→ローカル保存までの完全フロー検証
   - 部分テキスト/確定テキストの正しい生成とWebSocket配信

2. **IPC通信の信頼性とメッセージ順序保証**:
   - Rust→Python間のstdin/stdout JSON通信の正確性
   - 音声チャンク送信順序の保証
   - エラー応答の正しい処理

3. **オフラインモデルフォールバック統合テスト**:
   - HuggingFace Hub接続をモック化し、タイムアウト発生時のバンドルモデルフォールバック検証
   - プロキシ環境 (HTTPS_PROXY設定) での動作検証

4. **動的モデルダウングレード統合テスト**:
   - CPU/メモリ使用量をシミュレートし、ダウングレードトリガー検証
   - 進行中セグメント処理の継続性確認

5. **エラー発生時の回復フローとデータ整合性**:
   - デバイス切断→自動再接続シナリオ
   - ディスク容量不足時の安全な録音停止

**テストフレームワーク**:
- Rust: `cargo test --test integration`
- Python: `pytest tests/test_integration.py`

---

### E2E/UI Tests

**重要ユーザーパス**:

1. **新規音声セッション開始から議事録生成までの完全フロー**:
   - ユーザーが録音開始ボタンをクリック
   - RealAudioDeviceが音声ストリーム開始
   - VADが発話セグメントを検出
   - WhisperSTTEngineが文字起こし生成
   - LocalStorageServiceがtranscription.jsonlに保存
   - Chrome拡張コンソールに確定テキスト表示

2. **クロスプラットフォーム音声デバイステスト**:
   - macOS: Core Audio経由でマイク録音、BlackHoleループバックデバイステスト
   - Windows: WASAPI経由でマイク録音、WASAPI loopbackテスト
   - Linux: ALSA/PulseAudio経由でマイク録音、PulseAudio monitorテスト

3. **オフライン動作検証**:
   - ネットワーク接続を切断し、バンドルbaseモデルで文字起こし実行
   - オフラインモード強制設定での動作確認

4. **動的モデルダウングレードシナリオ**:
   - 長時間録音 (2時間以上) でのリソース監視とダウングレード動作
   - UI通知「モデル変更: small → base」の表示確認

5. **デバイス切断/再接続シナリオ**:
   - 録音中にマイクを物理的に切断
   - 自動再接続の成功確認 (5秒間隔、最大3回)

**テストフレームワーク**:
- E2E: Playwright (Chrome拡張連携テスト)
- UI: Tauri UIテスト (Vitest + @testing-library/react)

---

### Performance/Load Tests

**パフォーマンス検証**:

1. **音声処理遅延の測定**:
   - 目標: 部分テキスト0.5秒、確定テキスト2秒 (STT-NFR-001.1)
   - 実測: 各モデルサイズ (tiny/base/small/medium/large) でのレスポンス時間計測

2. **メモリ使用量の時間経過による増加率測定**:
   - 2時間録音でのメモリ使用量推移
   - 目標: 選択モデルサイズに応じた上限を超えない (tiny/base: 500MB、small: 1GB、medium: 2GB、large: 4GB) (STT-NFR-001.4)

3. **VAD処理のリアルタイム性能**:
   - 10msフレームごとの判定を1ms以内に完了 (STT-NFR-001.3)

4. **リソース監視オーバーヘッド**:
   - 30秒間隔でのCPU/メモリ監視によるオーバーヘッドを2%以内に抑える (STT-NFR-001.6)

5. **モデル切り替え時間**:
   - 動的ダウングレード実行時のモデル切り替えを3秒以内に完了 (STT-NFR-001.5)

**テストツール**:
- Rust: `cargo bench` (criterion)
- Python: `pytest-benchmark`
- メモリプロファイリング: `valgrind` (Linux), `Instruments` (macOS)

---

### CI/CD Integration Tests

**IPC互換性テスト**:
1. **バージョンネゴシエーションテスト**:
   - Rust 1.0 ↔ Python 1.0: 正常動作
   - Rust 1.0 ↔ Python 1.1: 正常動作 (下位互換性)
   - Rust 1.0 ↔ Python 2.0: エラー (メジャーバージョン不一致)
   - Rust 1.0 ↔ Python (versionフィールドなし): 正常動作 (デフォルト "1.0" 仮定)

2. **Schema検証テスト**:
   - JSON Schema検証 (各メッセージ型)
   - 必須フィールド欠落時のエラーハンドリング
   - 未知フィールド追加時の無視挙動 (forward compatibility)

3. **Python録音禁止テスト**:
   - 静的解析 (flake8-forbidden-imports)
   - 禁止パッケージ検出 (`sounddevice`, `pyaudio` 等)
   - pre-commit フック実行

**テストツール**:
- Rust: `cargo test --test integration`
- Python: `pytest tests/test_integration.py`
- CI/CD: GitHub Actions / GitLab CI

---

