# Serena + cc-sdd Integration Workflow

## 概要
本ドキュメントは、Serena（象徴的コードナビゲーション）とcc-sdd（Kiro仕様駆動開発）を効果的に統合するための詳細ガイドです。

## 基本哲学

### Serenaの役割
- **トークン効率的なコード探索**: ファイル全体を読まずに必要な部分だけを取得
- **象徴ベースの編集**: 関数、クラス、メソッド単位での精密な編集
- **依存関係の可視化**: コード間の参照関係を追跡

### cc-sddの役割
- **仕様駆動の開発**: 要件 → 設計 → タスク → 実装の一貫したフロー
- **トレーサビリティ維持**: 要件IDとコードの双方向リンク
- **品質保証**: 仕様との整合性検証

---

## カスタムエージェント統合

### kiro-spec-implementer エージェント

**場所**: `.claude/agents/kiro-spec-implementer.md`

**自動起動条件**:
- ユーザーが「タスクX.Xを実装して」と依頼
- `/kiro:spec-impl`コマンドが言及される
- 既存コードの修正で仕様整合性確認が必要な場合

**提供価値**:
- 🎯 **トークン効率**: Serenaで必要な部分のみ読み込み
- 📋 **トレーサビリティ**: REQ-### ↔ コードの自動リンク維持
- ✅ **品質保証**: 設計原則9項目の自動チェック
- 🔄 **TDD徹底**: RED → GREEN → REFACTOR強制

**エージェント vs 手動**:
- **エージェント使用**: ある程度AIに任せて効率化したい場合
- **手動（コマンド直接実行）**: 自分でハンドリングしながら進めたい場合

**使用例**:
```
User: タスク2.5を実装して
Agent: タスク2.5（デバイス切断検出と自動再接続）を実装します。
       Phase 1: 仕様確認（/kiro:spec-status + requirements.md）
       Phase 2: 既存コード理解（Serena symbolic tools）
       Phase 3: TDD実装（RED → GREEN → REFACTOR）
       Phase 4: 検証（/kiro:validate-design + tests）
```

---

## フェーズ別詳細ガイド

### Phase 1: 仕様作成フェーズ

**使用ツール**: cc-sddコマンドのみ

```bash
# 1. 要件定義
/kiro:spec-requirements <feature>

# 2. 要件検証
/kiro:validate-requirements <feature>

# 3. 設計作成
/kiro:spec-design <feature>

# 4. 設計検証
/kiro:validate-design <feature>

# 5. タスク生成
/kiro:spec-tasks <feature>

# 6. タスク検証
/kiro:validate-tasks <feature>
```

**重要**: このフェーズではSerenaツールは使用しません（コードがまだ存在しないため）。

**エージェント推奨**: `kiro-spec-guardian` （仕様一貫性チェック）

---

### Phase 2: 実装準備フェーズ

**目的**: コードベースの構造を理解し、実装戦略を立てる

#### Step 1: プロジェクト構造の把握
```python
# ルートディレクトリの確認
mcp__serena__list_dir(relative_path=".", recursive=False)

# サブディレクトリの確認
mcp__serena__list_dir(relative_path="src-tauri/src", recursive=True)
```

#### Step 2: 既存コードの象徴一覧取得
```python
# ファイル内の主要な象徴（関数、クラス等）の概要を取得
mcp__serena__get_symbols_overview(relative_path="src-tauri/src/audio.rs")
```

**出力例**:
```
- AudioDevice (Struct)
- FakeAudioDevice (Struct)
- AudioChunkCallback (TypeAlias)
```

#### Step 3: 実装タスクの確認
```bash
# 仕様ステータス確認
/kiro:spec-status <feature>

# 実装開始（TDD）
/kiro:spec-impl <feature> [task-numbers]
```

**エージェント推奨**: `kiro-spec-implementer` （実装自動化）

---

### Phase 3: 実装フェーズ

**原則**: Serenaで探索 → cc-sddで検証

**エージェント推奨**: `kiro-spec-implementer` （TDD実装 + 要件トレーサビリティ維持）

#### エージェント使用時の自動ワークフロー

エージェントは以下を自動実行：

1. **仕様確認** (1-2 thoughts):
   - `/kiro:spec-status` で現在位置確認
   - `tasks.md` から対象タスクの要件ID取得
   - `requirements.md` から受入条件読み取り

2. **既存コード理解** (2-3 thoughts via Serena):
   - `mcp__serena__get_symbols_overview` で関連ファイル構造把握
   - `mcp__serena__find_symbol` で編集対象特定
   - `mcp__serena__find_referencing_symbols` で影響範囲確認

3. **TDD実装** (3-5 thoughts):
   - RED: 失敗するテスト作成（要件ID含む）
   - GREEN: 最小実装でテスト緑化
   - REFACTOR: 設計原則との整合性確認

4. **検証** (1-2 thoughts):
   - `/kiro:validate-design` で仕様整合性確認
   - テスト実行（cargo test / pytest）
   - トレーサビリティ更新

#### 手動実装時のパターン

##### パターン A: 新規コード追加

1. **象徴的検索で挿入位置を特定**
```python
# クラスの概要を取得（bodyは含めない）
mcp__serena__find_symbol(
    name_path="AudioDeviceAdapter",
    relative_path="src-tauri/src/audio.rs",
    include_body=False,
    depth=1  # メソッド一覧も取得
)
```

2. **新しいメソッドを挿入**
```python
mcp__serena__insert_after_symbol(
    name_path="AudioDeviceAdapter/stop_capture",
    relative_path="src-tauri/src/audio.rs",
    body="""
    /// Pauses the current recording session
    /// Related requirement: STT-REQ-004.9
    pub async fn handle_device_disconnection(&mut self) -> Result<()> {
        self.state = DeviceState::Disconnected;
        Ok(())
    }
    """
)
```

3. **仕様との整合性確認**
```bash
/kiro:validate-design <feature>
```

##### パターン B: 既存コード修正

1. **編集対象の象徴を特定**
```python
# メソッド本体を取得
mcp__serena__find_symbol(
    name_path="AudioDeviceAdapter/start_capture",
    relative_path="src-tauri/src/audio.rs",
    include_body=True  # 本体を含める
)
```

2. **象徴本体を置換**
```python
mcp__serena__replace_symbol_body(
    name_path="AudioDeviceAdapter/start_capture",
    relative_path="src-tauri/src/audio.rs",
    body="""
    /// Starts audio capture with device validation
    /// Related requirement: STT-REQ-001.4, STT-REQ-004.1
    pub async fn start_capture(&mut self, device_id: &str) -> Result<()> {
        // Validate device exists (NEW)
        self.validate_device(device_id)?;
        
        self.stream = Some(self.adapter.open_stream()?);
        Ok(())
    }
    """
)
```

3. **影響範囲の確認**
```python
# start_captureを参照している箇所を検索
mcp__serena__find_referencing_symbols(
    name_path="start_capture",
    relative_path="src-tauri/src/audio.rs"
)
```

##### パターン C: 禁止パターンのチェック（ADR-001準拠）

```python
# Python側で音声録音ライブラリの使用を検索
mcp__serena__search_for_pattern(
    substring_pattern="import (sounddevice|pyaudio|wave)",
    relative_path="python-stt",
    paths_include_glob="**/*.py"
)
```

**期待**: 検索結果が空であることを確認（ADR-001違反チェック）

---

### Phase 4: レビュー・リファクタリングフェーズ

#### パターン A: 依存関係の可視化

```python
# AudioDeviceAdapterを参照している全ファイルを検索
mcp__serena__find_referencing_symbols(
    name_path="AudioDeviceAdapter",
    relative_path="src-tauri/src/audio.rs"
)
```

**使用場面**:
- リファクタリング前の影響範囲調査
- 循環依存の検出
- デッドコードの発見

#### パターン B: 設計原則の検証

```python
# Port/Adapterパターンの確認
mcp__serena__find_symbol(
    name_path="AudioDeviceAdapter",
    relative_path="src-tauri/src/audio_device_adapter.rs",
    include_body=True
)
```

**確認ポイント**:
- traitで抽象化されているか（設計原則5: ベンダーロックイン回避）
- 具体的な実装がAdapterに隔離されているか
- ADR-001準拠：録音責務がRust側のみか

#### パターン C: 要件トレーサビリティの確認

```bash
# 設計ドキュメントと実装の整合性確認
/kiro:validate-design <feature>

# タスクの完了状態確認
/kiro:spec-status <feature>
```

---

## ベストプラクティス

### DO: 推奨パターン

1. **概要から詳細へ**
   ```python
   # Good: まず概要を取得
   mcp__serena__get_symbols_overview("audio.rs")
   # 次に必要な象徴だけ詳細取得
   mcp__serena__find_symbol("AudioDeviceAdapter/start_capture", include_body=True)
   ```

2. **象徴単位での編集**
   ```python
   # Good: メソッド全体を置換
   mcp__serena__replace_symbol_body(name_path="Class/method", body="...")
   ```

3. **影響範囲の事前確認**
   ```python
   # Good: 変更前に参照箇所を確認
   mcp__serena__find_referencing_symbols(name_path="method")
   # その後、各参照箇所を更新
   ```

4. **要件IDの明記**
   ```rust
   /// Related requirements: STT-REQ-001.4, NFR-PERF-003
   pub fn process_audio(&self) -> Result<()> { ... }
   ```

5. **エージェント活用**
   ```
   # Good: 複雑なタスクはエージェントに任せる
   User: タスク2.5を実装して
   → kiro-spec-implementer エージェントが自動実行
   
   # Good: 簡単な修正は手動で
   User: この関数のログを追加して
   → 手動でSerenaツールを使用
   ```

### DON'T: 避けるべきパターン

1. **ファイル全体の読み込み**
   ```python
   # Bad: ファイル全体を読む
   Read(file_path="large_file.rs")
   
   # Good: 必要な象徴だけ取得
   mcp__serena__find_symbol("TargetClass", include_body=True)
   ```

2. **パターン検索の乱用**
   ```python
   # Bad: 象徴名が分かっているのにパターン検索
   mcp__serena__search_for_pattern(substring_pattern="AudioService")
   
   # Good: find_symbolを使う
   mcp__serena__find_symbol(name_path="AudioService")
   ```

3. **仕様確認の省略**
   ```python
   # Bad: 実装だけして仕様確認しない
   mcp__serena__replace_symbol_body(...)
   # コミット
   
   # Good: 仕様との整合性を確認
   mcp__serena__replace_symbol_body(...)
   /kiro:validate-design <feature>
   # その後コミット
   ```

4. **エージェントの過度な依存**
   ```
   # Bad: 簡単な修正もエージェントに丸投げ
   User: この変数名を変更して
   → エージェント起動はオーバーキル
   
   # Good: 状況に応じて使い分け
   User: この変数名を変更して
   → 手動で mcp__serena__replace_symbol_body 使用
   ```

---

## トラブルシューティング

### Q1: 象徴が見つからない
```python
# Symptom: find_symbolが空を返す
mcp__serena__find_symbol(name_path="MyClass")  # 見つからない

# Solution: 部分一致検索を試す
mcp__serena__find_symbol(
    name_path="MyClass",
    substring_matching=True
)
```

### Q2: 編集箇所が多すぎる
```python
# Symptom: 複数ファイルで同じ名前の象徴を編集したい

# Solution: relative_pathで絞り込む
mcp__serena__find_symbol(
    name_path="process",
    relative_path="src-tauri/src"  # このディレクトリ内のみ
)
```

### Q3: 要件との対応が不明
```bash
# Symptom: どの要件に対応するコードか分からない

# Solution: 設計ドキュメントを確認
/kiro:spec-status <feature>
# design.mdの該当セクションを読む
# 要件IDをコードコメントに追加
```

### Q4: エージェントが起動しない
```bash
# Symptom: kiro-spec-implementerが自動起動しない

# Solution 1: 明示的に言及
User: kiro-spec-implementerエージェントでタスク2.5を実装して

# Solution 2: コマンド形式で依頼
User: /kiro:spec-impl meeting-minutes-stt 2.5
```

---

## チェックリスト

### 実装前
- [ ] `/kiro:spec-status`で現在のフェーズを確認
- [ ] 関連要件ID（REQ-###、STT-REQ-###等）を把握
- [ ] `mcp__serena__get_symbols_overview`で既存構造を理解
- [ ] エージェント使用 vs 手動実装を判断

### 実装中
- [ ] `mcp__serena__find_symbol`で編集対象を特定
- [ ] 要件IDをコメントに含める（`/// Related requirement: REQ-###`）
- [ ] `mcp__serena__find_referencing_symbols`で影響範囲確認
- [ ] TDDサイクル（RED → GREEN → REFACTOR）を遵守

### 実装後
- [ ] `/kiro:validate-design`で仕様整合性確認
- [ ] Pre-commit hooksが成功（`check_forbidden_imports.py`等）
- [ ] コミットメッセージに要件IDを含める
- [ ] Requirement Traceability Matrix更新
- [ ] ADR準拠確認（ADR-001: Rust録音のみ、ADR-002: モデル配布等）

---

## プロジェクト固有のガイドライン

### Meeting Minutes Automatorでの実践

#### 現在の状況（2025-10-10）
- **MVP0 (meeting-minutes-core)**: ✅ 完了（44テスト合格）
- **MVP1 (meeting-minutes-stt)**: 🔵 実装中（タスク2.4完了）

#### 重要なADR
- **ADR-001**: Recording Responsibility（Rust側録音のみ、Python禁止）
- **ADR-002**: Model Distribution Strategy（HuggingFace Hub + バンドル）
- **ADR-003**: IPC Versioning（セマンティックバージョニング）
- **ADR-004**: Chrome Extension WebSocket Management（Content Script採用）

#### 設計原則チェック
実装時は必ず以下を確認：

| 原則 | チェック項目 | 確認方法 |
|-----|------------|---------|
| 1. プロセス境界の明確化 | 音声録音はRust側のみ？ | `check_forbidden_imports.py` |
| 2. オフラインファースト | ネットワークなしで動作？ | オフラインE2Eテスト |
| 5. ベンダーロックイン回避 | traitで抽象化？ | インターフェース確認 |
| 6. TDD原則 | テストファースト？ | RED → GREEN確認 |

---

## 参考リンク

- **CLAUDE.md**: 基本ワークフロー、エージェント使用例
- **coding_standards.md**: コーディング規約、テスト基準
- **design_principles.md**: 9つのコア設計原則
- **task_completion_workflow.md**: タスク完了手順
- **.claude/agents/kiro-spec-implementer.md**: 実装エージェント定義
- **.claude/agents/kiro-spec-guardian.md**: 仕様検証エージェント定義