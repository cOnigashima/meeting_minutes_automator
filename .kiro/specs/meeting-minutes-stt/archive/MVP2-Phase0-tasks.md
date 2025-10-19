# MVP2 Phase 0: 検証負債解消タスクリスト

**目的**: Phase 13延期タスクを完了し、MVP2本体（Google Docs連携）開始準備を整える

**完了基準**: セキュリティ修正5件完了 + 長時間稼働テスト合格

**推定作業量**: 1.5日（セキュリティ5h + 長時間テスト1日）

---

## Week 1: セキュリティ修正 + 長時間テスト（1.5日）

### 最優先タスク

#### SEC-001: pip-audit導入（1時間）

**要件**: SEC-001（security-test-report.md）

**実装内容**:
- [ ] `python-stt/requirements-dev.txt`にpip-audit追加
- [ ] `.pre-commit-config.yaml`にpip-auditフック追加
- [ ] GitHub Actions workflow追加（security-scan.yml）
- [ ] 初回実行・脆弱性レポート確認

**受け入れ基準**:
- `pip-audit`実行成功
- 脆弱性0件または既知の除外設定済み

---

#### SEC-002: CSP設定強化（1時間）

**要件**: SEC-002（security-test-report.md）

**実装内容**:
- [ ] Chrome拡張`manifest.json`更新
- [ ] CSP: `default-src 'self'; connect-src ws://localhost:*`
- [ ] Chrome拡張ローディングテスト
- [ ] WebSocket接続動作確認

**受け入れ基準**:
- Chrome拡張ロード成功
- WebSocket接続正常動作
- CSP violation 0件（Chrome DevTools Console確認）

**実装ファイル**:
- `chrome-extension/manifest.json`

---

#### SEC-003: Windows ACL設定（1時間）

**要件**: SEC-003（security-test-report.md）

**実装内容**:
- [ ] Windows ACL API調査（`windows-rs` crate）
- [ ] `storage.rs`のTODO実装（L27）
- [ ] Windows実機検証（または CI Windows runner）

**受け入れ基準**:
- Windows環境でファイル権限設定成功
- Owner以外のアクセス拒否確認

**実装ファイル**:
- `src-tauri/src/storage.rs` L27-32

**現状**:
```rust
// TODO(SEC-003): Windows file permissions
// Use windows-rs to set ACLs (owner only)
Ok(std::fs::File::create(path)?)
```

**実装方針**:
```rust
#[cfg(windows)]
fn create_file_owner_only(path: &std::path::Path) -> Result<std::fs::File> {
    use windows::Win32::Security::*;
    use windows::Win32::Storage::FileSystem::*;

    // 1. Create file
    let file = std::fs::File::create(path)?;

    // 2. Get owner SID
    let owner_sid = get_current_user_sid()?;

    // 3. Create DACL (owner: FULL_CONTROL, others: DENY)
    let dacl = create_owner_only_dacl(owner_sid)?;

    // 4. Set DACL to file
    set_file_dacl(path, dacl)?;

    Ok(file)
}
```

---

#### SEC-004: cargo-audit導入（1時間）

**要件**: SEC-004（security-test-report.md）

**実装内容**:
- [ ] Rust 1.85以降確認（`rustc --version`）
- [ ] cargo-audit統合（GitHub Actions）
- [ ] 初回実行・脆弱性レポート確認

**受け入れ基準**:
- `cargo audit`実行成功
- 脆弱性0件または既知の除外設定済み

**Note**: Rust 1.85リリース前（2025-02予定）は手動実行のみ

---

#### SEC-005: TLS証明書検証（1時間）

**要件**: SEC-005（security-test-report.md）

**実装内容**:
- [ ] HuggingFace Hub接続時のTLS証明書検証追加
- [ ] `reqwest` crate設定またはカスタムCA bundle
- [ ] オフラインフォールバックテスト（証明書エラー時）

**受け入れ基準**:
- 有効な証明書: 接続成功
- 無効な証明書: 接続失敗 + バンドルモデルフォールバック

**実装ファイル**:
- `python-stt/stt_engine/whisper_model_manager.py`

**実装方針**:
```python
import requests
from requests.adapters import HTTPAdapter
from urllib3.poolmanager import PoolManager
import ssl

class TLSAdapter(HTTPAdapter):
    def init_poolmanager(self, *args, **kwargs):
        context = ssl.create_default_context()
        context.check_hostname = True
        context.verify_mode = ssl.CERT_REQUIRED
        kwargs['ssl_context'] = context
        return super().init_poolmanager(*args, **kwargs)

# HuggingFace Hub接続時
session = requests.Session()
session.mount('https://', TLSAdapter())
```

---

#### Task 11.3: 2時間連続録音安定性テスト（1日）

**要件**: Task 11.3（tasks/phase-13-verification.md）

**実装内容**:
- [ ] 安定性テストスクリプト作成（`scripts/stability_burn_in.sh`）
- [ ] メモリ監視スクリプト作成（30分ごとスナップショット）
- [ ] 2時間連続録音実行（実音声またはtest fixtureループ再生）
- [ ] 結果分析レポート作成（`logs/platform/stability-<timestamp>/`）

**受け入れ基準**:
- 2時間録音完了（クラッシュ0件）
- メモリ使用量<500MB（開始時比+10%以内）
- CPU使用率平均<30%
- フレームドロップ率<1%

**テスト手順**:
```bash
# 1. 安定性テスト実行
./scripts/stability_burn_in.sh --duration 7200 --session-label macos

# 2. 別ターミナルでリソース監視
while true; do
    ps -o pid,%cpu,%mem,etime -p $(pgrep -f tauri) | tee -a logs/platform/stability-<timestamp>/resource-snapshots.txt
    sleep 1800  # 30分ごと
done

# 3. 完了後、ログ分析
python3 scripts/performance_report.py logs/platform/stability-<timestamp>/burnin.log
```

**実装ファイル**:
- `scripts/stability_burn_in.sh`（新規作成）
- `scripts/performance_report.py`（新規作成）

---

## Week 2-3: CI/CD整備 + Phase 13.1再開準備（3-4日）

### 並行作業タスク

#### CI-001: GitHub Actions CI/CD整備（2-3日）

**Spec**: `meeting-minutes-ci`

**実装内容**:
- [ ] Matrix strategy設定（macOS/Windows/Linux）
- [ ] Nightly CI設定（長時間テスト自動化）
- [ ] Artifact保存（テスト結果・ログ）
- [ ] Slack通知統合

**受け入れ基準**:
- 全プラットフォームでCI緑化
- Nightly CIで長時間テスト自動実行
- 失敗時にSlack通知

**実装ファイル**:
- `.github/workflows/ci.yml`
- `.github/workflows/nightly.yml`

---

#### P13-PREP-001: Python API追加（Task 10.3準備、2-3時間）

**要件**: Task 10.3ブロッカー解消

**実装内容**:
- [ ] `PythonSidecarManager::inject_cpu_load()`メソッド追加
- [ ] Python側IPC handler実装（`cpu_load_injection`メッセージ）
- [ ] 単体テスト追加（`tests/test_python_sidecar.rs`）

**受け入れ基準**:
- `inject_cpu_load(85, Duration::from_secs(60))`実行成功
- CPU使用率85%達成（`psutil`確認）

**実装ファイル**:
- `src-tauri/src/python_sidecar.rs`
- `python-stt/stt_engine/ipc_handler.py`

---

#### P13-PREP-002: STT-REQ-004.11仕様確定（Task 10.4準備、1時間）

**要件**: Task 10.4ブロッカー解消

**実装内容**:
- [ ] 自動再接続戦略確定（再試行回数・間隔・タイムアウト）
- [ ] requirements.md更新（STT-REQ-004.11を「⏳ 計画中」→「✅ 完了」）
- [ ] ADR作成（ADR-018: Audio Device Auto-Reconnect）

**受け入れ基準**:
- STT-REQ-004.11に再試行戦略記載
- ADR-018承認

**実装ファイル**:
- `.kiro/specs/meeting-minutes-stt/requirements.md` L690
- `.kiro/specs/meeting-minutes-stt/adrs/ADR-018-audio-device-auto-reconnect.md`（新規）

---

## Phase 13.1再開タスク（CI整備完了後）

### Task 10.3: 動的モデルダウングレードE2E（3時間）

**前提条件**: P13-PREP-001完了

**実装内容**:
- [ ] E2Eテスト実装（`tests/stt_e2e_test.rs`）
- [ ] CPU負荷注入シナリオ実装
- [ ] メモリ負荷注入シナリオ実装
- [ ] アップグレード提案検証

**受け入れ基準**:
- `test_dynamic_model_downgrade`合格
- `test_dynamic_model_upgrade_proposal`合格

---

### Task 10.4: デバイス切断/再接続E2E（4時間）

**前提条件**: P13-PREP-002完了

**実装内容**:
- [ ] E2Eテスト実装（`tests/stt_e2e_test.rs`）
- [ ] デバイス切断シミュレーション
- [ ] 自動再接続検証
- [ ] 録音再開確認

**受け入れ基準**:
- `test_device_disconnection_reconnection`合格

---

### Task 10.5: クロスプラットフォームE2E（6時間）

**前提条件**: CI-001完了

**実装内容**:
- [ ] Windows/Linux実機またはCI runner実行
- [ ] プラットフォーム固有バグ修正
- [ ] `platform-verification.md`更新

**受け入れ基準**:
- macOS/Windows/Linux全環境でテスト合格
- `platform-verification.md`に動作確認記録

---

## 完了判定基準（Phase 0 DoD）

### 必須条件

- [x] SEC-001: pip-audit導入完了
- [x] SEC-002: CSP設定強化完了
- [x] SEC-003: Windows ACL設定完了
- [x] SEC-004: cargo-audit導入完了
- [x] SEC-005: TLS証明書検証完了
- [x] Task 11.3: 2時間連続録音成功
- [x] セキュリティ脆弱性0件

### 推奨条件

- [ ] CI-001: GitHub Actions CI/CD整備完了
- [ ] P13-PREP-001/002: Phase 13.1再開準備完了
- [ ] Task 10.3/10.4/10.5: 再開タスク完了（CI整備後）

---

## タイムライン

| Week | タスク | 推定工数 |
|------|--------|----------|
| Week 1 | SEC-001〜005 + Task 11.3 | 1.5日 |
| Week 2-3 | CI-001 + P13-PREP-001/002 | 3-4日 |
| Week 4+ | Task 10.3/10.4/10.5 | 13h（1.5-2日） |

**合計**: 6-7.5日

---

## 次ステップ

1. **即座実行**: Week 1タスク開始（SEC-001〜005 + Task 11.3）
2. **Week 2**: CI整備開始（meeting-minutes-ci spec実装）
3. **Week 4**: Phase 13.1再開（Task 10.3/10.4/10.5）
4. **Phase 1**: MVP2本体開始（Google Docs連携）
