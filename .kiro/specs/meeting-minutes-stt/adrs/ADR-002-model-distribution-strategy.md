# ADR-002: faster-whisper モデル配布戦略

## Status
Accepted

## Context
faster-whisper base モデル (39MB) をインストーラーにバンドルすると、インストールサイズが2倍以上に増加します。一方、バンドルしない場合、初回起動時にネットワーク接続が必須となり、オフライン動作保証が困難です。

### トレードオフの検討

| 戦略 | インストールサイズ | オフライン動作 | 初回起動体験 | 企業環境対応 |
|-----|-----------------|--------------|-------------|-------------|
| **完全バンドル** | 89MB | ✅ 完全保証 | ✅ 即座起動 | △ 配布コスト大 |
| **完全ダウンロード** | 50MB | ❌ 不可 | ❌ ダウンロード待機 | ❌ ネットワーク必須 |
| **ハイブリッド** (採用) | 50MB | ⚠️ 条件付き | ⚠️ ユーザー選択 | ✅ システム共有パス対応 |

## Decision
**ハイブリッド戦略**: オンデマンドダウンロード + ローカルモデル優先 + システム共有パス活用

### 初回起動時のフロー
1. **ユーザー設定パス確認** (`~/.config/meeting-minutes-automator/whisper_model_path`)
2. **システム共有パス確認**:
   - macOS: `/usr/local/share/faster-whisper/`
   - Windows: `C:\ProgramData\faster-whisper\`
   - Linux: `/usr/share/faster-whisper/`
3. **HuggingFace Hubキャッシュ確認** (`~/.cache/huggingface/hub/`)
4. **ユーザー選択肢提示**:
   - a. 今すぐダウンロード (39MB) - HuggingFace Hub接続
   - b. 後でダウンロード - オフライン機能無効化、UI通知表示
   - c. ローカルモデルを指定 - ファイル選択ダイアログ
5. **バックグラウンドダウンロード** (非ブロッキング)
   - ダウンロード進捗UI表示 (進捗バー + 残り時間)
   - 一時停止/再開機能
   - ダウンロード失敗時: 自動リトライ (3回、指数バックオフ)

### インストーラーサイズ制約
- **目標**: 50MB以下 (モデルバンドルなし)
- **Full版オプション**: 企業向けに baseモデル同梱版を提供 (89MB)

## Consequences

### Positive
- インストールサイズ最小化 (50MB以下)
- 企業環境でのシステム共有パス活用 (IT部門による事前配布)
- ユーザー選択の自由度 (ダウンロード vs ローカル指定)
- バックグラウンドダウンロードによるUX低下最小化

### Negative
- 初回起動時のダウンロード待機 (ネットワーク接続必須の場合)
- システム共有パス検索の実装複雑性
- モデル配布UIの実装コスト (Phase 3.5として追加)

### Neutral
- Full版インストーラーのメンテナンスコスト (将来検討)

## Implementation

### UI実装 (Phase 3.5)
- Tauri UIにモデルダウンロードダイアログ追加
- ダウンロード進捗表示 (WebSocket経由でPython→Rust→UI)
- 一時停止/再開/キャンセル機能

### システム共有パス検索
```rust
fn search_system_model_path() -> Option<PathBuf> {
    let system_paths = if cfg!(target_os = "macos") {
        vec!["/usr/local/share/faster-whisper/"]
    } else if cfg!(target_os = "windows") {
        vec!["C:\\ProgramData\\faster-whisper\\"]
    } else {
        vec!["/usr/share/faster-whisper/", "/opt/faster-whisper/"]
    };

    system_paths
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}
```

### ユーザー選択肢UI (Tauri Dialog)
```typescript
interface ModelDownloadOptions {
  type: 'download_now' | 'download_later' | 'specify_local';
  localPath?: string;  // type='specify_local'の場合のみ
}
```

### 量子化モデル検討 (将来拡張)
- int8量子化: サイズ25%削減 (39MB → 10MB)
- 精度低下: 5%以内 (許容範囲)
- CTranslate2のquantization機能を活用

## References
- `.kiro/steering/principles.md` - オフラインファースト原則
- `design.md` L842-885 - WhisperSTTEngine モデル配布戦略
- `requirements.md` STT-REQ-002 - faster-whisper Integration (Offline-First)

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成 (ハイブリッド配布戦略) |
