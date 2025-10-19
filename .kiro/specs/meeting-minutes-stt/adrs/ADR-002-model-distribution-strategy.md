# ADR-002: faster-whisper モデル配布戦略

## Status
Accepted (Superseded by ADR-013 P0 bug fixes regarding bundled fallback)

## Context
faster-whisper base モデル (39MB) をインストーラーにバンドルすると、インストールサイズが2倍以上に増加します。一方、バンドルしない場合、初回起動時にネットワーク接続が必須となり、オフライン動作保証が困難です。

### トレードオフの検討

| 戦略 | インストールサイズ | オフライン動作 | 初回起動体験 | 企業環境対応 |
|-----|-----------------|--------------|-------------|-------------|
| **完全バンドル** | 89MB | ✅ 完全保証 | ✅ 即座起動 | △ 配布コスト大 |
| **完全ダウンロード** | 50MB | ❌ 不可 | ❌ ダウンロード待機 | ❌ ネットワーク必須 |
| **ハイブリッド** (採用) | 50MB | ⚠️ 条件付き | ⚠️ ユーザー選択 | ✅ システム共有パス対応 |

## Decision
**ハイブリッド戦略**: オンデマンドダウンロード + ローカルモデル優先 + システム共有パス活用 + **バンドルベースモデルフォールバック**

### 初回起動時のフロー（ADR-013で更新）
1. **ユーザー設定パス確認** (`~/.config/meeting-minutes-automator/whisper_model_path`)
2. **HuggingFace Hubキャッシュ確認** (`~/.cache/huggingface/hub/`)
3. **バンドルモデル確認** (STT-REQ-002.4/002.6):
   - 検索パス:
     - `python-stt/models/faster-whisper/`
     - `~/.local/share/meeting-minutes-automator/models/faster-whisper/`
     - `/opt/meeting-minutes-automator/models/faster-whisper/`
   - **2段階フォールバック**:
     - a. 要求されたモデルサイズを検索
     - b. 見つからない場合は **bundled 'base' モデルにフォールバック**
   - フォールバック発生時は `engine.model_size` を実際のモデルに更新
4. **ユーザー選択肢提示** (Phase 3.5で実装予定):
   - a. 今すぐダウンロード (39MB) - HuggingFace Hub接続
   - b. 後でダウンロード - バンドルbaseモデルで動作継続
   - c. ローカルモデルを指定 - ファイル選択ダイアログ
5. **バックグラウンドダウンロード** (非ブロッキング)
   - ダウンロード進捗UI表示 (進捗バー + 残り時間)
   - 一時停止/再開機能
   - ダウンロード失敗時: 自動リトライ (3回、指数バックオフ)

### インストーラーサイズ制約（ADR-013で実装状況更新）
- **現状**: バンドルbaseモデル同梱 (89MB) - **オフライン動作保証を優先**
- **MVP1実装**: `python-stt/models/faster-whisper/base/` にbaseモデルを配置
- **将来検討**: ライトバージョン (50MB, モデルなし) とフルバージョン (89MB) の2種類提供

## Consequences

### Positive
- **バンドルbaseモデルによるオフライン動作保証** (ADR-013で実装)
- 企業環境でのシステム共有パス活用 (IT部門による事前配布)
- ユーザー選択の自由度 (ダウンロード vs ローカル指定)
- バックグラウンドダウンロードによるUX低下最小化
- **2段階フォールバックによる起動失敗の回避** (STT-REQ-002.4/002.6)

### Negative
- インストールサイズ増加 (89MB) - オフライン保証とのトレードオフ
- システム共有パス検索の実装複雑性
- モデル配布UIの実装コスト (Phase 3.5として追加)
- **フォールバック発生時のモデルサイズミスマッチ対応が必要** (load_model返り値で解決)

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
| 2025-10-18 | 1.1 | Claude Code | ADR-013 P0バグ修正に伴う更新: バンドルbaseモデルフォールバック実装、2段階検索ロジック追加、load_model返り値仕様追記 |
