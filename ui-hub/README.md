# UI Hub - Design Token-Driven Development Environment

Meeting Minutes Automatorの既存UIを改善するためのトークン駆動開発環境。

## 概要

- **目的**: `src/App.css`の8つのCSS変数をDTCG形式トークンに変換し、Storybookで改善・開発
- **親プロジェクト**: [meeting-minutes-automator](../)
- **統合フロー**: ui-hubで開発 → `../src/`に統合

## セットアップ

```bash
# 依存関係インストール
pnpm install

# トークンビルド
pnpm tokens:build

# Storybook起動
pnpm sb

# 全プロセス並列起動（Storybook + tokens:watch + MCP）
pnpm dev
```

## アーキテクチャ

### Style Dictionary設定（`sd.config.js`）

#### 設計判断

**TypeScript型定義のみ生成（JavaScript実装なし）**:
- `ts`プラットフォームは`typescript/es6-declarations`のみ使用
- **理由**: トークンはCSS変数として使用（`var(--bg-color)`）、JavaScriptランタイムでのインポートは不要
- **将来的な拡張**: SSRや動的計算が必要になった場合、`typescript/es6`または`javascript/es6`を追加

**グローバル登録パターン**:
- `StyleDictionary.registerTransform/Format`でカスタム変換を登録
- **理由**: シンプルな単一パイプライン設計
- **将来的な移行**: Phase 3でwatch動作確認時に`StyleDictionary.extend()`への移行を検討

### Phase 3実装時の確認事項

#### ✅ 必須確認: `pnpm tokens:watch`の動作検証

**手順**:
```bash
# 1. watch起動
pnpm tokens:watch

# 2. トークンファイル編集
vi tokens/base.tokens.json
# （任意の値を変更して保存）

# 3. 2回目のビルドでエラー確認
# ✅ 正常: "Build completed successfully"
# ❌ エラー: "Error: Transform 'name/css/legacy' already registered"
```

**エラーが出た場合の対処法**:

`sd.config.js`を以下のように修正:

```javascript
import StyleDictionary from 'style-dictionary';

// ❌ 修正前（グローバル登録）
StyleDictionary.registerTransform({ name: 'name/css/legacy', ... });
StyleDictionary.registerFormat({ name: 'css/variables-with-dark-mode', ... });
export default { source: [...], platforms: {...} };

// ✅ 修正後（factory pattern）
export default new StyleDictionary({
  source: ['tokens/**/*.tokens.json'],
  platforms: {...},
  transform: {
    'name/css/legacy': {
      type: 'name',
      transform: (token) => {...}
    }
  },
  format: {
    'css/variables-with-dark-mode': {
      format: ({ dictionary }) => {...}
    }
  }
});
```

**注意**: Style Dictionary v4の`extend()`APIは2025年1月時点でドキュメントが不完全。実装前に公式リポジトリの最新issuesを確認すること。

---

## トークン構造

### 既存CSS変数マッピング（`src/App.css` → `tokens/base.tokens.json`）

| 既存CSS変数 | トークンパス | light | dark |
|------------|-------------|-------|------|
| `--bg-color` | `color.bg.{light\|dark}` | `#f6f6f6` | `#101015` |
| `--text-color` | `color.text.{light\|dark}` | `#0f0f0f` | `#f6f6f6` |
| `--card-bg` | `color.card.bg.{light\|dark}` | `#ffffff` | `rgba(255,255,255,0.05)` |
| `--card-border` | `color.card.border.{light\|dark}` | `rgba(0,0,0,0.08)` | `rgba(255,255,255,0.12)` |
| `--input-bg` | `color.input.bg.{light\|dark}` | `#ffffff` | `rgba(255,255,255,0.1)` |
| `--input-border` | `color.input.border.{light\|dark}` | `rgba(0,0,0,0.15)` | `rgba(255,255,255,0.25)` |
| `--input-text` | `color.input.text.{light\|dark}` | `#0f0f0f` | `#f6f6f6` |
| `--accent-color` | `color.accent.primary` | `#396cd8` | `#396cd8` |

### 追加トークン

- **spacing**: `space.2`, `space.4`, `space.6`
- **border-radius**: `radius.sm`, `radius.md`
- **box-shadow**: `shadow.card`, `shadow.sm`
- **danger/warning**: `color.danger.primary`, `color.warning.primary`

---

## 開発ワークフロー

1. **トークン編集**: `tokens/base.tokens.json`を編集
2. **自動ビルド**: chokidarが変更検知 → `pnpm tokens:build`実行
3. **UI即反映**: Storybook HMRで5秒以内に反映
4. **統合**: `src/styles/tokens.css` → `../src/App.css`に適用

---

## トラブルシューティング

### `[object Object]`がCSS変数に出力される

**原因**: shadowトークンに`shadow/css`変換が適用されていない

**確認**:
```javascript
// sd.config.js L62
transforms: ['attribute/cti', 'name/css/legacy', 'size/px', 'color/css', 'shadow/css'],
//                                                                        ↑ これがあるか確認
```

### `ui-hub/ui-hub/src/styles/`に出力される

**原因**: buildPathが絶対パスになっている

**確認**:
```javascript
// sd.config.js L63, L73
buildPath: 'src/styles/',  // ✅ 正しい（相対パス）
buildPath: 'ui-hub/src/styles/',  // ❌ 間違い（二重ネスト）
```

---

## 参照

- 仕様書: `../.kiro/specs/ui-hub/`
- 設計原則: `../.kiro/steering/principles.md`
- 親プロジェクト: `../README.md`
