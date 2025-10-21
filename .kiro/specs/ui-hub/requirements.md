# Requirements Document

## Project Description (Input)

**"UI Hub" - Meeting Minutes Automator UI改善のためのデザインシステム開発環境**

Meeting Minutes Automatorの既存UI (`src/App.tsx`, `src/App.css`) から設計トークンとコンポーネントを抽出し、Storybookで改善・開発し、AIツール連携（MCP）で効率化する統合開発環境。改善した内容は既存のsrc/に統合します。

**抽出 (`src/App.css` → `tokens/app-tokens.json`) → 変換 (Style Dictionary → `tokens.css`) → 開発 (Storybook + コンポーネント抽出) → AI連携 (MCPサーバ) → 統合 (`ui-hub/` → `src/`)**

### 要件サマリー

1. **既存UIからの抽出**
   - `src/App.css` の8つのCSS変数を抽出してDTCG形式JSONに変換
   - RecordButtonコンポーネントを `src/App.tsx` L239-244から抽出
   - light/darkモード対応のトークン定義

2. **依存関係（開発時のみ）**
   - Storybook (React + Vite)
   - @storybook/addon-essentials, @storybook/addon-a11y
   - style-dictionary, chokidar-cli, tsx
   - TypeScript
   - MCP公式TypeScript SDK（stdio ベース）

3. **ファイル/ディレクトリ構造**
   - tokens/app-tokens.json — src/App.cssから抽出したトークン（light/dark）
   - sd.config.json — Style Dictionary設定
   - src/styles/tokens.css — 生成されたCSS変数
   - .storybook/main.ts / .storybook/preview.ts — Storybook設定
   - scripts/mcp-server.ts — MCPサーバ本体
   - src/components/RecordButton.tsx — App.tsxから抽出したコンポーネント
   - src/stories/RecordButton.stories.tsx — 検証用ストーリー

4. **npm-scripts**
   - sb: Storybook起動（6006番ポート）
   - tokens:build: Style Dictionary実行
   - tokens:watch: トークンJSON監視→自動ビルド
   - mcp: MCPサーバ起動
   - dev: 上記3つを並列起動（run-p）

5. **Storybook設定**
   - React + Viteフレームワーク
   - addon: essentials, a11y有効化
   - tokens.cssをpreview.tsでインポート
   - RecordButton（Idle/Recording/Disabled 3状態）

6. **Style Dictionary設定**
   - source: tokens/**/*.json
   - platforms: CSS（tokens.css）、TypeScript（tokens.d.ts）

7. **MCPサーバ（3コマンド）**
   - list_stories(): Storybook index.jsonから{id, title, kind}[]を返す
   - get_story_url(id): 指定ストーリーのiframe URLを返す
   - get_tokens(): tokens.cssとtokens/ JSONを返す
   - エラー時は明確なエラーメッセージ
   - ポート/パスは定数化（環境変数で上書き可）

8. **統合ワークフロー**
   - ui-hubで開発したトークン変更を `src/App.css` に反映
   - RecordButton改善を `src/App.tsx` に統合
   - 段階的に既存UIコンポーネントを抽出・改善・統合

### 実行手順

- **初回**: pnpm i → pnpm tokens:build → pnpm sb でStorybook確認
- **運用**: pnpm dev で全プロセス並列起動
- **トークン更新**: tokens/ JSON編集 → 数秒でtokens.css反映 → Storybook自動更新
- **MCP**: stdio稼働、Claude Code等で「UI Hub」登録
- **統合**: ui-hub/での改善を ../src/ に反映（手動コピー or スクリプト）

### 品質条件（Done の定義）

- Storybook(6006)起動、RecordButtonのIdle/Recording/Disabled切替表示
- tokens/app-tokens.json編集 → 数秒でtokens.css反映 → UI即更新
- MCPサーバ3コマンドが正常動作（ストーリー一覧/URL/トークン）
- 既存 src/App.css のCSS変数がトークンとして抽出されている
- RecordButtonが src/App.tsx から抽出され、Storybookで動作
- CI/PR設定不要

### 実現する開発体験

- **既存UI改善**: src/App.tsxのコンポーネントをStorybookで分離開発
- **設計トークンが単一情報源**: トークン変更 → 数秒でUI反映、数値で議論収束
- **AIとの同期レビュー**: MCPサーバ経由でエージェントが「ストーリー一覧」「画面表示」「トークン確認」可能
- **段階的拡張**: 将来的に他のコンポーネント（デバイスセレクタ、モデルセレクタ等）も抽出可能

---

## イントロダクション

UI Hubは、Meeting Minutes Automatorの既存UIを改善するためのトークン駆動開発環境です。既存の `src/App.css` から設計トークンを抽出し、`src/App.tsx` からコンポーネントを分離してStorybookで開発、MCPサーバ経由でAIエージェントと連携します。開発した改善内容は既存のsrc/ディレクトリに統合されます。

**親スペックとの関係**:

UI Hubは `meeting-minutes-automator` の以下の要件を満たすためのUI改善基盤として機能します:

- **REQ-002 デスクトップUI機能**: 録音開始/停止ボタン（REQ-002.2.a）をStorybookで分離開発し、品質向上
- **NFR-005 使いやすさ**: 直感的なUI設計（NFR-005.a）を設計トークンとコンポーネント駆動開発で実現
- **NFR-003 セキュリティ**: ローカル処理優先（NFR-003.c）の原則に従い、設計トークンは非機密情報のみを対象とし、機密情報はTauriアプリが管理

将来的に、親スペックの以下の要件もUI Hub環境で開発予定:
- REQ-002.3.c: 音声入力デバイス選択UI
- REQ-002.2.d: セッション管理UI

**ビジネス価値**:
- 既存UIの段階的改善（ビッグバン書き換え不要）
- トークン変更が数秒でUIに反映され、設計の意図を即座に検証
- AIエージェント連携により、コードレビューとUI確認を自動化
- Storybookでコンポーネントを分離開発し、品質向上

---

## 要件

### UIH-REQ-001: 既存UIトークン抽出
**目的**: As a **開発者**, I want **既存 src/App.css のCSS変数をDTCG形式のトークンJSONに変換**できること, so that **設計トークンを単一情報源として管理できる**

#### 受け入れ条件

1. **UIH-REQ-001.1**: WHEN tokens/app-tokens.json を生成 THEN UI Hub システム SHALL src/App.css の以下8つのCSS変数を抽出してDTCG形式に変換する:
   - `--bg-color` → `color.bg.light` / `color.bg.dark`
   - `--text-color` → `color.text.light` / `color.text.dark`
   - `--card-bg` → `color.card.bg.light` / `color.card.bg.dark`
   - `--card-border` → `color.card.border.light` / `color.card.border.dark`
   - `--input-bg` → `color.input.bg.light` / `color.input.bg.dark`
   - `--input-border` → `color.input.border.light` / `color.input.border.dark`
   - `--input-text` → `color.input.text.light` / `color.input.text.dark`
   - `--accent-color` → `color.accent` (light/dark共通)

2. **UIH-REQ-001.2**: WHEN tokens/app-tokens.json を生成 THEN UI Hub システム SHALL light モードの値を src/App.css L8-22 から、dark モードの値を L231-238 から取得する

3. **UIH-REQ-001.3**: WHEN Style Dictionary でビルド THEN UI Hub システム SHALL tokens.css に元のCSS変数名（例: `--bg-color`）で出力し、既存 src/App.css との互換性を保つ

4. **UIH-REQ-001.4**: IF 将来的に追加のスペーシング・タイポグラフィトークンを定義 THEN UI Hub システム SHALL 同じ tokens/app-tokens.json に追加可能な構造を持つ

#### 設計への影響
- `tokens/app-tokens.json` の完全な定義（light/dark 分離）
- `sd.config.json` のカスタムフォーマット（CSS変数名維持）

---

### UIH-REQ-002: RecordButtonコンポーネント抽出
**目的**: As a **開発者**, I want **既存 src/App.tsx のRecordingボタンをコンポーネントとして抽出**できること, so that **Storybookで分離開発し、品質を向上できる**

#### 受け入れ条件

1. **UIH-REQ-002.1**: WHEN RecordButton.tsx を生成 THEN UI Hub システム SHALL src/App.tsx L239-244 の以下2つのボタンを統合したコンポーネントを作成する:
   ```tsx
   // 抽出元 (src/App.tsx L239-244)
   <button className="primary" onClick={startRecording} disabled={isRecording}>
     {isRecording ? "Recording..." : "Start Recording"}
   </button>
   <button className="danger" onClick={stopRecording} disabled={!isRecording}>
     Stop Recording
   </button>
   ```

2. **UIH-REQ-002.2**: WHEN RecordButton コンポーネントを定義 THEN UI Hub システム SHALL 以下3つの状態をサポートする:
   - `idle`: "Start Recording" 表示、プライマリスタイル、クリック可能
   - `recording`: "Stop Recording" 表示、デンジャースタイル、クリック可能
   - `disabled`: "Recording Disabled" 表示、無効化スタイル、クリック不可

3. **UIH-REQ-002.3**: WHEN RecordButton.stories.tsx を生成 THEN UI Hub システム SHALL 3状態（Idle/Recording/Disabled）のストーリーを定義する

4. **UIH-REQ-002.4**: WHEN RecordButton を src/App.css のスタイルでレンダリング THEN UI Hub システム SHALL 既存の `.primary` / `.danger` / `:disabled` スタイルを再現する

#### 設計への影響
- `src/components/RecordButton.tsx` 完全実装（3状態対応）
- `src/stories/RecordButton.stories.tsx` ストーリー定義
- 将来的に src/App.tsx L239-244 を RecordButton コンポーネントで置き換え

---

### UIH-REQ-003: 依存関係管理
**目的**: As a **開発者**, I want **必要な依存パッケージを明確に定義・インストール**できること, so that **開発環境を一貫して再現できる**

#### 受け入れ条件

1. **UIH-REQ-003.1**: WHEN プロジェクトルートで pnpm install を実行 THEN UI Hub システム SHALL Storybook (React + Vite), @storybook/addon-essentials, @storybook/addon-a11y, style-dictionary, chokidar-cli, tsx, TypeScript, 公式MCP TypeScript SDK を devDependencies としてインストール完了する

2. **UIH-REQ-003.2**: IF ui-hub/package.json が存在しない THEN UI Hub システム SHALL 初期セットアップ時に適切な package.json を生成する

3. **UIH-REQ-003.3**: WHERE npm/yarn を使用する環境 THE UI Hub システム SHALL 互換性のある install コマンドをドキュメントに記載する

#### 設計への影響
- `ui-hub/package.json` の完全な定義（devDependencies全リスト）
- MCP SDK 0.6.0 の厳密バージョン指定（後方互換性確保）

---

### UIH-REQ-004: ファイル/ディレクトリ構造
**目的**: As a **開発者**, I want **標準化されたディレクトリ構造**を持つこと, so that **ファイル配置が予測可能で、メンテナンス性が高い**

#### 受け入れ条件

1. **UIH-REQ-004.1**: WHEN プロジェクト初期化完了時 THEN UI Hub システム SHALL 以下のファイル/ディレクトリを生成する:
   ```
   ui-hub/
   ├── tokens/
   │   └── app-tokens.json         # src/App.css から抽出したトークン
   ├── sd.config.json               # Style Dictionary設定
   ├── src/
   │   ├── styles/
   │   │   ├── tokens.css           # 生成物（CSS変数）
   │   │   └── tokens.d.ts          # 生成物（TypeScript型定義）
   │   └── components/
   │       ├── RecordButton.tsx     # src/App.tsx から抽出
   │       └── RecordButton.stories.tsx  # ストーリー定義
   ├── .storybook/
   │   ├── main.ts                  # Storybook設定
   │   └── preview.ts               # tokens.css読み込み
   ├── scripts/
   │   └── mcp-server.ts            # MCPサーバ本体
   ├── package.json
   └── tsconfig.json
   ```

2. **UIH-REQ-004.2**: WHERE .storybook/preview.ts THE UI Hub システム SHALL `import '../src/styles/tokens.css'` を自動挿入する

3. **UIH-REQ-004.3**: WHERE ui-hub/ ディレクトリは meeting_minutes_automator/ プロジェクトルート直下に配置される

#### 設計への影響
- プロジェクト構造図（`design.md` に記載）
- meeting_minutes_automator/ と ui-hub/ の関係

---

### UIH-REQ-005: npm-scripts定義
**目的**: As a **開発者**, I want **共通タスクを npm-scripts で実行**できること, so that **ワークフローが統一され、チーム全体で再現可能**

#### 受け入れ条件

1. **UIH-REQ-005.1**: WHEN ui-hub/package.json を生成 THEN UI Hub システム SHALL 以下のスクリプトを定義する:
   - `sb`: `storybook dev -p 6006` （Storybook起動、ポート6006固定）
   - `tokens:build`: `style-dictionary build -c sd.config.json` （トークンビルド）
   - `tokens:watch`: `chokidar "tokens/**/*.json" -c "pnpm tokens:build"` （ファイル監視）
   - `mcp`: `tsx scripts/mcp-server.ts` （MCPサーバ起動）
   - `dev`: `pnpm tokens:build && run-p sb tokens:watch mcp` （並列起動、npm-run-all2使用）

2. **UIH-REQ-005.2**: IF pnpm が未インストール THEN UI Hub システム SHALL エラーメッセージで pnpm インストール手順を提示する

#### 設計への影響
- `ui-hub/package.json` scripts定義（実装時に生成）
- 並列起動フロー（`design.md` に記載）

---

### UIH-REQ-006: Storybook設定
**目的**: As a **開発者**, I want **Storybookが React + Vite で動作し、必須addonが有効**であること, so that **コンポーネントを即座に可視化・検証できる**

#### 受け入れ条件

1. **UIH-REQ-006.1**: WHEN .storybook/main.ts を生成 THEN UI Hub システム SHALL React + Vite フレームワークを framework として指定する

2. **UIH-REQ-006.2**: WHEN .storybook/main.ts を生成 THEN UI Hub システム SHALL addons 配列に @storybook/addon-essentials と @storybook/addon-a11y を含める

3. **UIH-REQ-006.3**: WHEN Storybook が起動 (pnpm sb) THEN UI Hub システム SHALL localhost:6006 でアクセス可能な状態になり、RecordButton ストーリーが表示される

4. **UIH-REQ-006.4**: WHEN RecordButton ストーリーを表示 THEN UI Hub システム SHALL Idle/Recording/Disabled の3状態を切り替え表示可能にする

#### 設計への影響
- `.storybook/main.ts` 完全実装（`design.md` L413-431相当）
- `.storybook/preview.ts` 完全実装（`design.md` L440-456相当）
- RecordButton 完全実装（3状態対応）

---

### UIH-REQ-007: Style Dictionary設定
**目的**: As a **開発者**, I want **設計トークンJSONからCSS変数とTypeScript型定義を自動生成**できること, so that **デザイントークンが単一情報源として機能する**

#### 受け入れ条件

1. **UIH-REQ-007.1**: WHEN sd.config.json を生成 THEN UI Hub システム SHALL source に `["tokens/**/*.json"]` を指定する

2. **UIH-REQ-007.2**: WHEN sd.config.json を生成 THEN UI Hub システム SHALL platforms として以下を定義する:
   - `css`: transformGroup=`css`, buildPath=`src/styles/`, files=[{destination:`tokens.css`, format:`css/variables`}]
   - `ts`: transformGroup=`js`, buildPath=`src/styles/`, files=[{destination:`tokens.d.ts`, format:`typescript/es6-declarations`}]

3. **UIH-REQ-007.3**: WHEN pnpm tokens:build を実行 THEN UI Hub システム SHALL src/styles/tokens.css と src/styles/tokens.d.ts を生成する

4. **UIH-REQ-007.4**: WHEN tokens/app-tokens.json を編集して保存 AND pnpm tokens:watch が動作中 THEN UI Hub システム SHALL 数秒以内に tokens.css を再生成する

#### 設計への影響
- `sd.config.json` 完全実装（`design.md` L474-503相当）
- CSS/TypeScript出力例（`design.md` L552-575相当）

---

### UIH-REQ-008: MCPサーバ（3コマンド）
**目的**: As a **AIエージェント**, I want **Storybook情報とトークン情報を取得するMCPサーバ**にアクセスできること, so that **UI状態を把握し、レビュー・提案を自動化できる**

#### 受け入れ条件

1. **UIH-REQ-008.1**: WHEN pnpm mcp を実行 THEN MCP サーバ SHALL stdio で待ち受け、公式 TypeScript MCP SDK を使用して動作する

2. **UIH-REQ-008.2**: WHEN list_stories コマンドを受信 THEN MCP サーバ SHALL `http://localhost:6006/index.json` を取得し、`{id, title, kind}[]` を返す

3. **UIH-REQ-008.3**: WHEN get_story_url コマンドを受信 AND id パラメータが提供 THEN MCP サーバ SHALL `http://localhost:6006/iframe.html?id=<ID>` 形式の URL を返す

4. **UIH-REQ-008.4**: WHEN get_tokens コマンドを受信 THEN MCP サーバ SHALL src/styles/tokens.css の内容（文字列）と tokens/ 配下の JSON（配列またはマージ）を返す

5. **UIH-REQ-008.5**: IF Storybook が起動していない THEN MCP サーバ SHALL list_stories / get_story_url 実行時に明確なエラーメッセージ（"Storybook not running on port 6006"）を返す

6. **UIH-REQ-008.6**: WHERE Storybook が起動していない THE MCP サーバ SHALL エラーメッセージに "Run 'pnpm sb' to start Storybook on http://localhost:6006" を含め、ユーザーに明確な次のアクションを提示する

#### 設計への影響
- `scripts/mcp-server.ts` 完全実装（`design.md` L822-1032相当）
- MCP連携フロー図（`design.md` L357-384相当）

---

### UIH-REQ-009: 統合ワークフロー（ui-hub → src/）
**目的**: As a **開発者**, I want **ui-hubで開発した改善内容を既存 src/ に統合**できること, so that **Meeting Minutes Automator の UI が段階的に改善される**

#### 受け入れ条件

1. **UIH-REQ-009.1**: WHEN ui-hub/tokens/app-tokens.json でトークン値を変更 THEN 開発者 SHALL 対応する ../src/App.css のCSS変数を手動で更新する（例: `color.bg.light` 変更 → `--bg-color` 更新）

2. **UIH-REQ-009.2**: WHEN ui-hub/src/components/RecordButton.tsx を改善 THEN 開発者 SHALL 改善したコンポーネントを ../src/components/ に配置し、../src/App.tsx でインポートして使用する

3. **UIH-REQ-009.3**: WHERE 統合スクリプトを作成 THE UI Hub システム SHALL tokens.css の差分を検出して src/App.css に自動マージするスクリプトを提供可能にする（将来拡張）

4. **UIH-REQ-009.4**: WHEN 統合後 THEN 開発者 SHALL Meeting Minutes Automator全体のテスト（cargo test, E2Eテスト）を実行し、既存機能が破壊されていないことを確認する

#### 設計への影響
- 統合ワークフロー図（design.mdに記載）
- 将来的な自動統合スクリプト（scripts/integrate.ts）の設計

---

### UIH-REQ-010: 初回セットアップ実行手順
**目的**: As a **開発者**, I want **明確な初回セットアップ手順**を持つこと, so that **環境構築で迷わない**

#### 受け入れ条件

1. **UIH-REQ-010.1**: WHEN README または セットアップドキュメントを参照 THEN UI Hub システム SHALL 以下の手順を記載する:
   - `cd ui-hub`
   - `pnpm i` (依存インストール)
   - `pnpm tokens:build` (初回トークンCSS生成)
   - `pnpm sb` (Storybook起動確認)

2. **UIH-REQ-010.2**: WHEN 上記手順を実行 THEN UI Hub システム SHALL Storybook が `http://localhost:6006` で起動し、RecordButton が表示される

#### 設計への影響
- ui-hub/README.md（実装時に作成）

---

### UIH-REQ-011: 運用フロー（並列起動）
**目的**: As a **開発者**, I want **一つのコマンドで全プロセスを起動**できること, so that **開発開始が迅速**

#### 受け入れ条件

1. **UIH-REQ-011.1**: WHEN pnpm dev を実行 THEN UI Hub システム SHALL Storybook, tokens:watch, MCP サーバを並列起動する

2. **UIH-REQ-011.2**: WHEN pnpm dev 実行中 AND tokens/app-tokens.json を編集 THEN UI Hub システム SHALL 数秒以内に tokens.css を再生成し、Storybook が HMR (Hot Module Replacement) で即座に反映する

#### 設計への影響
- 並列起動フロー図（`design.md` に記載）
- `package.json` の `dev` スクリプト定義

---

### UIH-REQ-012: トークン更新のリアルタイム反映
**目的**: As a **デザイナー/開発者**, I want **トークンJSON編集が即座にUIに反映**されること, so that **設計の意図を数秒で検証できる**

#### 受け入れ条件

1. **UIH-REQ-012.1**: WHEN tokens/app-tokens.json を編集（例: color.bg.light の値変更） AND pnpm dev が動作中 THEN UI Hub システム SHALL 5秒以内に tokens.css を再生成する

2. **UIH-REQ-012.2**: WHEN tokens.css が再生成 THEN Storybook SHALL HMR でコンポーネントを再レンダリングし、変更された色を即座に表示する

#### 設計への影響
- トークン更新フロー図（`design.md` L304-328相当）
- chokidar設定（`package.json` scripts）

---

### UIH-REQ-013: MCP統合（Claude Code等での登録）
**目的**: As a **AIエージェントユーザー**, I want **Claude Code等のMCPクライアントからUI Hubを利用**できること, so that **AIと同期的にUIレビューできる**

#### 受け入れ条件

1. **UIH-REQ-013.1**: WHEN Claude Code の MCP 設定で "UI Hub" を登録 AND pnpm mcp が動作中 THEN MCP クライアント SHALL list_stories, get_story_url, get_tokens コマンドを実行可能にする

2. **UIH-REQ-013.2**: WHEN AIエージェントが list_stories を実行 THEN MCP サーバ SHALL Storybook の全ストーリーIDリストを返す

3. **UIH-REQ-013.3**: WHEN AIエージェントが特定ストーリーのURLを要求 THEN MCP サーバ SHALL iframe URLを返し、エージェントがスクリーンショット取得やDOM解析を実行可能にする

#### 設計への影響
- `.claude/mcp.json` 設定例（`design.md` L1041-1051相当）

---

### UIH-REQ-014: 品質条件（Done の定義）
**目的**: As a **プロジェクトマネージャー/開発者**, I want **完了判定基準**を明確にすること, so that **実装完了を客観的に判断できる**

#### 受け入れ条件

1. **UIH-REQ-014.1** (既存UI抽出): WHEN tokens/app-tokens.json を確認 THEN UI Hub システム SHALL src/App.css の8つのCSS変数がDTCG形式で定義されている

2. **UIH-REQ-014.2** (RecordButton抽出): WHEN RecordButton.tsx を確認 THEN UI Hub システム SHALL src/App.tsx L239-244 のボタンロジックを統合した3状態コンポーネントが実装されている

3. **UIH-REQ-014.3** (Storybook動作): WHEN Storybook が `http://localhost:6006` で起動 THEN UI Hub システム SHALL RecordButton の Idle/Recording/Disabled 状態切替を表示可能にする

4. **UIH-REQ-014.4** (リアルタイム反映): WHEN tokens/app-tokens.json を編集 THEN UI Hub システム SHALL 数秒以内に tokens.css に反映し、UI が即座に更新される

5. **UIH-REQ-014.5** (MCP統合): WHEN MCP サーバの3コマンド (list_stories, get_story_url, get_tokens) を実行 THEN MCP サーバ SHALL 正常なデータ（ストーリー一覧/URL/トークン内容）を返す

6. **UIH-REQ-014.6** (CI不要): WHERE CI/PR設定 THE UI Hub システム SHALL CI/PR統合を要求せず、ローカル開発のみで完結する

#### 設計への影響
- テスト計画（実装フェーズで各要件に対応するテストケースを作成）
- リリース判定基準（全6条件を満たすことが必須）

---

## 用語集

| 用語 | 定義 |
|------|------|
| **設計トークン (Design Tokens)** | 色・スペーシング・タイポグラフィなどの**非機密UI定義**を構造化したJSON。APIキー・環境変数・OAuth トークン等の機密情報は含まず、principles.md §3（セキュリティ責任境界）に従いこれらはTauriアプリ（OS Keychain）が管理する。UI Hubのトークンは既存 `src/App.css` から抽出した公開情報のみを対象とする。 |
| **DTCG** | Design Tokens Community Group。W3C標準化を目指すトークンフォーマット仕様 |
| **Style Dictionary** | Salesforceが開発したトークン変換ツール。JSON → CSS/TS/Swift等に変換 |
| **Storybook** | コンポーネント開発環境。独立した状態でUIコンポーネントを開発・テスト |
| **MCP (Model Context Protocol)** | Anthropicが提唱するAIエージェントとツール連携プロトコル |
| **stdio** | Standard Input/Output。プロセス間通信の標準手段 |
| **HMR (Hot Module Replacement)** | ページ全体をリロードせずモジュール単位で更新する技術 |
| **抽出 (Extract)** | 既存の src/App.tsx や src/App.css からコンポーネント/トークンをui-hubに分離すること |
| **統合 (Integrate)** | ui-hubで開発した改善内容を既存のsrc/に反映すること |

---

## 受け入れ基準サマリー

本要件を満たすUI Hubは以下を実現します：

1. ✅ **既存UI抽出** (UIH-REQ-001, UIH-REQ-002): src/App.css のCSS変数とsrc/App.tsx のRecordButtonを抽出
2. ✅ **依存関係の明確化** (UIH-REQ-003): package.json で全依存を定義、pnpm install で環境再現
3. ✅ **標準化されたファイル構造** (UIH-REQ-004): トークン/Storybook/MCPサーバが予測可能な配置
4. ✅ **統一されたタスク実行** (UIH-REQ-005): npm-scripts で dev, tokens:build, mcp など共通化
5. ✅ **Storybook統合** (UIH-REQ-006): React + Vite + addon-essentials/a11y で即座にコンポーネント可視化
6. ✅ **トークン駆動開発** (UIH-REQ-007, UIH-REQ-012): Style Dictionary で JSON → CSS/TS 自動生成、HMRで即反映
7. ✅ **AI連携基盤** (UIH-REQ-008, UIH-REQ-013): MCPサーバで Storybook 情報とトークンを取得可能
8. ✅ **統合ワークフロー** (UIH-REQ-009): ui-hubで開発した内容をsrc/に反映する手順定義
9. ✅ **開発体験の最適化** (UIH-REQ-011, UIH-REQ-012): pnpm dev 一つで全プロセス並列起動、トークン変更が数秒で反映
10. ✅ **品質保証** (UIH-REQ-014): 6つの完了判定基準（抽出/Storybook/リアルタイム/MCP/CI不要）

---

## Requirement Traceability Matrix

| 要件ID | 親スペック要件ID | 要件概要 | 抽出元/統合先 | 設計への影響 | 実装コンポーネント | テスト方法 |
|--------|----------------|----------|--------------|-------------|-------------------|-----------|
| UIH-REQ-001 | NFR-005（使いやすさ） | 既存UIトークン抽出 | `src/App.css` L8-22, L231-238 | `tokens/app-tokens.json` (DTCG形式) | トークン抽出スクリプト | 8つのCSS変数がトークンとして定義されているか確認 |
| UIH-REQ-002 | REQ-002.2.a（録音開始/停止ボタン） | RecordButton抽出 | `src/App.tsx` L239-244 | RecordButton 3状態コンポーネント | RecordButton.tsx / .stories.tsx | Storybook で3状態表示確認 |
| UIH-REQ-003 | - | 依存関係管理 | - | `ui-hub/package.json` devDependencies | pnpm install | `node_modules/`に全依存がインストールされるか確認 |
| UIH-REQ-004 | - | ディレクトリ構造 | - | プロジェクト構造図 | ファイル/ディレクトリ生成 | 必須ファイル全てが存在するか `ls -R` で確認 |
| UIH-REQ-005 | - | npm-scripts定義 | - | `ui-hub/package.json` scripts | scripts定義 | 各スクリプト（sb/tokens:build/dev等）が正常実行されるか確認 |
| UIH-REQ-006 | NFR-005.a（直感的なUI設計） | Storybook設定 | - | `.storybook/main.ts`, `preview.ts` | Storybook設定 | localhost:6006でRecordButton 3状態表示確認 |
| UIH-REQ-007 | NFR-005（使いやすさ） | Style Dictionary設定 | - | `sd.config.json` | トークン変換パイプライン | `tokens.css`と`tokens.d.ts`が生成されるか確認 |
| UIH-REQ-008 | - | MCPサーバ（3コマンド） | - | `scripts/mcp-server.ts`, 3ツール実装 | MCP Server (stdio) | Claude Codeから3ツール実行して正常レスポンス確認 |
| UIH-REQ-009 | REQ-002（デスクトップUI機能） | 統合ワークフロー | ui-hub → `src/App.css`, `src/App.tsx` | 統合手順ドキュメント | 手動統合 or スクリプト | 統合後に Meeting Minutes Automator のテスト全合格確認 |
| UIH-REQ-010 | - | 初回セットアップ手順 | - | ui-hub/README.md | セットアップドキュメント | 手順に従ってStorybook起動確認 |
| UIH-REQ-011 | - | 並列起動 | - | `dev` スクリプト | npm-run-all2 | `pnpm dev`で3プロセス並列起動確認 |
| UIH-REQ-012 | NFR-005.a（直感的なUI設計） | リアルタイム反映 | - | chokidar設定、HMR連携 | tokens:watch + Storybook HMR | JSON編集→5秒以内にUI更新を目視確認 |
| UIH-REQ-013 | - | MCP統合 | - | `.claude/mcp.json` 設定例 | MCP クライアント設定 | Claude CodeからMCPツール利用確認 |
| UIH-REQ-014 | 全要件の統合 | 品質条件 | 全要件の統合 | テスト計画、リリース判定基準 | 全コンポーネント | 6条件（抽出/Storybook/リアルタイム/MCP/CI不要）を全て満たすか確認 |

---

## 次のステップ

要件が承認されたら、`/kiro:spec-design ui-hub` で技術設計フェーズに進みます。
