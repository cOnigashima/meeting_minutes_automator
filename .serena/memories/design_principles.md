# Design Principles

## Core Design Principles

### 1. プロセス境界の明確化原則
- 各プロセス（Rust/Python/Chrome拡張）は独立した責務を持つ
- **重要**: 音声録音はRust側`AudioDeviceAdapter`のみが担当（ADR-001）
- Pythonサイドカーは録音を行わない（レース条件防止）
- 静的解析で強制: `scripts/check_forbidden_imports.py`

### 2. オフラインファースト原則
- コア機能（録音、VAD、STT）はインターネット接続不要で完全動作
- ネットワーク依存機能は段階的縮退（graceful degradation）を実装
- Tier 1（オフライン必須）、Tier 2（オンライン推奨）、Tier 3（オンライン必須）の3段階

### 3. セキュリティ責任境界の原則
- OAuth token保管: Tauri App（OS Keychain）
- Chrome拡張: 表示専用UI、機密情報禁止
- 機密情報はWebSocket経由でも平文送信禁止

### 4. 段階的リソース管理原則
3段階閾値の定義:
- 警告（黄）: UI通知バナー、古いセッション削除提案
- 制限（赤）: 録音品質低下、要約生成停止
- 強制停止: 録音停止、データ保存、エラーログ記録

### 5. ベンダーロックイン回避原則
- STT Engine、LLM API、Document APIは全てAdapterパターンで抽象化
- 設定ファイルでの実装切り替えをサポート
- ADR-002: ハイブリッド配布戦略（モデル交換可能性確保）

### 6. TDD原則（Skeleton-First with TDD）
- 新規機能はスケルトン → ユニットテスト → 統合テスト → E2Eの順で実装
- 失敗するテストを先に用意する
- カバレッジ閾値: ユニット80% / 統合主要シナリオ100%

### 7. 非機能ベースライン原則
- ログ方針: INFO/DEBUG/ERROR、構造化JSON、PIIマスク
- エラー分類: Recoverable/Non-Recoverable
- セキュリティ: 入出力バリデーション、トークン保護

### 8. 図版管理原則
- PlantUMLソースを`docs/uml/<spec-slug>/<カテゴリ>/ID_*.puml`に配置
- 図種別の標準化: UC/CMP/SEQ/CLS/STM/ACT/DEP
- PRで図版更新理由を明記

## ADRs (Architecture Decision Records)
- **ADR-001**: Recording Responsibility（録音責務の一元化）
- **ADR-002**: Model Distribution Strategy（ハイブリッド配布戦略）
- **ADR-003**: IPC Versioning（セマンティックバージョニング）
