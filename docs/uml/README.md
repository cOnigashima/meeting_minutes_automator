# UML Assets

## Directory Convention
- `meeting-minutes-automator/` など仕様スラッグごとにサブディレクトリを作成する。
- 各サブディレクトリ配下に `uc`, `cmp`, `seq`, `cls`, `stm`, `act`, `dep` を用意し、必要な図のみコミットする。

## Naming Convention
- ファイル名は `ID_Title.puml` 形式とし、ID は `UC-001` のように種類ごとに連番で管理する。
- タイトルには図の主題を英語スラッグまたは短いスネークケースで記述する（例: `UC-001_record_and_transcribe.puml`）。

## Referencing
- 各 spec / design ドキュメントから `#[[file:docs/uml/<spec>/<category>/<file>.puml]]` で参照する。
- 図の更新が実装へ影響する場合は PR で変更理由を明記し、該当ドキュメントへのリンクを添付する。

## Tooling
- 図は PlantUML ソースのみをコミットし、生成画像は CI もしくはレビューコメントで共有する。
- CI で図の再生成を自動化する場合は、生成スクリプトを `scripts/` に配置し README に使い方を記載する。

## Review Checklist
- 仕様変更がコードへ影響する場合は、対応する spec から `#[[file:...]]` 参照を追加・更新し、PR で差分を確認する。
- フェーズごとの必須図: requirements では UseCase、design では Component + Dependency + Class 骨子、tasks では Sequence（必要に応じて Activity）、implementation では StateMachine + Class 詳細を網羅する。
- 図を更新したら、PR テンプレートの図版チェックボックスをオンにし、理由と関連ドキュメントを記載する。
- 四半期ごとに本 README をレビューし、最新の設計原則と矛盾が無いか確認する。
