# Internal Tooling Overview

最終更新: 2025-10-19  
対象スクリプト:

- `scripts/docs_crawler.py`
- `scripts/performance_report.py`
- `scripts/stability_burn_in.sh`

## docs_crawler.py

ドキュメント在庫化と公開 API 抽出を担う CLI ツール。

- `walk_files` — `os.walk()` を用いた探索ユーティリティ。`.shiori/` や `node_modules/` など無視対象のディレクトリを事前に pruning する。
- `is_binary`, `read_text`, `sha1_of`, `relpath` — 低レベルのファイル操作ヘルパー。
- `extract_headings_md_like`, `extract_links_md_like` — Markdown 系文書から見出しとリンクを抽出。
- `discover_docs` — 文書インベントリを構築し、見出し・リンク・最終更新時刻などをキャッシュ。
- `detect_lang_of_file`, `extract_symbols_from_code`, `extract_api_surface` — Python / TypeScript / Go / Rust などの公開 API をヒューリスティックに抽出。テスト系の記号 (`Test*`) は除外される。
- `build_symbol_index`, `compute_doc_mentions`, `compute_drift` — API とドキュメントの対応付け、未ドキュメント／削除済み記号の検知。
- `sanitize_docs_for_output`, `write_csv`, `write_reports`, `save_snapshot`, `load_latest_snapshot`, `now_iso` — CSV・Markdown・スナップショット出力を管理。

生成物は `.shiori/` 配下に配置され、`drift_report.md` で乖離状況（未ドキュメント記号・孤立参照など）を確認できる。

## performance_report.py

STT 統合テストやベンチマークのログを集計し、メトリクスとして出力するスクリプト。

- `parse_metrics_from_log` — ログファイルから `ipc_latency_ms` などのメトリクスを抽出。
- `check_file` — ログファイルの存在・読み取り可能性の検証。
- `analyze_metrics` — メトリクスの平均・P95・P99 などを算出し、後続のレポート生成に渡す。
- `generate_json_report`, `generate_markdown_report` — JSON / Markdown 形式のレポートを出力。

`docs_crawler.py` と `performance_report.py` は合わせて、実装コードに手を入れずにドキュメント鮮度や性能メトリクスを可視化するためのツール群である。今後は、フレッシュネス指標・参照グラフ・TODO棚卸しなどを `drift_report.md` や新規レポートへ拡張していく予定。

## stability_burn_in.sh

長時間稼働テスト（デフォルト 2 時間）を半自動化するラッパースクリプト。`cargo run --manifest-path src-tauri/Cargo.toml --bin stt_burn_in` を適切なログディレクトリ付きで実行し、`logs/platform/stability-<timestamp>-<label>/` に成果物を集約する。`--duration`・`--python`・`--session-label` オプションでランを調整でき、Step 5 の手動リソース計測と組み合わせて `docs/platform-verification.md` の「Long-run Stability Playbook」を埋めることを目的とする。実行中のログは JSON 形式なので、`python -m json.tool` や `jq` でフィルタリングできる。トランスクリプトをマスクしたまま確認する場合は既定値そのままで、プレーンテキストが必要なら `LOG_TRANSCRIPTS=1` を設定すること。
