# ADR-003: IPC通信プロトコルバージョニング戦略

## Status
Accepted

## Context
IPC通信プロトコルは、meeting-minutes-core (Walking Skeleton) からmeeting-minutes-stt (Real STT) への移行時に拡張フィールドを追加します。将来的なプロトコル変更時の互換性保証と、段階的なシステム進化を支える戦略が必要です。

### 課題

1. **Walking Skeleton互換性**: meeting-minutes-core (Fake実装) が `version` フィールドを持たない場合の挙動
2. **プロトコル進化**: 将来的にフィールド追加/削除時の移行パス
3. **バージョン不一致検出**: Rust側とPython側のバージョン不一致時のエラーハンドリング
4. **デバッグ困難性**: バージョン不一致がユーザーエラーとして顕在化するまで気づかない可能性

## Decision
**厳格な互換性保証**: `version` フィールドがない場合、デフォルトで "1.0" と仮定し、バージョンネゴシエーションをオプショナル化

### バージョニングポリシー
- **versionフィールド省略時**: デフォルトで "1.0" と仮定 (meeting-minutes-core Fake実装との互換性保証)
- **メジャーバージョン変更** (1.0 → 2.0): 互換性なし、エラー表示
- **マイナーバージョン変更** (1.0 → 1.1): 下位互換性保証、新フィールドは無視
- **パッチバージョン変更** (1.0.0 → 1.0.1): 完全互換性

### バージョンネゴシエーション (オプショナル)
- 起動時にRust→Python へ `protocol_version_check` リクエスト
- タイムアウト3秒で失敗時、デフォルト "1.0" と仮定
- meeting-minutes-core Fake実装との互換性保証

## Consequences

### Positive
- Walking Skeletonからの段階的移行が円滑
- 古いクライアント (Fake実装) との互換性保証
- プロトコル進化時の明確な移行パス
- デバッグ情報 (versionフィールド) の自動収集

### Negative
- バージョンチェックのオーバーヘッド (起動時3秒タイムアウト)
- Schema定義のメンテナンスコスト
- バージョンネゴシエーション失敗時のフォールバック処理の複雑性

### Neutral
- JSON Schema検証の導入 (CI/CD互換性テストで活用)

## Implementation

### バージョンネゴシエーションプロトコル

```rust
// Rust側: 起動時バージョン確認 (オプション)
pub async fn negotiate_protocol_version(&mut self) -> Result<String> {
    let request = json!({
        "id": Uuid::new_v4().to_string(),
        "type": "request",
        "method": "protocol_version_check",
        "params": {
            "rust_version": "1.0",
            "supported_versions": ["1.0"],
        }
    });

    // タイムアウト3秒でバージョンチェック
    match timeout(Duration::from_secs(3), self.send_and_receive(request)).await {
        Ok(Ok(response)) => {
            let python_version = response["result"]["version"].as_str()
                .unwrap_or("1.0");  // デフォルト "1.0" と仮定

            if !is_compatible(python_version, "1.0") {
                return Err(AppError::IncompatibleProtocolVersion {
                    rust: "1.0".to_string(),
                    python: python_version.to_string(),
                });
            }

            Ok(python_version.to_string())
        }
        Ok(Err(_)) | Err(_) => {
            // バージョンチェック失敗時、デフォルト "1.0" と仮定
            log::warn!("Protocol version check failed, assuming version 1.0");
            Ok("1.0".to_string())
        }
    }
}

fn is_compatible(version1: &str, version2: &str) -> bool {
    // メジャーバージョンが一致すれば互換性あり
    let major1 = version1.split('.').next().unwrap_or("1");
    let major2 = version2.split('.').next().unwrap_or("1");
    major1 == major2
}
```

### Schema定義 (JSON Schema)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "IPC Request",
  "type": "object",
  "required": ["id", "type", "method"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "type": { "enum": ["request"] },
    "method": { "type": "string" },
    "version": { "type": "string", "default": "1.0" },
    "params": { "type": "object" }
  }
}
```

### CI/CD互換性テスト

**テストケース**:
1. Rust 1.0 ↔ Python 1.0: 正常動作
2. Rust 1.0 ↔ Python 1.1: 正常動作 (下位互換性)
3. Rust 1.0 ↔ Python 2.0: エラー (メジャーバージョン不一致)
4. Rust 1.0 ↔ Python (versionフィールドなし): 正常動作 (デフォルト "1.0" 仮定)

**実装ファイル**:
- `src-tauri/tests/integration_tests/ipc_versioning_test.rs`
- `python-stt/tests/test_ipc_versioning.py`

### エラーハンドリング

```rust
#[derive(Debug, thiserror::Error)]
pub enum IpcProtocolError {
    #[error("Incompatible IPC protocol version: Rust={rust}, Python={python}")]
    IncompatibleProtocolVersion { rust: String, python: String },
}

// ユーザー通知メッセージ
"PythonサイドカーとRustアプリのバージョンが一致しません (Rust: {}, Python: {})。アプリを再インストールしてください。"
```

## Alternatives Considered

### Alternative 1: versionフィールド必須化
- **Pros**: バージョン不一致を確実に検出
- **Cons**: meeting-minutes-core (Fake実装) の更新が必須、段階的移行が困難
- **却下理由**: Walking Skeleton互換性を優先

### Alternative 2: バージョンネゴシエーション不使用
- **Pros**: 実装簡素化、オーバーヘッドなし
- **Cons**: プロトコル変更時の移行パスが不明確、デバッグ困難
- **却下理由**: 将来的なプロトコル進化を考慮

## References
- `tech.md` L162-248 - IPC通信プロトコル (stdin/stdout JSON)
- `design.md` L1373-1453 - IPC Protocol Extension
- API Versioning Best Practices: [endgrate.com](https://www.endgrate.com/blog/api-versioning-best-practices)

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成 (厳格な互換性保証戦略) |
