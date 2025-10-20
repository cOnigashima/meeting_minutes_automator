# Rust E2E Testing Constraints

このドキュメントは、Task 10.3（Dynamic Model Downgrade E2E）の外部レビュー対応として作成されました。

## 背景

Task 10.3では当初、以下のような理想的なE2Eテスト設計を目指していました：

- リソース圧迫をシミュレーションして、実際のダウングレードトリガーを発動
- IPCを通じて内部状態を取得し、状態マシン遷移を検証
- WebSocket経由でChrome拡張まで通知が届くことを確認

しかし、外部レビューにより以下の**技術的制約**が明らかになりました。

## 技術的制約

### 1. 実リソース圧迫シミュレーションは不可 (External Review, 2025-10-20)

**理由:**
- メモリ圧迫: CIに危険（OOMキラー発動リスク）
- CPU圧迫: 不安定（ビジーループでタイムアウト）
- WhisperモデルのGB級ダウンロードでCI環境が破綻

**対応:**
- Python側に `TEST_FIXTURE_MODE` を導入（環境変数 `TEST_FIXTURE_MODE=1`）
- Whisperモデルを読み込まず、スクリプト化されたIPCイベントを送信
- 実際のリソース監視ロジックはPython単体テストでカバー

### 2. IPC経由での内部状態取得APIは追加しない (External Review, 2025-10-20)

**理由:**
- IPC表面積最小化原則（セキュリティ）
- Python sidecarの単一責任原則（音声処理に専念）
- テストのためだけにIPC APIを追加することは設計の妥協

**対応:**
- 状態マシン遷移の検証はPython単体テストで完結
- Rust E2Eは「IPC event path verification」に焦点を絞る

### 3. Tauri統合テスト（WebSocket → Chrome拡張）は手動検証 (本設計)

**理由:**
- TauriアプリのE2E起動にはXvfb等の仮想ディスプレイが必要
- Chrome拡張のロード、WebSocket接続、UI更新の自動検証は複雑
- CIコストと安定性のトレードオフ

**対応:**
- Rust単体テストでWebSocket broadcast schemaを検証（`commands.rs::tests`）
- IPC → Rust event receptionはE2Eで検証（`dynamic_model_downgrade_e2e.rs`）
- 実際のUI通知は手動検証項目として残す

## カバレッジ戦略

Task 10.3の完全なカバレッジは、以下の**3層テスト戦略**で達成されます：

| テスト層 | スコープ | テストファイル | カバー項目 |
|---------|---------|---------------|-----------|
| **Python単体** | リソース監視ロジック | `python-stt/tests/test_resource_monitor.py` | ✅ CPU/メモリ閾値検出<br>✅ Debounce（60秒）<br>✅ 状態マシン（monitoring→degraded→recovering）<br>✅ 回復カウンタ（10サンプル=5分）<br>✅ 5レベルモデル選択 |
| **Rust単体** | WebSocket broadcast | `src-tauri/src/commands.rs::tests` | ✅ model_changeイベントのschema検証<br>✅ 必須フィールド（old_model, new_model, reason） |
| **Rust E2E** | IPC event path | `src-tauri/tests/dynamic_model_downgrade_e2e.rs` | ✅ Python → Rust IPC通信<br>✅ TEST_FIXTURE_MODEでのイベント受信<br>✅ Event schema準拠確認 |
| **手動検証** | Tauri統合 | _(手動実行手順書は未作成)_ | ⚠️ WebSocket → Chrome拡張通知<br>⚠️ UI更新（model_changeトースト） |

## TEST_FIXTURE_MODE 仕様

### 目的
- CIでのWhisperモデルロード回避（GBダウンロード防止）
- 決定論的IPCイベント送信（リソース圧迫不要）
- E2Eテストの安定性向上

### 使用方法（RAII Guard Pattern）

**環境変数設定:**
```rust
// ❌ WRONG: Pollutes global state, breaks other tests
std::env::set_var("TEST_FIXTURE_MODE", "1");

// ✅ CORRECT: Use RAII guard for automatic cleanup
let _guard = TestFixtureModeGuard::new();
// Guard automatically removes or restores TEST_FIXTURE_MODE on drop
```

**Critical Issue (External Review, 2025-10-20)**:
- `std::env::set_var`はプロセス全体の環境変数を変更
- テスト終了後もクリーンアップされないと、他のテストが汚染される
- 実Whisperを期待するテスト（stt_e2e_test.rs）がfixture modeで動作
- `AudioProcessor.stt_engine = None`状態で`process_audio_*`を呼ぶとクラッシュ

**修正内容**:
- `TestFixtureModeGuard` 構造体追加（RAII pattern）
- 前の値を保存し、Dropで復元（panic時も安全）
- 2つのunit testで動作検証（cleanup/restore）
- `#[serial(env_test)]` 属性追加（同一ファイル内の並列実行を防止）

**並列実行の制約**:
- Rustテストハーネスは**ファイル単位**でプロセス分離
- `dynamic_model_downgrade_e2e.rs`と`stt_e2e_test.rs`は別プロセス（環境変数共有なし）
- **同一ファイル内**のテストは並列実行される（race condition発生）
- `serial_test` crateで`#[serial(env_test)]`を付与し、環境変数を触るテストを直列化
- `RUST_TEST_THREADS=1`は**不要**（過剰な制約、別ファイルには影響しない）

**Python sidecarの動作（main.py L684-720）:**
1. `TEST_FIXTURE_MODE=1` を検出
2. Whisperモデルの初期化をスキップ（`processor.stt_engine = None`）
3. Ready signalを送信
4. 0.5秒待機（Rust側のready受信を待つ）
5. スクリプト化された `model_change` イベントを送信:
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
6. IPCループを維持（sidecarをアクティブ状態に保つ）

**Rust E2E側の検証（dynamic_model_downgrade_e2e.rs L32-148）:**
1. `TEST_FIXTURE_MODE=1` を設定
2. Python sidecar起動
3. Ready signal待機
4. 20イテレーション × 500msタイムアウトでIPCイベントをポーリング
5. **CRITICAL ASSERTION**: `model_change` イベント受信を必須とする
   - 失敗 = IPC pathが壊れている
6. Event schema検証（`old_model`, `new_model`, `reason` 必須）
7. スクリプト内容検証（`medium` → `base`, `reason=cpu_high`）

## 重要な制約の明文化

### ❌ このE2Eテストが **検証しない** こと

1. **実際のリソース監視トリガー**
   - CPU 85%/60秒でのダウングレード発動
   - メモリ 2.0GB超過での即時ダウングレード
   - → Python単体テストで検証済み

2. **Pythonの内部状態マシン**
   - `monitoring` → `degraded` → `recovering` 遷移
   - 回復カウンタのリセットタイミング
   - → Python単体テストで検証済み

3. **WebSocket経由のChrome拡張通知**
   - Tauriアプリ起動
   - WebSocket broadcast
   - Chrome拡張でのtoast表示
   - → 手動検証項目（まだ手順書なし）

### ✅ このE2Eテストが **検証する** こと

1. **Python → Rust IPCイベント経路**
   - sidecar起動の成否
   - Ready signal受信
   - `model_change` イベントの正常受信

2. **IPCイベントスキーマ準拠**
   - `type=event`, `version=1.0`, `eventType=model_change`
   - `data` フィールドに `old_model`, `new_model`, `reason` が含まれる

3. **TEST_FIXTURE_MODE動作確認**
   - Whisperモデル未ロードでのsidecar起動
   - スクリプト化イベント送信

## 外部レビュー履歴

### 2025-10-20: External Review 2 (Devastating Critique)

**指摘事項:**
1. `test_model_change_event_ipc_path_verification` は `events.is_empty()` で成功していた
   - ダウングレードフローが完全に壊れていてもテストが通る
2. 本番sidecarが実Whisperモデルをロードし、CIがハング/クラッシュ
3. IPC → WebSocket flowが未テスト（commands.rsが呼ばれない）

**対応:**
- テスト名を `test_model_change_event_end_to_end` に変更
- **CRITICAL ASSERTION追加**: イベント未受信で必ず失敗させる
- `TEST_FIXTURE_MODE` 導入でWhisperロード回避
- カバレッジ戦略をこのドキュメントで明文化

### 2025-10-20: External Review 1 (6 Critical Issues)

**指摘事項:**
1. Rust E2Eに `#[ignore]` 属性（実行されていない）
2. Memory downgradeテストが失敗（app memory vs system memory）
3. Simulation hooks vs clean design
4. State machine coverage gaps
5. Rust E2E timing issues
6. Incomplete coverage claims

**対応:**
- `#[ignore]` 削除
- Memory downgrade修正（`patch.object(monitor, 'get_current_memory_usage')`）
- Simulation hooks拒否（Option A）、Python単体テスト強化（Option B採用）
- 8テスト追加（debounce/state machine/set_model_level）

## まとめ

Task 10.3のE2Eテストは、以下の理由で**意図的に限定的**です：

1. **セキュリティ**: IPC表面積最小化のため、テスト用APIを追加しない
2. **安定性**: CIでのリソース圧迫シミュレーションは危険
3. **保守性**: Whisperモデルの実ロードはCI環境を破壊する
4. **現実的**: Tauri統合テストはコスト・複雑性の観点で手動検証が妥当

その代わり、**3層テスト戦略**で完全なカバレッジを達成しています：

- Python単体: ロジック検証（トリガー条件、状態マシン）
- Rust単体: WebSocket schema検証
- Rust E2E: IPC event path検証（TEST_FIXTURE_MODE）
- 手動検証: Tauri統合（Chrome拡張通知）

この制約は**設計判断**であり、テスト実装の不備ではありません。
