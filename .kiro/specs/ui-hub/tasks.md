# UI Hub 実装タスク

## 実装計画概要

**目的**: Meeting Minutes Automatorの既存UI (`src/App.tsx` + `src/App.css`) をデザイントークン駆動に移行するため、`ui-hub/`配下に開発環境を構築し、最終的に本体に適用する

**前提条件**:
- 既存UI: `src/App.tsx` (271行) + `src/App.css` (257行、8個のCSS変数）
- 開発環境: `ui-hub/`ディレクトリに独立したStorybookセットアップ
- 本体適用: `ui-hub/src/styles/tokens.css` → `src/App.css`統合

---

## Phase 0: 既存UI分析と準備

### タスク0.1: 既存UIトークン抽出とマッピング表作成
**目的**: `src/App.css`の全CSS変数とハードコード値を抽出し、トークンJSONへのマッピングを定義

**成果物**:
- CSS変数マッピング表（既存変数名 → 新トークン名）
- ハードコード値リスト（border-radius, padding, gap, shadow, button colors）

**実装内容**:
1. `src/App.css` L8-21のライトモードCSS変数8個を抽出
2. `src/App.css` L231-238のダークモードCSS変数7個を抽出
3. ハードコード値を抽出:
   - `border-radius: 8px, 12px` → `radius.sm, radius.md`
   - `padding: 0.6em 1.2em, 1.5rem` → `space.2, space.4, space.6`
   - `gap: 1rem, 1.5rem` → `space.4, space.6`
   - `box-shadow: 0 6px 18px...` → `shadow.card, shadow.sm`
   - Button colors: `#f44336, #ff9800` → `color.danger.primary, color.warning.primary`
4. マッピング表をドキュメント化（design.md L70-93参照）

**検証**:
- 全8個のCSS変数がマッピング表に含まれる
- ハードコード値が適切なトークンカテゴリに分類される

_Requirements: REQ-001_

---

### タスク0.2: 既存UIコンポーネント構造分析
**目的**: `src/App.tsx`から抽出すべきコンポーネントを特定

**成果物**:
- コンポーネント抽出リスト（RecordButton, DeviceSelector等）
- 各コンポーネントの状態・プロパティ定義

**実装内容**:
1. `src/App.tsx` L239-244のRecordButton部分を特定
   - 状態: Idle/Recording/Disabled
   - プロパティ: `isRecording`, `onClick`, `disabled`
2. `src/App.tsx` L170-183のDeviceSelector部分を特定
   - 状態: デバイスリスト、選択中デバイス
   - プロパティ: `audioDevices`, `selectedDeviceId`, `onChange`, `disabled`
3. 抽出優先度を決定（Phase 1ではRecordButtonのみ）

**検証**:
- 既存UIの全主要コンポーネントがリストに含まれる
- 各コンポーネントの状態・プロパティが正確に記述される

_Requirements: REQ-005_

---

## Phase 1: 基盤セットアップ

### タスク1.1: ui-hub/プロジェクト初期化
**目的**: 本体と独立した開発環境をセットアップ

**成果物**:
- `ui-hub/package.json`
- `ui-hub/tsconfig.json`, `ui-hub/tsconfig.node.json`
- `ui-hub/.gitignore`

**実装内容**:
1. `ui-hub/`ディレクトリ作成
2. `pnpm init`で`package.json`生成
3. devDependenciesインストール:
   ```json
   {
     "@storybook/react": "^8.0.0",
     "@storybook/react-vite": "^8.0.0",
     "@storybook/addon-essentials": "^8.0.0",
     "@storybook/addon-a11y": "^8.0.0",
     "style-dictionary": "^4.0.0",
     "chokidar-cli": "^3.0.0",
     "tsx": "^4.0.0",
     "@modelcontextprotocol/sdk": "0.6.0",
     "npm-run-all2": "^6.0.0",
     "react": "^18.2.0",
     "react-dom": "^18.2.0",
     "typescript": "^5.0.0",
     "vite": "^5.0.0"
   }
   ```
4. `tsconfig.json`作成（React + Vite設定）
5. npm-scriptsを定義:
   ```json
   {
     "sb": "storybook dev -p 6006",
     "tokens:build": "style-dictionary build -c sd.config.json",
     "tokens:watch": "chokidar \"tokens/**/*.json\" -c \"pnpm tokens:build\"",
     "mcp": "tsx scripts/mcp-server.ts",
     "dev": "pnpm tokens:build && run-p sb tokens:watch mcp"
   }
   ```

**検証**:
- `pnpm install`が成功
- `ui-hub/`が本体の`node_modules`と独立
- TypeScriptコンパイルが成功

_Requirements: REQ-002, REQ-004_

---

### タスク1.2: 基本ディレクトリ構造作成
**目的**: トークン・コンポーネント・設定ファイルの配置場所を確立

**成果物**:
- `ui-hub/tokens/`
- `ui-hub/src/components/`
- `ui-hub/src/stories/`
- `ui-hub/src/styles/`
- `ui-hub/scripts/`
- `ui-hub/.storybook/`

**実装内容**:
1. 上記ディレクトリを作成
2. `.gitkeep`を配置（空ディレクトリのgit管理）
3. `src/styles/`に`README.md`（「このディレクトリは自動生成」の注記）

**検証**:
- 全ディレクトリがgit管理下
- ディレクトリ構造が予測可能

_Requirements: REQ-003_

---

## Phase 2: デザイントークンパイプライン構築

### タスク2.1: tokens/base.tokens.json作成
**目的**: 既存CSS変数をトークンJSONに変換

**成果物**:
- `ui-hub/tokens/base.tokens.json`

**実装内容**:
1. タスク0.1のマッピング表を基に、トークンJSONを作成:
   ```json
   {
     "color": {
       "bg": {
         "light": {"value": "#f6f6f6", "type": "color"},
         "dark": {"value": "#101015", "type": "color"}
       },
       "text": {
         "light": {"value": "#0f0f0f", "type": "color"},
         "dark": {"value": "#f6f6f6", "type": "color"}
       },
       "card": {
         "bg": {
           "light": {"value": "#ffffff", "type": "color"},
           "dark": {"value": "rgba(255, 255, 255, 0.05)", "type": "color"}
         },
         "border": {
           "light": {"value": "rgba(0, 0, 0, 0.08)", "type": "color"},
           "dark": {"value": "rgba(255, 255, 255, 0.12)", "type": "color"}
         }
       },
       "input": {
         "bg": {
           "light": {"value": "#ffffff", "type": "color"},
           "dark": {"value": "rgba(255, 255, 255, 0.1)", "type": "color"}
         },
         "border": {
           "light": {"value": "rgba(0, 0, 0, 0.15)", "type": "color"},
           "dark": {"value": "rgba(255, 255, 255, 0.25)", "type": "color"}
         },
         "text": {
           "light": {"value": "#0f0f0f", "type": "color"},
           "dark": {"value": "#f6f6f6", "type": "color"}
         }
       },
       "accent": {
         "primary": {"value": "#396cd8", "type": "color"}
       },
       "danger": {
         "primary": {"value": "#f44336", "type": "color"}
       },
       "warning": {
         "primary": {"value": "#ff9800", "type": "color"}
       }
     },
     "space": {
       "2": {"value": "0.6em", "type": "dimension"},
       "4": {"value": "1rem", "type": "dimension"},
       "6": {"value": "1.5rem", "type": "dimension"}
     },
     "radius": {
       "sm": {"value": "8px", "type": "dimension"},
       "md": {"value": "12px", "type": "dimension"}
     },
     "shadow": {
       "sm": {"value": "0 2px 2px rgba(0, 0, 0, 0.2)", "type": "shadow"},
       "card": {"value": "0 6px 18px rgba(15, 15, 15, 0.08)", "type": "shadow"}
     }
   }
   ```

**検証**:
- 全8個のCSS変数がトークンとして定義される
- ライト/ダーク両モードの値が含まれる
- JSON構文が正しい

_Requirements: REQ-006_

---

### タスク2.2: Style Dictionary設定作成
**目的**: トークンJSON → CSS変数への変換ルールを定義

**成果物**:
- `ui-hub/sd.config.json`

**実装内容**:
1. Style Dictionary設定ファイル作成:
   ```json
   {
     "source": ["tokens/**/*.json"],
     "platforms": {
       "css": {
         "transformGroup": "css",
         "buildPath": "src/styles/",
         "files": [
           {
             "destination": "tokens.css",
             "format": "css/variables",
             "options": {
               "outputReferences": false
             }
           }
         ]
       },
       "ts": {
         "transformGroup": "js",
         "buildPath": "src/styles/",
         "files": [
           {
             "destination": "tokens.d.ts",
             "format": "typescript/es6-declarations"
           }
         ]
       }
     }
   }
   ```
2. カスタムtransformの追加（既存CSS変数名との互換性維持）:
   - `color.bg.light` → `--bg-color` (ライトモード)
   - `color.bg.dark` → `--bg-color` (ダークモード)
   - `@media (prefers-color-scheme: dark)`での自動切替

**検証**:
- `pnpm tokens:build`が成功
- `src/styles/tokens.css`が生成される
- 既存CSS変数名（`--bg-color`等）が出力される

_Requirements: REQ-006_

---

### タスク2.3: トークンビルド実行と検証
**目的**: Style Dictionaryが正しくCSS変数を生成することを確認

**成果物**:
- `ui-hub/src/styles/tokens.css`
- `ui-hub/src/styles/tokens.d.ts`

**実装内容**:
1. `pnpm tokens:build`実行
2. 生成されたCSS変数を確認:
   ```css
   :root {
     --bg-color: #f6f6f6;
     --text-color: #0f0f0f;
     --card-bg: #ffffff;
     --card-border: rgba(0, 0, 0, 0.08);
     --input-bg: #ffffff;
     --input-border: rgba(0, 0, 0, 0.15);
     --input-text: #0f0f0f;
     --accent-color: #396cd8;
     --space-2: 0.6em;
     --space-4: 1rem;
     --space-6: 1.5rem;
     --radius-sm: 8px;
     --radius-md: 12px;
     --shadow-sm: 0 2px 2px rgba(0, 0, 0, 0.2);
     --shadow-card: 0 6px 18px rgba(15, 15, 15, 0.08);
   }

   @media (prefers-color-scheme: dark) {
     :root {
       --bg-color: #101015;
       --text-color: #f6f6f6;
       --card-bg: rgba(255, 255, 255, 0.05);
       --card-border: rgba(255, 255, 255, 0.12);
       --input-bg: rgba(255, 255, 255, 0.1);
       --input-border: rgba(255, 255, 255, 0.25);
       --input-text: #f6f6f6;
     }
   }
   ```
3. 既存`src/App.css`との差分を確認（変数名が一致）

**検証**:
- 全8個のCSS変数が正確に出力される
- ライト/ダークモードの値が正確
- 変数名が既存`src/App.css`と一致

_Requirements: REQ-006_

---

### タスク2.4: トークンファイル監視機能の実装
**目的**: トークンJSON編集時に自動でCSS再生成

**成果物**:
- `tokens:watch`スクリプトの動作確認

**実装内容**:
1. `pnpm tokens:watch`をバックグラウンド実行
2. `tokens/base.tokens.json`を編集（例: `color.accent.primary`の値変更）
3. 5秒以内に`src/styles/tokens.css`が再生成されることを確認
4. JSON構文エラー時のエラーメッセージを確認

**検証**:
- ファイル変更から再生成まで5秒以内
- JSON構文エラー時にプロセスが継続（クラッシュしない）

_Requirements: REQ-007_

---

## Phase 3: Storybook統合とコンポーネント可視化

### タスク3.1: Storybook設定ファイル作成
**目的**: Storybookの基本設定とtokens.css読み込み

**成果物**:
- `ui-hub/.storybook/main.ts`
- `ui-hub/.storybook/preview.ts`

**実装内容**:
1. `.storybook/main.ts`作成:
   ```typescript
   import type { StorybookConfig } from '@storybook/react-vite';

   const config: StorybookConfig = {
     stories: ['../src/stories/**/*.stories.@(ts|tsx)'],
     addons: [
       '@storybook/addon-essentials',
       '@storybook/addon-a11y'
     ],
     framework: {
       name: '@storybook/react-vite',
       options: {}
     }
   };

   export default config;
   ```
2. `.storybook/preview.ts`作成:
   ```typescript
   import type { Preview } from '@storybook/react';
   import '../src/styles/tokens.css';

   const preview: Preview = {
     parameters: {
       actions: { argTypesRegex: '^on[A-Z].*' },
       controls: {
         matchers: {
           color: /(background|color)$/i,
           date: /Date$/
         }
       }
     }
   };

   export default preview;
   ```

**検証**:
- `pnpm sb`が起動
- ブラウザでlocalhost:6006にアクセス可能

_Requirements: REQ-005_

---

### タスク3.2: RecordButtonコンポーネント実装
**目的**: 既存UI (`src/App.tsx` L239-244) のRecordButton機能を独立コンポーネントとして再実装

**成果物**:
- `ui-hub/src/components/RecordButton.tsx`
- `ui-hub/src/components/RecordButton.module.css`

**実装内容**:
1. コンポーネントAPI定義:
   ```typescript
   export interface RecordButtonProps {
     state: 'idle' | 'recording' | 'disabled';
     onClick: () => void;
     label?: string;
   }

   export const RecordButton: React.FC<RecordButtonProps> = ({
     state,
     onClick,
     label
   }) => {
     const buttonLabel = label || (state === 'recording' ? 'Recording...' : 'Start Recording');
     const disabled = state === 'disabled';
     const className = state === 'recording' ? 'recording' : 'idle';

     return (
       <button
         className={className}
         onClick={onClick}
         disabled={disabled}
         aria-label={buttonLabel}
       >
         {buttonLabel}
       </button>
     );
   };
   ```
2. スタイリング（既存`src/App.css` L174-196を再現）:
   ```css
   button {
     border-radius: var(--radius-sm);
     padding: var(--space-2) var(--space-4);
     background-color: var(--accent-color);
     color: #ffffff;
     border: none;
     cursor: pointer;
   }

   button:disabled {
     opacity: 0.35;
     cursor: not-allowed;
   }

   button.recording {
     /* 録音中の追加スタイル */
   }
   ```

**検証**:
- 3状態（Idle/Recording/Disabled）が正常に表示される
- CSS変数が正しく参照される

_Requirements: REQ-005_

---

### タスク3.3: RecordButtonストーリー作成
**目的**: Storybookで各状態を可視化

**成果物**:
- `ui-hub/src/stories/RecordButton.stories.tsx`

**実装内容**:
1. ストーリーファイル作成:
   ```typescript
   import type { Meta, StoryObj } from '@storybook/react';
   import { RecordButton } from '../components/RecordButton';

   const meta: Meta<typeof RecordButton> = {
     title: 'Components/RecordButton',
     component: RecordButton,
     tags: ['autodocs']
   };

   export default meta;
   type Story = StoryObj<typeof RecordButton>;

   export const Idle: Story = {
     args: {
       state: 'idle',
       onClick: () => console.log('Start recording'),
       label: 'Start Recording'
     }
   };

   export const Recording: Story = {
     args: {
       state: 'recording',
       onClick: () => console.log('Stop recording'),
       label: 'Recording...'
     }
   };

   export const Disabled: Story = {
     args: {
       state: 'disabled',
       onClick: () => console.log('Cannot click'),
       label: 'Start Recording'
     }
   };
   ```

**検証**:
- Storybookで3つのストーリーが表示される
- 各ストーリーでクリック動作が確認できる

_Requirements: REQ-005_

---

### タスク3.4: Storybook起動とHMR動作確認
**目的**: トークン変更がStorybookに即座に反映されることを確認

**成果物**:
- HMR動作確認レポート

**実装内容**:
1. `pnpm dev`で全プロセス並列起動
2. `tokens/base.tokens.json`の`color.accent.primary`を`#2563eb`に変更
3. Storybookでボタン色が即座に変わることを確認（5秒以内）
4. ライト/ダークモード切替（OSのシステム設定変更）で色が変わることを確認

**検証**:
- トークン変更からUI反映まで5秒以内
- ライト/ダークモード切替が正常動作

_Requirements: REQ-007, REQ-011_

---

## Phase 4: MCPサーバ実装とAI連携基盤構築

### タスク4.1: MCPサーバ骨格実装
**目的**: MCP TypeScript SDKを使用したstdioサーバの基本構造を作成

**成果物**:
- `ui-hub/scripts/mcp-server.ts`

**実装内容**:
1. MCPサーバ初期化:
   ```typescript
   import { Server } from '@modelcontextprotocol/sdk/server/index.js';
   import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

   const server = new Server(
     {
       name: 'ui-hub',
       version: '1.0.0'
     },
     {
       capabilities: {
         tools: {}
       }
     }
   );

   const transport = new StdioServerTransport();
   await server.connect(transport);
   ```
2. エラーハンドリングの実装
3. `pnpm mcp`で起動確認

**検証**:
- `pnpm mcp`が起動
- stdioで待ち受け状態

_Requirements: REQ-008_

---

### タスク4.2: list_storiesツール実装
**目的**: Storybookのストーリー一覧を取得

**成果物**:
- `list_stories`ツール実装

**実装内容**:
1. ツール定義:
   ```typescript
   server.setRequestHandler(ListToolsRequestSchema, async () => {
     return {
       tools: [
         {
           name: 'list_stories',
           description: 'Get all Storybook stories',
           inputSchema: {
             type: 'object',
             properties: {}
           }
         }
       ]
     };
   });
   ```
2. ハンドラ実装:
   ```typescript
   server.setRequestHandler(CallToolRequestSchema, async (request) => {
     if (request.params.name === 'list_stories') {
       try {
         const res = await fetch('http://localhost:6006/index.json');
         if (!res.ok) {
           throw new Error('Storybook not running on port 6006');
         }
         const data = await res.json();
         const stories = Object.entries(data.entries).map(([id, entry]: any) => ({
           id,
           title: entry.title,
           kind: entry.type
         }));
         return { content: [{ type: 'text', text: JSON.stringify(stories) }] };
       } catch (error) {
         return { content: [{ type: 'text', text: `Error: ${error.message}` }], isError: true };
       }
     }
   });
   ```

**検証**:
- Storybook起動中に`list_stories`を実行してストーリー一覧が取得できる
- Storybook未起動時にエラーメッセージが返る

_Requirements: REQ-008_

---

### タスク4.3: get_story_url, get_tokensツール実装
**目的**: ストーリーURLとトークン情報を取得

**成果物**:
- `get_story_url`, `get_tokens`ツール実装

**実装内容**:
1. `get_story_url`ツール:
   ```typescript
   {
     name: 'get_story_url',
     description: 'Get iframe URL for a story',
     inputSchema: {
       type: 'object',
       properties: {
         id: { type: 'string', description: 'Story ID' }
       },
       required: ['id']
     }
   }
   // Handler: return `http://localhost:6006/iframe.html?id=${id}`
   ```
2. `get_tokens`ツール:
   ```typescript
   {
     name: 'get_tokens',
     description: 'Get tokens.css and tokens JSON',
     inputSchema: { type: 'object', properties: {} }
   }
   // Handler:
   // const css = await fs.readFile('ui-hub/src/styles/tokens.css', 'utf-8');
   // const json = await fs.readFile('tokens/base.tokens.json', 'utf-8');
   // return {css, tokens: JSON.parse(json)};
   ```

**検証**:
- `get_story_url`が正しいiframe URLを返す
- `get_tokens`がCSS内容とトークンJSONを返す
- tokens.css不在時にエラーメッセージが返る

_Requirements: REQ-008_

---

## Phase 5: プロセス管理と並列起動実装

### タスク5.1: devスクリプト実装と並列起動確認
**目的**: `pnpm dev`で全プロセスを並列起動

**成果物**:
- `dev`スクリプトの動作確認

**実装内容**:
1. `package.json`の`dev`スクリプト確認:
   ```json
   {
     "dev": "pnpm tokens:build && run-p sb tokens:watch mcp"
   }
   ```
2. `pnpm dev`実行
3. 3プロセス（Storybook, tokens:watch, MCP）が並列起動することを確認
4. Ctrl+Cで一括終了することを確認

**検証**:
- 3プロセスが同時に起動
- 初回ビルドが必ず実行される
- 一括終了が正常動作

_Requirements: REQ-010_

---

### タスク5.2: プロセスクラッシュ時の動作確認
**目的**: 1プロセスがクラッシュしても他が継続動作することを確認

**成果物**:
- クラッシュ動作確認レポート

**実装内容**:
1. `pnpm dev`起動中に`tokens/base.tokens.json`に構文エラーを挿入
2. `tokens:watch`プロセスがエラーログを出力し、watchは継続
3. Storybookが引き続き動作することを確認
4. JSON修正後に自動復旧することを確認

**検証**:
- 1プロセスのエラーが他プロセスに影響しない
- エラーログが明確に出力される

_Requirements: REQ-010_

---

## Phase 6: 本体適用準備

### タスク6.1: 本体適用手順ドキュメント作成
**目的**: ui-hub成果物を本体に統合する明確な手順を文書化

**成果物**:
- `ui-hub/INTEGRATION.md`

**実装内容**:
1. ステップバイステップの適用手順:
   ```markdown
   # UI Hub成果物の本体適用手順

   ## 前提条件
   - ui-hubでの開発が完了していること
   - Storybookで全コンポーネントの動作確認済み
   - テスト実行済み

   ## 手順1: tokens.cssを本体に統合
   1. `ui-hub/src/styles/tokens.css`の内容をコピー
   2. `src/App.css`のL8-28（ライトモード変数）を置き換え
   3. `src/App.css`のL230-256（ダークモード変数）を置き換え
   4. ハードコード値をCSS変数に置き換え:
      - `border-radius: 8px` → `var(--radius-sm)`
      - `padding: 1.5rem` → `var(--space-6)`
      - 等

   ## 手順2: コンポーネントを本体に移行
   1. `src/components/`ディレクトリを作成
   2. `ui-hub/src/components/RecordButton.tsx`を`src/components/`にコピー
   3. `src/App.tsx`でインポート:
      ```typescript
      import { RecordButton } from './components/RecordButton';
      ```
   4. `src/App.tsx` L239-244を`<RecordButton>`に置き換え

   ## 手順3: Tauriアプリで動作確認
   1. `pnpm tauri dev`で起動
   2. 全機能が正常動作することを確認
   3. ライト/ダークモード切替を確認
   4. レスポンシブ動作を確認

   ## 手順4: 統合テスト実行
   1. Tauriアプリの全E2Eテストを実行
   2. 視覚的回帰テストを実行（任意）
   3. パフォーマンステストを実行（起動時間、レンダリング性能）

   ## 手順5: コミット
   1. `git add src/App.css src/components/ src/App.tsx`
   2. `git commit -m "feat(ui): デザイントークン駆動UIに移行"`
   3. PRを作成し、レビュー依頼
   ```

**検証**:
- 手順が明確で実行可能
- ロールバック手順も含まれる

_Requirements: REQ-009_

---

### タスク6.2: 本体適用シミュレーション
**目的**: 実際に本体に適用し、動作確認

**成果物**:
- 動作確認レポート

**実装内容**:
1. 別ブランチで本体適用を実施
2. `src/App.css`を更新
3. `pnpm tauri dev`で起動確認
4. 全機能が正常動作することを確認
5. スクリーンショットで既存UIとの差分を確認（視覚的に同じであることを確認）

**検証**:
- Tauriアプリが正常起動
- 既存機能が全て正常動作
- ライト/ダークモード切替が正常動作
- 視覚的に既存UIと同じ

_Requirements: REQ-009, REQ-011_

---

## Phase 7: テスト実装

### タスク7.1: Style Dictionary設定のユニットテスト
**目的**: トークンJSON → CSS変換が正確であることを検証

**成果物**:
- `ui-hub/tests/tokens.test.ts`

**実装内容**:
1. テストケース作成:
   ```typescript
   import { readFileSync } from 'fs';
   import { describe, test, expect } from 'vitest';

   describe('tokens.css generation', () => {
     test('全8個のCSS変数が出力される', () => {
       const css = readFileSync('src/styles/tokens.css', 'utf-8');
       expect(css).toContain('--bg-color');
       expect(css).toContain('--text-color');
       expect(css).toContain('--card-bg');
       expect(css).toContain('--card-border');
       expect(css).toContain('--input-bg');
       expect(css).toContain('--input-border');
       expect(css).toContain('--input-text');
       expect(css).toContain('--accent-color');
     });

     test('ダークモード変数が@media内に出力される', () => {
       const css = readFileSync('src/styles/tokens.css', 'utf-8');
       expect(css).toContain('@media (prefers-color-scheme: dark)');
     });
   });
   ```

**検証**:
- 全テストが合格

_Requirements: REQ-011_

---

### タスク7.2: RecordButtonコンポーネントのユニットテスト
**目的**: コンポーネントの状態切替が正確であることを検証

**成果物**:
- `ui-hub/tests/RecordButton.test.tsx`

**実装内容**:
1. React Testing Libraryでテスト作成:
   ```typescript
   import { render, screen, fireEvent } from '@testing-library/react';
   import { RecordButton } from '../src/components/RecordButton';

   describe('RecordButton', () => {
     test('Idle状態でボタンが有効', () => {
       const onClick = vi.fn();
       render(<RecordButton state="idle" onClick={onClick} />);
       const button = screen.getByRole('button');
       expect(button).not.toBeDisabled();
       fireEvent.click(button);
       expect(onClick).toHaveBeenCalled();
     });

     test('Recording状態でラベルが変わる', () => {
       render(<RecordButton state="recording" onClick={() => {}} />);
       expect(screen.getByText('Recording...')).toBeInTheDocument();
     });

     test('Disabled状態でボタンが無効', () => {
       render(<RecordButton state="disabled" onClick={() => {}} />);
       const button = screen.getByRole('button');
       expect(button).toBeDisabled();
     });
   });
   ```

**検証**:
- 全テストが合格

_Requirements: REQ-011_

---

### タスク7.3: 統合テスト - トークン更新フローE2E
**目的**: トークン編集 → CSS再生成 → HMR反映のフロー全体を検証

**成果物**:
- `ui-hub/tests/integration/token-update-flow.test.ts`

**実装内容**:
1. E2Eテスト作成:
   ```typescript
   import { execSync } from 'child_process';
   import { readFileSync, writeFileSync } from 'fs';

   describe('Token Update Flow E2E', () => {
     test('トークン編集からCSS再生成まで5秒以内', async () => {
       const tokensPath = 'tokens/base.tokens.json';
       const cssPath = 'src/styles/tokens.css';

       const beforeMtime = statSync(cssPath).mtimeMs;

       // トークン編集
       const tokens = JSON.parse(readFileSync(tokensPath, 'utf-8'));
       tokens.color.accent.primary.value = '#2563eb';
       writeFileSync(tokensPath, JSON.stringify(tokens, null, 2));

       // 5秒待機
       await new Promise(resolve => setTimeout(resolve, 5000));

       const afterMtime = statSync(cssPath).mtimeMs;
       expect(afterMtime).toBeGreaterThan(beforeMtime);

       // CSSが更新されていることを確認
       const css = readFileSync(cssPath, 'utf-8');
       expect(css).toContain('#2563eb');
     });
   });
   ```

**検証**:
- テストが5秒以内に完了
- CSS更新が確認できる

_Requirements: REQ-011_

---

## Phase 8: 最終統合と完了

### タスク8.1: 全機能の統合確認
**目的**: 全フェーズの成果物が正常に動作することを確認

**成果物**:
- 統合確認チェックリスト

**実装内容**:
1. チェックリスト実行:
   - [ ] `pnpm dev`で全プロセス起動
   - [ ] Storybookでストーリー表示
   - [ ] トークン編集でUI即座に更新
   - [ ] MCP 3コマンドが正常動作
   - [ ] 本体適用シミュレーションが成功
   - [ ] 全テストが合格

**検証**:
- 全チェック項目がクリア

_Requirements: REQ-011_

---

### タスク8.2: README.md作成
**目的**: ui-hubの使い方を文書化

**成果物**:
- `ui-hub/README.md`

**実装内容**:
1. README作成:
   ```markdown
   # UI Hub - Meeting Minutes Automator Design System Development Environment

   ## 概要
   Meeting Minutes Automatorの既存UIをデザイントークン駆動に移行するための開発環境

   ## セットアップ
   ```bash
   cd ui-hub
   pnpm install
   pnpm tokens:build
   ```

   ## 開発
   ```bash
   pnpm dev  # Storybook + tokens:watch + MCP並列起動
   ```

   ## トークン編集
   1. `tokens/base.tokens.json`を編集
   2. 5秒以内にStorybookに反映

   ## 本体適用
   詳細は`INTEGRATION.md`参照

   ## テスト
   ```bash
   pnpm test
   ```
   ```

**検証**:
- READMEが明確で実行可能

_Requirements: REQ-011_

---

### タスク8.3: spec.json更新と完了報告
**目的**: 仕様ドキュメントのフェーズを更新

**成果物**:
- `.kiro/specs/ui-hub/spec.json`更新

**実装内容**:
1. `spec.json`を更新:
   ```json
   {
     "phase": "implementation-complete",
     "approvals": {
       "tasks": {
         "generated": true,
         "approved": true
       }
     },
     "ready_for_deployment": true
   }
   ```
2. 完了報告作成

**検証**:
- 全タスクが完了
- 本体適用準備完了

---

## 実装優先度サマリー

### 🔴 Phase 0-2: 基盤とトークンパイプライン（必須）
- タスク0.1-0.2: 既存UI分析
- タスク1.1-1.2: プロジェクト初期化
- タスク2.1-2.4: トークンパイプライン構築

### 🟡 Phase 3: Storybook統合（コア機能）
- タスク3.1-3.4: Storybookセットアップとコンポーネント実装

### 🟢 Phase 4-5: MCP・プロセス管理（付加機能）
- タスク4.1-4.3: MCPサーバ実装
- タスク5.1-5.2: 並列起動実装

### 🔵 Phase 6-8: 本体適用・テスト・完了（仕上げ）
- タスク6.1-6.2: 本体適用準備
- タスク7.1-7.3: テスト実装
- タスク8.1-8.3: 最終統合

---

## 注意事項

1. **本体への影響を最小化**: `src/App.tsx`のロジックは変更しない。スタイリングのみを改善。
2. **既存CSS変数名との互換性**: `--bg-color`等の既存変数名を維持。
3. **段階的な本体適用**: ui-hubで開発完了後、別ブランチで本体適用を実施。
4. **ロールバック可能性**: 本体適用時は必ずバックアップを取得。
