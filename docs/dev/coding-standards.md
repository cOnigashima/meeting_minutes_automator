# Coding Standards

## Purpose
Meeting Minutes Automator のコードベース全体で一貫した品質と保守性を確保するための必須ガイドラインを定める。設計原則と整合し、レビューや自動化された品質ゲートの基準となる。

## Scope
- Rust/Tauri コアアプリケーション
- React/TypeScript ベースの Tauri UI と Chrome 拡張
- Python 音声処理サイドカー
- インフラ構成および共通ドキュメント更新フロー

各チームメンバーはプルリクエスト提出前に本ドキュメントの遵守を確認すること。

## Global Policies
- `main` への統合前に自動フォーマッタとリンタを必ず実行し、CI で同じツールを走らせる。
- プルリクエストには関連仕様/設計ドキュメントの更新、もしくは「変更不要」の根拠を記載する。
- 設計原則に反する場合は ADR を作成し、承認なしに例外を導入しない。
- コードレビューはテスト有無とリグレッションリスクを最優先で確認し、スタイル逸脱は CI チェックに委任する。

## Rust / Tauri Backend
- フォーマット: `cargo fmt` を必須化。
- Lint: `cargo clippy --workspace --all-targets --all-features -D warnings` を CI で実行。
- 命名: プロセス境界に合わせてモジュール (`audio::`, `ipc::`, `storage::` など) を分離し、IPC 型は `Serde` 派生構造体でスキーマを固定。
- セキュリティ: `tauri` の `allowlist` は必要最小限の feature のみに限定し、`api-all` は禁止。
- テスト: `cargo nextest` でユニット/統合テストを実行し、失敗するテストがない状態で PR を作成。80% のカバレッジラインは下回らないよう維持。

## React / TypeScript (Tauri UI)
- フォーマット: `prettier --write` を commit 時に走らせる。
- Lint: `eslint` (Flat Config, `@typescript-eslint`, `eslint-plugin-tailwindcss`) を `pnpm lint` で実行。
- 命名: コンポーネントは `PascalCase.tsx`、フックは `useCamelCase.ts`、Zustand ストアは `*.store.ts` とする。
- UI: shadcn/ui のトークン/コンポーネントをベースにし、`cn` ヘルパーでクラスを統合。デザイン変更時は Figma → `design.md` の整合を確認。
- テスト: ビジネスロジックは `vitest`、主要 UI フローは `@testing-library/react` を併用し、ユーザーストーリー単位で最低 1 ケースを自動化。

## Chrome Extension
- フォーマット/リンタ: React UI と同じ設定を `apps/extension/` に適用。
- Manifest: v3 を前提とし、`minimum_chrome_version` を維持。Service Worker は `strict` モード、`tsconfig.json` の `moduleResolution` は `bundler` 固定。
- DOM 操作: Content Script は `data-mm-*` 属性経由で要素特定し、`dangerouslySetInnerHTML` 禁止。
- 通信: WebSocket メッセージ型を Rust 側と共有するため、`packages/protocol` に型定義を集約。
- テスト: `playwright` で主要シナリオ（Docs 連携、有効/無効切替）を E2E 化し、CI で smoke テストを実行。

## Python Sidecar
- フォーマット: `black` + `isort` (`profile=black`) を適用。
- Lint: `ruff check --select ALL --ignore I` で静的解析し、警告を無視したままにしない。
- 型安全性: `mypy --strict` を実行し、TypedDict / Protocol で IPC 契約を表現。
- 命名: モジュールとファイルは `snake_case.py`。テストダブルは `tests/fakes/` に配置。
- テスト: `pytest --asyncio` で IPC 経路をモックしつつ Golden Audio を用いた STT/VAD 検証を行う。

## Testing Baseline
- 新規機能はスケルトン → ユニット → 統合 → E2E の順で実装し、失敗するテストを先に用意する (TDD 原則)。
- すべての Tier 1 機能はオフライン環境での E2E テストケースを持つこと。
- リソース閾値 (ディスク/メモリ/CPU) の縮退挙動は長時間テストで検証し、結果を QA ノートに記録。
- CI は `lint`, `fmt`, `test` ステージを分離し、いずれかが失敗した PR はマージしない。

## Documentation & Diagram Updates
- コード変更に伴う仕様差分は `design.md` / `requirements.md` の該当セクションを更新し、PR で diff を参照できるようにする。
- 図版は `docs/uml/<spec-slug>/<カテゴリ>/ID_xxx.puml` を更新し、PR テンプレートの図版チェックリストを更新。
- 各ドキュメント末尾の「Next Actions」を再評価し、担当者・期日を反映。未更新の場合は理由をコメントで残す。



## Requirements Management
- 要件・設計ドキュメントの運用手順は `docs/dev/spec-authoring.md` を参照。実装時は必要に応じて要件IDやTraceability表を確認する。

## UML Assets
- ディレクトリ: `docs/uml/<spec-slug>/<カテゴリ>/` を用い、カテゴリは `uc|cmp|seq|cls|stm|act|dep` のいずれかとする。
- 命名: `ID_Title.puml` 形式で管理し、ID は `UC-001` などドキュメント内で一意にする。
- ツール: PlantUML ソースを Git 管理し、生成画像は CI もしくはPRコメントでレビューする（バイナリはコミットしない）。
- 運用: フェーズごとの必須図（requirements:UC / design:CMP+DEP+CLS骨子 / tasks:SEQ+必要時ACT / impl:STM+CLS詳細）を満たし、更新理由をPRに記載。
- レビューチェック: 図に影響のある実装変更では対応する spec から `#[[file:...]]` 参照を追加・更新する。

## Governance & Exceptions
- スタイル変更、テストポリシーの更新は本ドキュメントを先に修正し、関連チームの承認を得る。
- 原則違反が必要な場合は ADR を提出し、リスク評価と代替案を併記。
- 定期的に (四半期ごと) 本ドキュメントをレビューし、最新の技術選択および設計原則と整合させる。

