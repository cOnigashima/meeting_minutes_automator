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
- テスト: `cargo test --workspace --all-targets` を実行し、ユニット/統合/E2E をすべて通過させてから PR を作成する。`nextest` やカバレッジ計測は将来導入予定。

## React / TypeScript (Tauri UI)
- フォーマット: 2025-10-19 時点では公式なフォーマッタは未導入。VS Code 既定フォーマッタ or `npm run format`（導入予定）で差分を最小化する。Prettier 導入時はコマンドを本ドキュメントに追記する。
- Lint: ESLint 設定は未整備。ルールセットを導入する際は ADR で合意し、`npm run lint` を標準スクリプトとして追加する。
- 命名/配置: コンポーネントは `PascalCase.tsx`、フックは `useCamelCase.ts`。画面は `src/screens/`、再利用 UI は `src/components/` に配置する。
- 状態管理: 現行 UI は `useState` / `useReducer` を基本とし、グローバルストア採用が必要になった場合は ADR で目的・影響を合意する。親子間共有には `useReducer` + `React.Context` を優先し、props のネストを避ける。
- 副作用: `useEffect` の依存配列を厳守し、非同期処理や `invoke` 呼び出しは今後 `src/services/tauri.ts`（導入予定）のラッパに集約する。
- UI ステート: 読み込み中・エラー・正常の 3 パターンを小さなコンポーネントで表現し、スタイルは `App.css` を中心に管理する。Tailwind 等を導入する場合は ADR で決定。
- エラーハンドリング: Tauri コマンド呼び出しの戻り値は `try/catch` で補足し、ユーザー向けメッセージと `console.error` のログを分離する。
- テスト: 現在は手動確認が中心。UI の自動テストを追加する際は `vitest` + `@testing-library/react` を第一候補とし、E2E は必要性が明確になってから Playwright を導入する。
- 型定義: IPC やステータスの型は `src/types/` に切り出し、直接の `any` / 無根拠な `as` キャストを避ける。

## Chrome Extension
- フォーマット/リンタ: 現状はプレーン TypeScript。拡張のビルド環境を React 化する際に Prettier / ESLint を導入する。
- Manifest: v3 (`minimum_chrome_version: "116"`) を維持。Service Worker は最小限のメッセージリレーのみ実装する。
- DOM 操作: Content Script で `data-mm-*` 属性を利用し、直接的な `innerHTML` 挿入は避ける。ログは `[Meeting Minutes]` プレフィックスで統一。
- 通信: WebSocket メッセージスキーマは Rust の `WebSocketMessage` と整合させる（`isPartial` / `confidence` などの新フィールドを含める）。
- テスト: 自動テストは未導入。Docs 同期機能を実装するタイミングで Playwright などを検討し、このドキュメントを更新する。

## Python Sidecar
- フォーマット: `black` + `isort` (`profile=black`) の導入を計画し、現行コードも整形して差分を最小化する。
- Lint: `ruff check` を段階的に導入し、既存ファイルは WIP 中でも `ruff --fix` を試す。許容できないルールはコメントではなく `pyproject.toml` で管理する。
- 型安全性: `typing.TypedDict` で IPC メッセージ構造を表現し、`mypy --strict` を将来的な目標とする。現状の同期実装では戻り値タイプを明示する。
- 命名/分割: ファイルは `snake_case.py` を遵守し、`main.py` では I/O、`stt_engine/` では処理責務を分離する。副作用がある処理は関数に閉じ込め、ユニットテストが容易なように設計する。
- ロギング: 標準 `logging` で `logging.getLogger("python-stt")` を用い、IPC 送受信時に `info`、例外時に `exception` を出力する。`print` を使用する場合はログと重複しないよう統一する。
- 例外: 基底の `PythonSidecarError` を定義し、IPC 関連は `IpcError`、処理系は `ProcessingError` で区別する。ユーザーに返すエラーメッセージと内部ログ出力を分ける。
- テスト: `pytest` を基本とし、既存の `tests/test_integration.py` に加えてユニットテストを追加する。将来 async API を導入する際は `pytest.mark.asyncio` を活用する。
- IPC ベストプラクティス: `json.loads` 失敗時は `logger.exception` で記録し、`{"type": "error", "message": ...}` 形式の安全なレスポンスを返す。EOF 検知時はクリーンに終了する。



## Testing Baseline
- 新規機能は **スケルトン → ユニット → 統合 → E2E** の順に実装し、RED→GREEN サイクルを踏む。
- Rust: `cargo test --workspace --all-targets`（E2E を含む）。長時間テストが必要な場合は `-- --nocapture` でログを確認。
- Python: `.venv/bin/python -m pytest -v`。Whisper モデルが必要なテストは `tests/test_audio_integration.py` を単独で実行して検証。
- Chrome 拡張: 現状は手動検証（content-script のログ確認）。自動 E2E 化は MVP2 で Playwright を導入予定。
- カバレッジ計測は将来の課題。導入時にコマンドと合格基準をこの章に追記する。


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
