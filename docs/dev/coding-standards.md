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

## Subsystem Design & Implementation Guidelines (Draft)

### Rust / Tauri Backend — Result エラー分類と再試行戦略 (Backend TL Proposal)
- `Result<T, DomainError>` で戻り値を統一し、ドメイン例外は `Transient` / `Retryable` / `Fatal` の 3 種に分類して `thiserror` でエンコードする。`DomainError` には `category()` ヘルパーを実装し、呼び出し側でガードを簡潔化する。
- 再試行ポリシーは `Transient` のみ対象。`backoff` クレートで指数バックオフ (初期 200ms、係数 2.0、最大 5 試行) を標準化し、`tracing` スパンに `retry.attempt` メタデータを残す。最終失敗時は `retry.exhausted=true` を付与。
- IPC/FFI 例外は `Fatal` とみなし即座に UI へ通知、ユーザー可視メッセージと `Sentry` タグ (`component=tauri-backend`, `error.category=fatal`) を付与する。
- エラー分類カタログを `docs/dev/rust-error-catalog.md` に保守し、追加時は PR で diff を確認。代表的な API 呼び出しの分類表は以下を初期案とする。

| API/処理 | エラー例 | DomainError | ハンドリング | 計測 |
| --- | --- | --- | --- | --- |
| `audio::ingest()` | I/O 一時失敗 (`std::io::ErrorKind::WouldBlock`) | `Transient` | 指数バックオフ + 最大 3 回リトライ | `metrics::counter!("audio.retry", {"source" = "ingest"})` |
| `ipc::send_transcript()` | JSON 変換失敗 | `Fatal` | 即失敗応答、UI へ `error-modal` 表示 | `tracing::error!` + Sentry capture |
| `storage::session::load()` | SQLite ロック | `Retryable` | 100ms 間隔で最大 5 回リトライ、失敗時に復旧ガイド表示 | `metrics::histogram!("storage.retry_wait_ms")` |

- **実行計画 (Backend TL / Due: 2025-10-24)**
  - 2025-10-14: 既存エラーを棚卸しし、`DomainError` に分類を実装。
  - 2025-10-16: `backoff` ラッパと `tracing` メタデータ付与ユーティリティを PR 化。
  - 2025-10-21: `docs/dev/rust-error-catalog.md` に API 別テーブルを追記し、レビュー完了後 `Subsystem` セクションの TODO を解消。

### React / TypeScript — 状態管理ポリシー (Frontend TL Proposal)
- グローバル状態は `zustand` の `appStore` に限定し、UI 表現専用状態は React ローカルステートを使用。`useEffect` での副作用は `async` 関数を分離してテスト可能化する。副作用が 2 つ以上ある場合は `useAsyncTransition` カスタムフックを追加。
- Server state は `@tanstack/react-query` を第一選択とし、`queryKey` は `[domain, entityId, scope]` パターンを必須化。手動キャッシュ操作は禁止し、`invalidateQueries` で整合性を保つ。`staleTime` は API ごとに `docs/dev/frontend-latency-matrix.md` で管理する。
- 状態遷移図 (主要モーダルとレコーディングフロー) を `docs/uml/mm-recorder/STM_stateflow.puml` に同期させる。レイテンシ >300ms の API 呼び出しでは optimistic update を採用し、`OptimisticState` インターフェースに `apply() / rollback()` を実装、失敗時には UI トースト + React Query `onError` でロールバックする。
- 型安全性を保つため、状態セレクタは `ReturnType<typeof useStore>` のみ公開し、任意の `getState()` 取得は禁止。外部アクセスが必要なケースはファサードフック (`useRecordingState`) を追加する。
- **実行計画 (Frontend TL / Due: 2025-10-24)**
  - 2025-10-15: `frontend-latency-matrix.md` を作成し、API ごとの `staleTime` と optimistic 条件を明記。
  - 2025-10-18: `useOptimisticMutation` ユーティリティと `OptimisticState` インターフェースを実装し、録音開始/停止フローでパイロット適用。
  - 2025-10-22: 状態遷移図を更新し、Figma とドキュメントの整合をデザインチームとレビュー。

### Python Sidecar — async/await ハンドリング (ML TL Proposal)
- 音声処理は `asyncio` イベントループ上で実行し、CPU バウンド処理は `anyio.to_thread.run_sync` でオフロード。`asyncio.run` はエントリポイント以外で呼び出さない。
- タスクキャンセルは `contextlib.AsyncExitStack` で集中管理し、`CancelledError` を飲み込まずに呼び出し元へ再伝播。ログには `task.name` と `recording_id` を含める。
- I/O 待ちのタイムアウトは `asyncio.wait_for(..., timeout=3.0)` を標準とし、タイムアウト例外は `Transient` として Rust 側へ再試行ヒントを返す。API 呼び出し前後で `asyncio.timeout` コンテキストを利用。
- ストリーミング API のバックプレッシャは `asyncio.Queue(maxsize=4)` を標準とし、オーバーフロー時は `BackpressureWarning` を発火させ Rust 側にビジュアル通知 (UI バナー) を要求。メモリ使用量は `tracemalloc` で計測し、1 セッション上限 256 MB を監視する。
- **実行計画 (ML TL / Due: 2025-10-24)**
  - 2025-10-13: 既存のストリーム処理を棚卸しし、`Queue` ベースへ統一するためのリファクタリング計画を策定。
  - 2025-10-17: `BackpressureWarning` 実装と Rust 連携の e2e テストを追加。
  - 2025-10-23: `docs/dev/python-streaming-guidelines.md` を執筆し、カバレッジ計測とともにレビュー提出。

## Code Review & PR Checklist (Draft)
- **機能リスク**: 要件との差分、ロールバック戦略、既存データ互換性を確認し、PR テンプレートの「Impact」欄に記述を必須化。
- **テスト**: 追加/更新された自動テスト、手動確認ケース、CI 結果のステータスを明記。テスト欠如時は期限付きフォローアップ Issue をリンク。
- **パフォーマンス**: 主要パスのスループット・メモリへの影響を計測 (例: `cargo bench`, `pnpm vitest --runInBand --runTestsByPath`). 測定が難しい場合は仮説と未検証理由を記録。
- **セキュリティ**: 権限変更、データ境界、シークレット取扱いをチェック。`threat-model.md` に該当行がある場合は更新を要求。
- **ドキュメント更新**: 仕様・ユーザードキュメント・運用 Runbook の差分を確認し、必要な追記を完了しているかチェックリストで検証する。


## UI/UX Quality Standards (Draft)
- アクセシビリティ: WCAG 2.2 AA を最低ラインとし、対話要素には `aria-*` と `role` を付与。Vitest + `jest-axe` で自動検査を実施。
- レスポンシブ/テーマ対応: 320px〜1440px のブレークポイントをデザインレビューで確認し、ダーク/ライトテーマ両方でコントラスト比 4.5:1 を維持する。
- エラー表示: 共通 `ErrorBanner`/`Toast` コンポーネントを使用し、ユーザー向け文言は `messages/error-catalog.json` から取得。技術詳細は `traceId` と共に開発者向けログへ送出。
- TODO: UI アクセシビリティ手動検証手順を `docs/dev/ui-accessibility-checklist.md` として起草する。

## Testing Baseline
- 新規機能はスケルトン → ユニット → 統合 → E2E の順で実装し、失敗するテストを先に用意する (TDD 原則)。
- カバレッジ測定は Rust: `cargo tarpaulin --engine llvm --out Xml`, TypeScript: `pnpm vitest --coverage`, Python: `pytest --cov` を実行し、CI で `artifacts/coverage/<lang>/` に保存する。週次の品質会議 (毎週月曜 10:00) でダッシュボードをレビュー。
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

## Draft Outline & Next Steps
- `Subsystem Design & Implementation Guidelines`: エラー分類サンプル、状態遷移図リンク、バックプレッシャ制御の詳細を 2025-10-24 までに補完 (担当: 各 TL)。
- `Code Review & PR Checklist`: PR テンプレ改訂とレビュアートレーニング資料作成を 2025-10-17 に完了 (担当: QA Lead)。
- `Dependency Lifecycle Standards`: 監査レポート自動化とバージョンマトリクス更新を 2025-10-31 までに実装 (担当: DevOps Lead)。
- `UI/UX Quality Standards`: アクセシビリティ手動検証手順とデザイン QA フローを 2025-10-24 までにドラフト化 (担当: Design Lead)。
- `Testing Baseline`: カバレッジ可視化ダッシュボード (Looker Studio または Grafana) を 2025-11-07 までに PoC 提出 (担当: QA Lead + Data Engineer)。
- `Incident Response & Postmortems`: オンコールローテーションとコミュニケーション手順を 2025-10-20 までにテンプレへ統合 (担当: SRE Lead)。
