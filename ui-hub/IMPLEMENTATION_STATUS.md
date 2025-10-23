# UI Hub 実装ステータス

**最終更新**: 2025-10-24
**現在のフェーズ**: Phase 2完了 → Phase 3実装準備

---

## 完了フェーズ

### Phase 0: 既存UI分析と準備 ✅
- タスク0.1: 既存UIトークン抽出とマッピング表作成 ✅
- タスク0.2: 既存UIコンポーネント構造分析 ✅

### Phase 1: 基盤セットアップ ✅
- タスク1.1: ui-hub/プロジェクト初期化 ✅
- タスク1.2: 基本ディレクトリ構造作成 ✅

**成果物**:
- `ui-hub/package.json` - ESM構成（`"type": "module"`）
- `ui-hub/sd.config.js` - Style Dictionary v4設定（ESM構文）
- ディレクトリ構造: tokens/, src/, .storybook/, scripts/

### Phase 2: デザイントークンパイプライン構築 ✅
- タスク2.1: tokens/base.tokens.json作成 ✅
- タスク2.2: Style Dictionary設定作成 ✅

**成果物**:
- `ui-hub/tokens/base.tokens.json` (166行) - DTCG準拠、単一ファイル構造
- `ui-hub/sd.config.js` - カスタムtransform/format実装

**重要な技術判断**:

#### 1. Style Dictionary v4 API - `transformer` vs `transform`
**結論**: `transformer`を使用（ユーザーの実証テストに基づく）

```javascript
// ui-hub/sd.config.js L7
StyleDictionary.registerTransform({
  name: 'name/css/legacy',
  type: 'name',
  transformer: (token) => {  // v4 still expects transformer
    // 8つの既存CSS変数名マッピング
  }
});
```

**経緯**:
- 当初、Style Dictionary v4ドキュメントから`transform`が新APIと判断
- design.mdとtasks.mdを`transform`に修正
- ユーザーが実証テストで`transformer`が動作することを確認
- Web検証で`transform`は新API、`transformer`は後方互換で残存と判明
- 最終判断: 実証結果を優先し`transformer`を採用

#### 2. buildPath - 二重ネスト問題
**結論**: `buildPath: 'src/styles/'`（相対パス、ui-hub/基準）

```javascript
// ui-hub/sd.config.js L63, L73
platforms: {
  css: {
    buildPath: 'src/styles/',  // NOT 'ui-hub/src/styles/'
  },
  ts: {
    buildPath: 'src/styles/',
  }
}
```

**理由**: `pnpm tokens:build`はui-hub/ディレクトリから実行されるため、`'ui-hub/src/styles/'`では`ui-hub/ui-hub/src/styles/`となる。

#### 3. shadow/css Transform追加
**結論**: transforms配列に`'shadow/css'`を明示的に追加

```javascript
// ui-hub/sd.config.js L62
transforms: ['attribute/cti', 'name/css/legacy', 'size/px', 'color/css', 'shadow/css'],
```

**理由**: shadow tokenは`{offsetX, offsetY, blur, spread, color}`構造を持ち、専用transformがないと`[object Object]`になる。

#### 4. DTCG Format - "v2.0"は存在しない
**結論**: DTCG W3C Draft Spec（v1.0.0に向けた作業仕様）を使用

```json
{
  "$schema": "https://design-tokens.github.io/community-group/spec/packages/format/latest/schema.json",
  "color": {
    "bg": {
      "light": { "$type": "color", "$value": "#f6f6f6", "$description": "..." }
    }
  }
}
```

**理由**: 2025年9月時点でDTCGはv1.0.0を目指しており、v2.0は存在しない。

#### 5. Token構造 - 単一ファイル vs 複数ファイル
**結論**: 単一ファイル（`base.tokens.json`）にlight/darkをsuffixとして含める

```json
{
  "color": {
    "bg": {
      "light": { "$type": "color", "$value": "#f6f6f6" },
      "dark": { "$type": "color", "$value": "#101015" }
    }
  }
}
```

**理由**:
- Style Dictionaryのsource配列はdeep mergeする
- base.json + light.json + dark.jsonを同時読込すると、後から読んだファイルが前のファイルを上書き
- 既存の`css/variables-with-dark-mode` formatはlight/dark suffixを前提に実装済み

#### 6. Shadow Token DTCG構造
**結論**: オブジェクト形式で定義

```json
{
  "shadow": {
    "$type": "shadow",
    "card": {
      "$value": {
        "offsetX": "0px",
        "offsetY": "6px",
        "blur": "18px",
        "spread": "0px",
        "color": "rgba(15, 15, 15, 0.08)"
      }
    }
  }
}
```

**理由**: DTCG shadow typeはCSS shorthand文字列ではなく、構造化オブジェクトを要求。

---

## Phase 2最終レビュー検証結果

### Issue 1: size/px Transform問題（誤指摘）
**指摘**: `size/px`がem/remを固定ピクセルに変換

**検証結果**: ❌ **誤り**

Style Dictionary内部実装:
```javascript
if (token.value.endsWith('px') || token.value.endsWith('em') || token.value.endsWith('rem')) {
  return token.value; // 単位付きはそのまま
}
return `${token.value}px`; // 単位なしのみpx追加
```

**結論**: `"0.6em"` → `0.6em`（変換されない）。修正不要。

### Issue 2: TypeScript Build問題（設計意図内）
**指摘**: `.d.ts`のみでruntime exportがない。light/darkオブジェクトを手動構築必要。

**検証結果**: ⚠️ **技術的には正しいが、設計意図と矛盾しない**

**設計意図（requirements.md REQ-001.2.a）**:
> "CSS変数（--bg-color等）を生成し、Reactコンポーネントから`style={{}}`やCSS-in-JSで参照可能にする"

**現在の設計**: CSS Variables First
- `tokens.css`を生成してReactで`var(--bg-color)`として使用
- `.d.ts`は型ヒントのみ提供（runtime exportは不要）

**指摘者の提案**: Runtime export追加（`export const light = {...}; export const dark = {...};`）

**判断**:
- Phase 2の範囲内では修正不要
- 動的テーマ切替が必要になるPhase 4以降で再検討
- 現時点ではCSS変数+prefers-color-schemeで十分

---

## 次のステップ: Phase 3実装

### タスク2.3: トークンビルド実行と検証（未実施）
**実行コマンド**:
```bash
cd ui-hub
pnpm install
pnpm tokens:build
```

**期待される成果物**:
- `ui-hub/src/styles/tokens.css`
- `ui-hub/src/styles/tokens.d.ts`

**検証ポイント**:
- 全8個のCSS変数が正確に出力される
- ライト/ダークモードの値が正確
- 変数名が既存`src/App.css`と一致

### タスク2.4: トークンファイル監視機能の実装（未実施）
**実行コマンド**:
```bash
pnpm tokens:watch
# 別ターミナルでtokens/base.tokens.jsonを編集
```

**Phase 3実装時の重要注意事項**:
`.kiro/specs/ui-hub/README.md`に記載の確認手順を必ず実施:

```bash
pnpm tokens:watch
# トークンファイル編集（2回目のビルドを発生させる）
# エラーチェック:
# ✅ "Build completed successfully" → 問題なし
# ❌ "Error: Transform 'name/css/legacy' already registered" → 対処法参照
```

**重複登録エラーが発生した場合の対処法**:
1. `sd.config.js`の`registerTransform/registerFormat`をif文で保護
2. Style Dictionary v4のバグレポート確認
3. watch modeではなく手動ビルドに切り替え検討

---

## 技術スタック記録

### 依存関係（ui-hub/package.json）
```json
{
  "type": "module",
  "dependencies": {
    "react": "^19.1.0",
    "react-dom": "^19.1.0"
  },
  "devDependencies": {
    "style-dictionary": "^4.0.0",
    "@storybook/react": "^8.0.0",
    "chokidar-cli": "^3.0.0",
    "@modelcontextprotocol/sdk": "0.6.0",
    "npm-run-all2": "^6.0.0",
    "tsx": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }
}
```

### npm Scripts
```json
{
  "sb": "storybook dev -p 6006",
  "tokens:build": "style-dictionary build -c sd.config.js",
  "tokens:watch": "chokidar \"tokens/**/*.json\" -c \"pnpm tokens:build\"",
  "mcp": "tsx scripts/mcp-server.ts",
  "dev": "pnpm tokens:build && run-p sb tokens:watch mcp"
}
```

---

## Agent定義の更新履歴

### context-scout.md ✅
**追加内容**: Code-diagram alignment checks

```markdown
1. **Targeted Consistency Verification**
   - **Verify code-diagram alignment**: Check if code structure changes are reflected in diagrams

2. **Consistency Check** (2-3 minutes)
   - **Code-diagram sync check**: Use `mcp__serena__get_symbols_overview`

**For Code Files**:
- **Code-diagram alignment**: Use Serena to check if new classes/functions are in diagrams
- **Escalate to docs-gardener** if 3+ symbols need updates
```

### docs-gardener.md ✅
**追加内容**: Diagram drift detection

```markdown
### Diagram & UML Data Sources
- **`docs/uml/`**: PlantUML/Mermaid diagrams
- **ADRs**: Architecture Decision Records with embedded diagrams
- **Code structure**: Use `mcp__serena__get_symbols_overview`

3. **[Identify]** Detect:
   - **Diagram Drift**: Code structure not reflected in diagrams

5. **[Update Diagrams]** Synchronize:
   - Use Serena (`get_symbols_overview`, `find_symbol`)
   - Update `docs/uml/*.puml` or `design.md` Mermaid diagrams
```

### validate-gap.md ✅
**追加内容**: Code-Diagram Alignment Check section

```markdown
**Code-Diagram Alignment Check**:
- Use Serena to extract actual class/function structure
- Compare with diagrams in `docs/uml/`, ADRs, `design.md`
- Identify missing/obsolete diagram elements

#### Code-Diagram Alignment Status
- **Diagram Drift Detected**: List classes/functions missing from diagrams
- **Obsolete Diagram Elements**: List diagram components no longer in codebase
```

---

## 次のセッション引き継ぎ事項

1. **Phase 3開始前の確認**:
   - `ui-hub/package.json`が存在（Phase 1完了）
   - `ui-hub/tokens/base.tokens.json`が存在（Phase 2完了）
   - `ui-hub/sd.config.js`が`transformer`を使用

2. **Phase 3実行順序**:
   1. `cd ui-hub && pnpm install`
   2. `pnpm tokens:build` → 成果物確認
   3. `pnpm tokens:watch` → 重複登録エラーチェック
   4. Storybook設定作成（タスク3.1）

3. **Phase 3以降で注意すべき点**:
   - Watch mode重複登録エラーが発生する可能性（README.md記載の対処法参照）
   - TypeScript runtime export不要（CSS Variables Firstの設計）
   - 本体適用時は別ブランチで実施（ロールバック可能性確保）

4. **spec.json更新タイミング**:
   - Phase 2完了: `spec.json`の`phase`フィールドは現在`"ready-for-implementation"`
   - Phase 8完了時: `"implementation-complete"`に更新
