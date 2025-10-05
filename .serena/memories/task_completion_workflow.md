# Task Completion Workflow

## タスク完了時に実行すべきこと

### 1. Pre-commit Hooksの実行
```bash
# 全ファイルに対してフックを実行
pre-commit run --all-files
```

### 2. Lintとフォーマット

#### Rust
```bash
cargo fmt
cargo clippy --workspace --all-targets --all-features -D warnings
```

#### TypeScript/React
```bash
pnpm lint
prettier --write .
```

#### Python
```bash
black .
isort .
ruff check --select ALL --ignore I
```

### 3. テストの実行

#### ユニットテスト
```bash
# Rust
cargo test

# TypeScript
pnpm test

# Python
pytest tests/
```

#### 統合テスト
```bash
pnpm test:integration
```

#### E2Eテスト（主要シナリオ）
```bash
pnpm test:e2e
```

### 4. ドキュメント更新
- コード変更に伴う仕様差分を`design.md`/`requirements.md`で更新
- PlantUML図の更新（`docs/uml/<spec-slug>/<カテゴリ>/ID_xxx.puml`）
- 各ドキュメント末尾の「Next Actions」を再評価

### 5. Gitコミット
```bash
# ファイルをステージング
git add .

# コミット（pre-commitフックが自動実行される）
git commit -m "feat: implement feature X

- Detailed description
- Related requirements: REQ-001, REQ-002

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"

# プッシュ（必要に応じて）
git push
```

## 重要な注意事項

### 静的解析の強制
- Python側での音声録音ライブラリ（sounddevice, pyaudio）の使用は禁止
- `scripts/check_forbidden_imports.py`が自動チェック
- ADR-001に基づく設計原則の強制

### テストカバレッジ
- ユニットテスト: 80%以上
- 統合テスト: 主要シナリオ100%
- Tier 1機能（オフライン必須）: オフラインE2Eテスト必須

### ドキュメント整合性
- 要件ID（REQ-###, NFR-###等）の追跡
- Requirement Traceability Matrixの更新
- PlantUML図とテキストの一致確認

### PR作成前チェックリスト
- [ ] Pre-commit hooksが成功
- [ ] 全テストが成功
- [ ] ドキュメントが更新済み
- [ ] Requirement Traceability Matrixが更新済み
- [ ] コミットメッセージに要件IDを含む
