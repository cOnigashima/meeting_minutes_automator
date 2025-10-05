# Suggested Commands for Development

## Spec-Driven Development Workflow

### Phase 1: Specification Creation
```bash
# 1. Initialize spec with detailed project description
/kiro:spec-init [detailed description]

# 2. Generate requirements document
/kiro:spec-requirements [feature]

# 3. Validate requirements (ID, traceability, spec.json)
/kiro:validate-requirements [feature]

# 4. Generate design document
/kiro:spec-design [feature]

# 5. Validate design
/kiro:validate-design [feature]

# 6. Generate implementation tasks
/kiro:spec-tasks [feature]

# 7. Validate tasks
/kiro:validate-tasks [feature]
```

### Phase 2: Check Progress
```bash
# Check current spec status and progress
/kiro:spec-status [feature]
```

## Serena Tools for Code Navigation

### 構造探索
```python
# ディレクトリ構造の確認
mcp__serena__list_dir(relative_path=".", recursive=False)
mcp__serena__list_dir(relative_path="src-tauri/src", recursive=True)

# ファイル検索
mcp__serena__find_file(file_mask="*.rs", relative_path="src-tauri")

# ファイル内の象徴一覧（概要のみ）
mcp__serena__get_symbols_overview(relative_path="src-tauri/src/main.rs")
```

### 象徴的検索
```python
# 象徴を名前で検索（bodyなし）
mcp__serena__find_symbol(
    name_path="AudioService",
    relative_path="src-tauri/src/services/audio_service.rs",
    include_body=False,
    depth=1  # メソッド一覧も取得
)

# 象徴を名前で検索（body含む）
mcp__serena__find_symbol(
    name_path="AudioService/start_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    include_body=True
)

# 部分一致検索
mcp__serena__find_symbol(
    name_path="Audio",
    substring_matching=True,
    relative_path="src-tauri/src"
)
```

### パターン検索
```python
# 正規表現パターンで検索
mcp__serena__search_for_pattern(
    substring_pattern="import (sounddevice|pyaudio)",
    relative_path="python-stt",
    paths_include_glob="**/*.py"
)

# コンテキスト付き検索
mcp__serena__search_for_pattern(
    substring_pattern="TODO:",
    context_lines_before=2,
    context_lines_after=2,
    relative_path="."
)
```

### 依存関係分析
```python
# 象徴を参照している箇所を検索
mcp__serena__find_referencing_symbols(
    name_path="AudioService",
    relative_path="src-tauri/src/services/audio_service.rs"
)
```

### コード編集
```python
# 象徴本体を置換
mcp__serena__replace_symbol_body(
    name_path="AudioService/start_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    body="pub async fn start_recording(&mut self) -> Result<()> { ... }"
)

# 象徴の後に挿入
mcp__serena__insert_after_symbol(
    name_path="AudioService/stop_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    body="pub async fn pause_recording(&mut self) -> Result<()> { ... }"
)

# 象徴の前に挿入
mcp__serena__insert_before_symbol(
    name_path="AudioService",
    relative_path="src-tauri/src/services/audio_service.rs",
    body="use crate::models::RecordingState;"
)
```

### よく使うパターン

#### パターン1: ファイル構造の把握
```python
# Step 1: 概要取得
mcp__serena__get_symbols_overview(relative_path="file.rs")

# Step 2: 必要な象徴の詳細取得
mcp__serena__find_symbol(name_path="TargetClass", include_body=True)
```

#### パターン2: 安全な編集
```python
# Step 1: 編集対象を特定
mcp__serena__find_symbol(name_path="Class/method", include_body=True)

# Step 2: 影響範囲確認
mcp__serena__find_referencing_symbols(name_path="method")

# Step 3: 編集実行
mcp__serena__replace_symbol_body(name_path="Class/method", body="...")
```

#### パターン3: 設計原則チェック
```python
# ADR-001違反チェック（Python側での録音禁止）
mcp__serena__search_for_pattern(
    substring_pattern="import (sounddevice|pyaudio|soundfile)",
    relative_path="python-stt",
    paths_include_glob="**/*.py"
)
```

## Development Commands (実装開始後に使用)

### Tauri Application Development
```bash
# 開発モード起動
cargo tauri dev

# ビルド
cargo tauri build

# テスト実行
cargo test
cargo nextest run

# Lint and format
cargo clippy --workspace --all-targets --all-features -D warnings
cargo fmt
```

### Frontend Development
```bash
# 依存関係インストール
pnpm install

# 開発モード
pnpm dev

# ビルド
pnpm build

# Lint and format
pnpm lint
prettier --write .
```

### Chrome Extension Development
```bash
# 拡張ビルド
pnpm build:extension

# 開発モード（ウォッチ）
pnpm dev:extension

# テスト実行
pnpm test:extension
```

### Python Sidecar Development
```bash
# STTエンジン単体テスト
python -m pytest tests/test_stt.py

# VAD性能テスト
python -m pytest tests/test_vad.py

# Lint and format
black .
isort .
ruff check --select ALL --ignore I
mypy --strict .
```

### Pre-commit Hooks
```bash
# 手動実行（全ファイル）
pre-commit run --all-files

# Gitコミット時に自動実行
git commit -m "message"  # pre-commitフックが自動起動
```

## Testing Commands

### Unit Tests
```bash
# Rust
cargo test

# TypeScript
pnpm test

# Python
pytest tests/
```

### Integration Tests
```bash
pnpm test:integration
```

### E2E Tests
```bash
pnpm test:e2e
```

### Performance Tests
```bash
pnpm test:performance
python scripts/benchmark_audio.py
```

## macOS-Specific Commands
```bash
# ファイル一覧
ls -la

# ディレクトリ移動
cd /path/to/directory

# パターン検索
grep -r "pattern" .

# ファイル検索
find . -name "*.ts"

# プロセス確認
ps aux | grep python
```

## Important Notes
- **現在の開発フェーズ**: 仕様検証完了・実装準備中
- **次のステップ**: meeting-minutes-core (MVP0) のタスク承認後、Walking Skeleton実装開始
- **実装コードベースは未作成**: `src-tauri/`, `src/`, `chrome-extension/`, `python-stt/`はまだ存在しない

## Quick Reference

### Serenaの使い分け
- **ファイル全体を読みたい時**: `Read` ツール（非推奨、最終手段）
- **象徴一覧を知りたい時**: `mcp__serena__get_symbols_overview`
- **特定の象徴を読みたい時**: `mcp__serena__find_symbol`
- **パターンで検索したい時**: `mcp__serena__search_for_pattern`
- **依存関係を知りたい時**: `mcp__serena__find_referencing_symbols`
- **象徴を編集したい時**: `mcp__serena__replace_symbol_body`

### cc-sddの使い分け
- **仕様作成時**: `/kiro:spec-requirements`, `/kiro:spec-design`, `/kiro:spec-tasks`
- **仕様確認時**: `/kiro:spec-status`
- **仕様検証時**: `/kiro:validate-requirements`, `/kiro:validate-design`, `/kiro:validate-tasks`
- **実装開始時**: `/kiro:spec-impl`
