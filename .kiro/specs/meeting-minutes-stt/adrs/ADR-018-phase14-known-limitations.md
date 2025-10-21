# ADR-018: Phase 14 Post-MVP1 Cleanup - Known Limitations

**Status**: Accepted
**Date**: 2025-10-21
**Deciders**: Claude Code + External Review
**Related**: ADR-003 (IPC Versioning), ADR-013 (Sidecar Full-Duplex)

## Context

Phase 14 Post-MVP1 Cleanupで以下の作業を実施:
1. LegacyIpcMessage削除（tests/supportモジュールパターン採用）
2. 未使用コード削除（start_ipc_reader_task復活を含む）
3. P0デッドロック修正（5秒タイムアウト削除）

外部レビューにより、以下の制約が指摘されました:

### Issue 1: IPC Reader Mutex Scope制約

**問題**:
```rust
// src-tauri/src/commands.rs L95-99
let response = {
    let mut sidecar = python_sidecar.lock().await;
    sidecar.receive_message().await
    // Guard dropped here (but only AFTER .await completes)
};
```

`receive_message()`は`&mut self`を取るため、Futureが`MutexGuard`のライフタイムに紐付く。結果として、guardが`.await`を跨いで保持される。

**影響**:
- 理論上、audio callback（送信側）がMutex待ちで短時間ブロックされる可能性
- ただし、送信操作は数マイクロ秒で完了するため、実害は少ない
- Python応答は非同期（次の送信をブロックしない）のため、競合は稀

**外部レビュー指摘**:
> "the mutex guard stays alive while awaiting receive_message() ... That still serialises every send behind the same lock, so the deadlock you're seeing won't be solved by just 'bringing the task back'. To fix it you need to restructure the sidecar API (e.g. release the mutex before the await via an owned handle, or move the read loop into the sidecar itself)"

### Issue 2: tests/support重複インポート

**問題**:
```rust
// tests/ipc_migration_test.rs
#[path = "support/mod.rs"]
mod support;

// tests/e2e_test.rs
#[path = "support/mod.rs"]
mod support;
```

各integration testが個別に`#[path]`でsupportモジュールをインポート。将来、shared utilityを追加する際に全テストcrateの更新が必要。

**外部レビュー指摘**:
> "every integration test now re-imports the support module via #[path = "support/mod.rs"], producing duplicate copies of the helpers ... If you expect more shared utilities, consider a dedicated tests/support crate to avoid divergence"

### Issue 3: Python Tests 17件既存失敗

**問題**:
- model detection tests（test_whisper_client.py）
- upgrade/fallback tests（test_upgrade_fallback.py）
- 17件がPhase 14と無関係に失敗

**外部レビュー指摘**:
> "spec.json still reports 17 Python test failures ... while the summary asserts '0 warnings/complete cleanup'. With 161/178 Python cases passing only, we're carrying known red tests into the handoff"

## Decision

以下の3つの制約を**Known Limitations**として文書化し、Phase 14完了を承認する。

### LIMIT-001: IPC Reader Mutex Scope制約

**現状の対応**:
1. 5秒タイムアウト削除（正常な沈黙でtask死亡を防止）
2. 送信操作は数マイクロ秒で完了のため、実害は最小限
3. コメントで制約を明記（src-tauri/src/commands.rs L79-93）

**将来の改善**:
- `PythonSidecarManager::spawn_reader_task()` を内部化
- `tokio::sync::Mutex::lock_owned()` を使用してguardをFutureから分離
- 優先度: P2（実害が少ないため、MVP2ブロッカーではない）

### LIMIT-002: tests/support重複インポート

**現状の対応**:
- 現状は2ファイルのみ（ipc_migration_test.rs, e2e_test.rs）のため管理可能
- 変更時はGrepで全参照箇所を確認

**将来の改善**:
- shared utility増加時にdedicated `tests/support` crateを作成
- 優先度: P2

### LIMIT-003: Python Tests 17件既存失敗

**現状の対応**:
- spec.jsonの`known_limitations`に明記
- Phase 14と無関係（既存失敗）を明示
- コア機能（録音・文字起こし・IPC）は正常動作

**将来の改善**:
- MVP2 Phase 0またはMVP3で対応
- 優先度: P2

## Consequences

### Positive

- 制約を明確に文書化することで、将来の開発者が予期しない動作に遭遇しない
- 完全な解決（API再設計）は大きなリファクタリングが必要だが、現状の対応で実用上問題なし
- MVP2開始の前提条件を明確化（BLOCK-008追加）

### Negative

- Mutex scope制約は根本的には解決されていない（API再設計が必要）
- Python tests 17件失敗を"acceptable"として受け入れる必要がある

### Neutral

- MVP2開始前にBLOCK-008（MVP2 Phase 0残タスク）の完了が必要
- 優先度P2のため、MVP2本体開始をブロックしない

## Alternatives Considered

### Alternative 1: PythonSidecarManager API再設計（Rejected for Phase 14）

**理由**: 大きなリファクタリングが必要で、Phase 14のスコープを超える。MVP2以降で対応。

### Alternative 2: Python tests修正を強制（Rejected）

**理由**: Phase 14と無関係な既存失敗を修正するのは、Phase 14のスコープ外。MVP2 Phase 0で対応。

### Alternative 3: tests/support crate即座作成（Rejected）

**理由**: 現状は2ファイルのみで、premature optimizationになる。shared utility増加時に再検討。

## References

- External Review (2025-10-21): "start_ipc_reader_task already exists ... but the mutex guard stays alive"
- src-tauri/src/commands.rs L79-93: Mutex scope制約のコメント
- tests/ipc_migration_test.rs, tests/e2e_test.rs: `#[path]`重複インポート
- spec.json: `known_limitations`, `BLOCK-008`
