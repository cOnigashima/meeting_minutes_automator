# Rust E2E Testing Constraints

Task 10.3（Dynamic Model Downgrade E2E）の技術的制約とテスト戦略を記載。

## 技術的制約

### 1. 実リソース圧迫シミュレーションは不可

**理由**: CIでのメモリ/CPU圧迫は危険（OOMキラー、タイムアウト）、WhisperモデルGB級ダウンロードで破綻

**対応**: `TEST_FIXTURE_MODE`導入（スクリプト化イベント送信、Whisperロード回避）

### 2. IPC経由での内部状態取得APIは追加しない

**理由**: IPC表面積最小化（セキュリティ）、単一責任原則、テストのための設計妥協を回避

**対応**: 状態マシン遷移検証はPython単体テストで完結、Rust E2EはIPC event path検証に専念

### 3. Tauri統合テスト（WebSocket → Chrome拡張）は手動検証

**理由**: Xvfb仮想ディスプレイ必要、Chrome拡張ロード複雑、CIコストと安定性トレードオフ

**対応**: Rust単体でWebSocket schema検証、実UI通知は手動検証

## カバレッジ戦略（3層）

| テスト層 | スコープ | ファイル | カバー項目 |
|---------|---------|---------|-----------|
| **Python単体** | リソース監視ロジック | `python-stt/tests/test_resource_monitor.py` | CPU/メモリ閾値、Debounce（60秒）、状態マシン、回復カウンタ、5レベルモデル選択 |
| **Rust単体** | WebSocket broadcast | `src-tauri/src/commands.rs::tests` | model_changeイベントschema検証、必須フィールド |
| **Rust E2E** | IPC event path | `src-tauri/tests/dynamic_model_downgrade_e2e.rs` | Python → Rust IPC通信、TEST_FIXTURE_MODEイベント受信、schema準拠 |
| **手動検証** | Tauri統合 | _(未作成)_ | WebSocket → Chrome拡張通知、UI更新 |

## TEST_FIXTURE_MODE 仕様

### 目的
- CIでのWhisperモデルロード回避（GB級ダウンロード防止）
- 決定論的IPCイベント送信（リソース圧迫不要）

### 使用方法（RAII Guard Pattern）

```rust
// ❌ WRONG: グローバル状態汚染、他テストに影響
std::env::set_var("TEST_FIXTURE_MODE", "1");

// ✅ CORRECT: RAII guardで自動クリーンアップ
let _guard = TestFixtureModeGuard::new();
// Guard自動でTEST_FIXTURE_MODEを削除/復元（panic時も安全）
```

### Critical Issue（外部レビュー対応3、2025-10-20）

**問題**: `std::env::set_var`はプロセス全体の環境変数を変更、クリーンアップなしで他テスト汚染

**影響**: 実Whisper期待のテスト（stt_e2e_test.rs）がfixture modeで動作、`AudioProcessor.stt_engine = None`でクラッシュ

**修正**:
- `TestFixtureModeGuard` RAII pattern実装（panic時も安全）
- 前の値を保存、Dropで復元
- `#[serial(env_test)]`属性追加（同一ファイル内race condition防止）

### 並列実行の制約

- Rustテストハーネスは**ファイル単位**でプロセス分離
- `dynamic_model_downgrade_e2e.rs`と`stt_e2e_test.rs`は別プロセス（環境変数共有なし）
- **同一ファイル内**のテストは並列実行される（race condition発生）
- `serial_test` crateで`#[serial(env_test)]`付与し、環境変数を触るテストを直列化
- `RUST_TEST_THREADS=1`は**不要**（過剰制約、別ファイルには影響しない）

### Python sidecar動作（main.py L684-720）

1. `TEST_FIXTURE_MODE=1`検出
2. Whisperモデル初期化スキップ（`processor.stt_engine = None`）
3. Ready signal送信
4. 0.5秒待機（Rust ready受信待ち）
5. スクリプト化`model_change`イベント送信:
   ```python
   {
       'type': 'event',
       'version': '1.0',
       'eventType': 'model_change',
       'data': {
           'old_model': 'medium',
           'new_model': 'base',
           'reason': 'cpu_high',
           'timestamp': <current_time_ms>
       }
   }
   ```
6. IPCループ維持（sidecarアクティブ状態）

### Rust E2E検証（dynamic_model_downgrade_e2e.rs L61-177）

1. `TEST_FIXTURE_MODE=1`設定（RAII Guard使用）
2. Python sidecar起動
3. Ready signal待機
4. 20イテレーション × 500msでIPCイベントポーリング
5. **CRITICAL ASSERTION**: `model_change`イベント受信必須（失敗 = IPC path破綻）
6. Event schema検証（`old_model`, `new_model`, `reason`必須）
7. スクリプト内容検証（`medium` → `base`, `reason=cpu_high`）

## 外部レビュー対応履歴

### Review 1（2025-10-20）
- `#[ignore]`削除、memory downgrade修正（app memory mock）

### Review 2（2025-10-20）
- 偽陽性テスト修正（`events.is_empty()`成功問題）
- TEST_FIXTURE_MODE導入
- CRITICAL ASSERTION追加

### Review 3（2025-10-20）
- グローバル環境変数汚染問題
- `TestFixtureModeGuard` RAII pattern実装
- `#[serial(env_test)]`属性追加
- race condition検証テスト2つ追加

## 最終テスト結果（2025-10-20）

- Python単体: 44/44 PASS
- Rust単体: 5/5 PASS
- Rust E2E: 3/3 PASS（IPC path + guard動作検証2）
- **合計**: 52/52テスト合格
