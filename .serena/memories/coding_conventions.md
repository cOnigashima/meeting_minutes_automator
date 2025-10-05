# Coding Conventions and Style

## Global Policies
- `main`への統合前に自動フォーマッタとリンタを必ず実行
- プルリクエストには関連仕様/設計ドキュメントの更新を含める
- 設計原則に反する場合はADRを作成

## Rust / Tauri Backend
- **フォーマット**: `cargo fmt` 必須
- **Lint**: `cargo clippy --workspace --all-targets --all-features -D warnings`
- **命名**: モジュール分離（`audio::`, `ipc::`, `storage::`）
- **セキュリティ**: `tauri`の`allowlist`は必要最小限、`api-all`禁止
- **テスト**: `cargo nextest`、カバレッジ80%以上

## React / TypeScript (Tauri UI)
- **フォーマット**: `prettier --write`
- **Lint**: `eslint` (Flat Config, `@typescript-eslint`, `eslint-plugin-tailwindcss`)
- **命名**: 
  - コンポーネント: `PascalCase.tsx`
  - フック: `useCamelCase.ts`
  - Zustandストア: `*.store.ts`
- **UI**: shadcn/uiベース、`cn`ヘルパーでクラス統合
- **テスト**: `vitest` + `@testing-library/react`

## Chrome Extension
- **Manifest**: v3、`minimum_chrome_version` 維持
- **Service Worker**: `strict`モード
- **DOM操作**: `data-mm-*`属性で要素特定、`dangerouslySetInnerHTML`禁止
- **通信**: WebSocketメッセージ型を`packages/protocol`で共有
- **テスト**: `playwright`でE2E

## Python Sidecar
- **フォーマット**: `black` + `isort` (profile=black)
- **Lint**: `ruff check --select ALL --ignore I`
- **型安全性**: `mypy --strict`
- **命名**: モジュール `snake_case.py`、テストダブルは `tests/fakes/`
- **テスト**: `pytest --asyncio`、Golden Audioで検証
- **重要禁止事項**: `sounddevice`, `pyaudio`の使用禁止（ADR-001）

## File Naming Conventions
- **Rust**: `snake_case.rs`（モジュール）、`PascalCase`（構造体）、`SCREAMING_SNAKE_CASE`（定数）
- **TypeScript/React**: `PascalCase.tsx`（コンポーネント）、`camelCase.ts`（ユーティリティ）
- **Python**: `snake_case.py`（モジュール）、`PascalCase`（クラス）

## Import Organization
### Rust
1. Standard library
2. External crate
3. Internal module
4. Local imports

### TypeScript
1. React and React ecosystem
2. External libraries
3. Internal utilities and hooks
4. Type imports (separated)
5. Relative imports

### Python
1. Standard library
2. Third-party packages
3. Local application imports
