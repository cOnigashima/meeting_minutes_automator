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

## フェーズ別詳細ガイド

### Phase 1: 仕様作成フェーズ（現在）

**使用ツール**: cc-sddコマンドのみ

```bash
# 1. 要件定義
/kiro:spec-requirements meeting-minutes-core

# 2. 要件検証
/kiro:validate-requirements meeting-minutes-core

# 3. 設計作成
/kiro:spec-design meeting-minutes-core

# 4. 設計検証
/kiro:validate-design meeting-minutes-core

# 5. タスク生成
/kiro:spec-tasks meeting-minutes-core

# 6. タスク検証
/kiro:validate-tasks meeting-minutes-core
```

**重要**: このフェーズではSerenaツールは使用しません（コードがまだ存在しないため）。

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
mcp__serena__get_symbols_overview(relative_path="src-tauri/src/services/audio_service.rs")
```

**出力例**:
```
- AudioService (Class)
  - new() (Method)
  - start_recording() (Method)
  - stop_recording() (Method)
```

#### Step 3: 実装タスクの確認
```bash
# 仕様ステータス確認
/kiro:spec-status meeting-minutes-core

# 実装開始（TDD）
/kiro:spec-impl meeting-minutes-core 1,2,3
```

---

### Phase 3: 実装フェーズ

**原則**: Serenaで探索 → cc-sddで検証

#### パターン A: 新規コード追加

1. **象徴的検索で挿入位置を特定**
```python
# クラスの概要を取得（bodyは含めない）
mcp__serena__find_symbol(
    name_path="AudioService",
    relative_path="src-tauri/src/services/audio_service.rs",
    include_body=False,
    depth=1  # メソッド一覧も取得
)
```

2. **新しいメソッドを挿入**
```python
mcp__serena__insert_after_symbol(
    name_path="AudioService/stop_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    body="""
    /// Pauses the current recording session
    /// Related requirement: REQ-003.2
    pub async fn pause_recording(&mut self) -> Result<()> {
        self.state = RecordingState::Paused;
        Ok(())
    }
    """
)
```

3. **仕様との整合性確認**
```bash
/kiro:validate-design meeting-minutes-core
```

#### パターン B: 既存コード修正

1. **編集対象の象徴を特定**
```python
# メソッド本体を取得
mcp__serena__find_symbol(
    name_path="AudioService/start_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    include_body=True  # 本体を含める
)
```

2. **象徴本体を置換**
```python
mcp__serena__replace_symbol_body(
    name_path="AudioService/start_recording",
    relative_path="src-tauri/src/services/audio_service.rs",
    body="""
    /// Starts a new recording session with device validation
    /// Related requirement: REQ-002.1, NFR-Perf-001
    pub async fn start_recording(&mut self, device_id: &str) -> Result<()> {
        // Validate device exists (NEW)
        self.validate_device(device_id)?;
        
        self.state = RecordingState::Recording;
        self.start_time = SystemTime::now();
        Ok(())
    }
    """
)
```

3. **影響範囲の確認**
```python
# start_recordingを参照している箇所を検索
mcp__serena__find_referencing_symbols(
    name_path="start_recording",
    relative_path="src-tauri/src/services/audio_service.rs"
)
```

#### パターン C: 禁止パターンのチェック

```python
# Python側で音声録音ライブラリの使用を検索
mcp__serena__search_for_pattern(
    substring_pattern="import (sounddevice|pyaudio)",
    relative_path="python-stt",
    paths_include_glob="**/*.py"
)
```

**期待**: 検索結果が空であることを確認（ADR-001違反チェック）

---

### Phase 4: レビュー・リファクタリングフェーズ

#### パターン A: 依存関係の可視化

```python
# AudioServiceを参照している全ファイルを検索
mcp__serena__find_referencing_symbols(
    name_path="AudioService",
    relative_path="src-tauri/src/services/audio_service.rs"
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
    name_path="SpeechToTextEngine",
    relative_path="src-tauri/src/adapters/stt_adapter.rs",
    include_body=True
)
```

**確認ポイント**:
- traitで抽象化されているか
- 具体的な実装がAdapterに隔離されているか
- 設計原則5（ベンダーロックイン回避）に準拠しているか

#### パターン C: 要件トレーサビリティの確認

```bash
# 設計ドキュメントと実装の整合性確認
/kiro:validate-design meeting-minutes-core

# タスクの完了状態確認
/kiro:spec-status meeting-minutes-core
```

---

## ベストプラクティス

### DO: 推奨パターン

1. **概要から詳細へ**
   ```python
   # Good: まず概要を取得
   mcp__serena__get_symbols_overview("file.rs")
   # 次に必要な象徴だけ詳細取得
   mcp__serena__find_symbol("ClassName/method_name", include_body=True)
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
   ```python
   # Good: コメントに要件IDを含める
   body = """
   /// Related requirements: REQ-001.2, NFR-Perf-003
   pub fn process_audio(&self) -> Result<()> { ... }
   """
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
   # /kiro:validate-design <feature>
   # その後コミット
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
    relative_path="src-tauri/src/services"  # このディレクトリ内のみ
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

---

## チェックリスト

### 実装前
- [ ] `/kiro:spec-status`で現在のフェーズを確認
- [ ] 関連要件ID（REQ-###）を把握
- [ ] `mcp__serena__get_symbols_overview`で既存構造を理解

### 実装中
- [ ] `mcp__serena__find_symbol`で編集対象を特定
- [ ] 要件IDをコメントに含める
- [ ] `mcp__serena__find_referencing_symbols`で影響範囲確認

### 実装後
- [ ] `/kiro:validate-design`で仕様整合性確認
- [ ] Pre-commit hooksが成功
- [ ] コミットメッセージに要件IDを含める
- [ ] Requirement Traceability Matrix更新

---

## 参考リンク

- CLAUDE.md: 基本ワークフロー
- coding_standards.md: コーディング規約
- design_principles.md: 設計原則
- task_completion_workflow.md: タスク完了手順
