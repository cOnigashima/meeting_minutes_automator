# Phase 13: 検証負債解消

**目的**: MVP1 Core Implementationで延期した検証タスクを完了させ、本番リリース準備を整える

**前提条件**: Phase 1-12完了（MVP1 Core Implementation Milestone達成）

**完了日**: 未開始

---

## 概要

MVP1 Core Implementationでは、以下の検証タスクを「検証負債」として延期しました:
- Task 10.2-10.7: Rust E2Eテスト（`#[ignore]` + `unimplemented!()`）
- Task 11.3: 長時間稼働安定性テスト（2時間録音）
- SEC-001〜005: セキュリティ修正5件（Task 11.5で検出、修正保留）

Phase 13では、これらを完了させ、**meeting-minutes-sttを本番リリース可能な状態**にします。

---

## 13.1 Rust E2Eテスト実装（Task 10.2-10.7）

**目的**: Python単体テストで検証済みの機能を、Rust E2Eテストで統合動作確認

**現状**: `src-tauri/tests/stt_e2e_test.rs`に7テストが`#[ignore]` + `unimplemented!()`で存在

### 13.1.1 Task 10.2: オフラインモデルフォールバックE2E

**目的**: HuggingFace Hub接続失敗時のbundled baseモデルフォールバック動作確認

**実装内容**:
- [ ] ネットワーク切断シミュレーション実装（環境変数`HTTPS_PROXY=http://invalid-proxy:9999`）
- [ ] Python sidecar起動時のHuggingFace Hub接続失敗確認
- [ ] `model_change`イベント受信検証（`old_model`: "small", `new_model`: "base", `reason`: "offline_fallback"）
- [ ] bundled baseモデルでの文字起こし成功確認

**テストコード**:
```rust
#[tokio::test]
async fn test_offline_model_fallback() -> Result<()> {
    // 1. 環境変数でHuggingFace Hub接続失敗をシミュレート
    env::set_var("HTTPS_PROXY", "http://invalid-proxy:9999");

    // 2. Python sidecar起動（bundled baseモデルにフォールバック）
    let sidecar = PythonSidecarManager::start().await?;

    // 3. model_changeイベント受信検証
    let event = sidecar.wait_for_event("model_change", Duration::from_secs(10)).await?;
    assert_eq!(event["new_model"], "base");
    assert_eq!(event["reason"], "offline_fallback");

    // 4. 文字起こし実行確認
    let audio = load_test_audio("test_audio_short.wav")?;
    sidecar.send_audio_frames(&audio).await?;
    let transcript = sidecar.wait_for_transcription(Duration::from_secs(5)).await?;
    assert!(!transcript.is_empty());

    Ok(())
}
```

**要件**:
- STT-REQ-002.4: ネットワークエラー時にbundled baseモデルへフォールバック
- STT-REQ-002.5: バンドルモデル不在時はエラー
- ADR-016: Offline Model Fallback P0 Fix

**推定時間**: 4時間

---

### 13.1.2 Task 10.3: 動的モデルダウングレードE2E

**目的**: CPU/メモリ使用率に応じた自動モデル切替動作確認

**実装内容**:
- [ ] CPU使用率85%超過シミュレーション（60秒持続、Python側でCPU負荷注入）
- [ ] 自動ダウングレード検証（small → base、`model_change`イベント受信）
- [ ] メモリ使用率75%超過シミュレレーション（Python側でメモリ確保）
- [ ] 自動ダウングレード検証（medium → small、`model_change`イベント受信）
- [ ] アップグレード提案検証（リソース回復後5分待機、`upgrade_proposal`イベント受信）

**テストコード**:
```rust
#[tokio::test]
async fn test_dynamic_model_downgrade() -> Result<()> {
    // 1. Python sidecar起動（smallモデル）
    let sidecar = PythonSidecarManager::start_with_config(json!({
        "model_size": "small"
    })).await?;

    // 2. CPU負荷注入（85%超過、60秒）
    sidecar.inject_cpu_load(85, Duration::from_secs(60)).await?;

    // 3. ダウングレードイベント受信検証
    let event = sidecar.wait_for_event("model_change", Duration::from_secs(70)).await?;
    assert_eq!(event["old_model"], "small");
    assert_eq!(event["new_model"], "base");
    assert_eq!(event["reason"], "cpu_high");

    // 4. リソース回復シミュレーション
    sidecar.stop_cpu_load().await?;
    tokio::time::sleep(Duration::from_secs(300)).await; // 5分待機

    // 5. アップグレード提案イベント受信検証
    let event = sidecar.wait_for_event("upgrade_proposal", Duration::from_secs(10)).await?;
    assert_eq!(event["proposed_model"], "small");
    assert_eq!(event["reason"], "resources_recovered");

    Ok(())
}
```

**要件**:
- STT-REQ-006.6: 30秒間隔でCPU/メモリ監視
- STT-REQ-006.7: CPU 85%超過60秒 or メモリ75%超過で自動ダウングレード
- STT-REQ-006.8: ダウングレード時にUI通知
- ADR-017: Latency Requirements Adjustment

**推定時間**: 6時間

---

### 13.1.3 Task 10.4: デバイス切断/再接続E2E

**目的**: 音声デバイス切断時の自動再接続動作確認

**実装内容**:
- [ ] 音声デバイス切断シミュレーション（OS APIでデバイス無効化、またはMockデバイスでエラー注入）
- [ ] `device_disconnected`イベント受信検証
- [ ] 3秒間隔・最大3回の再接続試行確認（ログ出力検証）
- [ ] `device_reconnected`イベント受信検証（再接続成功時）
- [ ] 録音再開確認（音声フレーム送信再開）

**テストコード**:
```rust
#[tokio::test]
async fn test_device_disconnection_reconnection() -> Result<()> {
    // 1. MockAudioDevice起動
    let device = MockAudioDevice::new("test-device");
    let sidecar = PythonSidecarManager::start().await?;

    // 2. 録音開始
    device.start_recording().await?;

    // 3. デバイス切断シミュレート
    device.inject_error(AudioDeviceError::Disconnected).await?;

    // 4. device_disconnectedイベント受信検証
    let event = sidecar.wait_for_event("device_disconnected", Duration::from_secs(5)).await?;
    assert_eq!(event["device_id"], "test-device");

    // 5. 再接続試行確認（3秒間隔×3回）
    tokio::time::sleep(Duration::from_secs(10)).await;

    // 6. デバイス復旧シミュレート
    device.recover().await?;

    // 7. device_reconnectedイベント受信検証
    let event = sidecar.wait_for_event("device_reconnected", Duration::from_secs(5)).await?;
    assert_eq!(event["device_id"], "test-device");

    // 8. 録音再開確認
    let audio = device.capture_audio(Duration::from_secs(1)).await?;
    assert!(!audio.is_empty());

    Ok(())
}
```

**要件**:
- STT-REQ-004.9: デバイスエラー検出時に`StreamError`イベント配信
- STT-REQ-004.10: 3秒間隔で最大3回再接続試行
- STT-REQ-004.11: 再接続成功時に`Reconnected`イベント配信

**推定時間**: 5時間

---

### 13.1.4 Task 10.5: クロスプラットフォーム互換性E2E

**目的**: Windows/Linux環境での動作確認（現在macOSのみ検証済み）

**実装内容**:
- [ ] **Windows**: WASAPI loopback audio capture動作確認
- [ ] **Windows**: Python `py.exe` launcher detection動作確認（`src-tauri/src/python_sidecar.rs`）
- [ ] **Windows**: Cross-platform path handling動作確認（`std::path::Path` API）
- [ ] **Linux**: ALSA/PulseAudio audio capture動作確認
- [ ] **Linux**: Audio group permissions確認（`/dev/snd/*`）
- [ ] **Linux**: GTK dependencies確認（Tauri 2.0要件）
- [ ] プラットフォーム検証結果を`docs/platform-verification.md`に追記

**テストコード**:
```rust
#[tokio::test]
#[cfg(target_os = "windows")]
async fn test_windows_wasapi_loopback() -> Result<()> {
    // 1. WASAPI loopbackデバイス列挙
    let devices = list_audio_devices()?;
    let loopback = devices.iter().find(|d| d.name.contains("Loopback"));
    assert!(loopback.is_some(), "WASAPI loopback device not found");

    // 2. 録音開始
    start_recording(loopback.unwrap().id.clone()).await?;

    // 3. 音声フレーム受信確認
    let frames = capture_audio_frames(Duration::from_secs(1)).await?;
    assert!(!frames.is_empty());

    Ok(())
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_linux_pulseaudio_monitor() -> Result<()> {
    // 1. PulseAudio monitorデバイス列挙
    let devices = list_audio_devices()?;
    let monitor = devices.iter().find(|d| d.name.contains("Monitor"));
    assert!(monitor.is_some(), "PulseAudio monitor device not found");

    // 2. 録音開始
    start_recording(monitor.unwrap().id.clone()).await?;

    // 3. 音声フレーム受信確認
    let frames = capture_audio_frames(Duration::from_secs(1)).await?;
    assert!(!frames.is_empty());

    Ok(())
}
```

**要件**:
- STT-NFR-003: macOS 12+, Windows 10 22H2+, Ubuntu 22.04+対応

**推定時間**: 6時間（Windows/Linux実機検証含む）

---

### 13.1.5 Task 10.6: 非機能要件E2E

**目的**: レイテンシ・パフォーマンス要件の実測確認

**実装内容**:
- [ ] 部分テキスト応答時間 <0.5s 検証（音声フレーム送信 → `transcription`イベント受信）
- [ ] 確定テキスト応答時間 <2s 検証（VAD speech_end → 確定テキスト受信）
- [ ] IPC latency <5ms 検証（stdin書き込み → stdoutイベント受信）
- [ ] Audio callback latency <10μs 検証（ring buffer push操作時間）
- [ ] E2E latency <100ms 検証（音声フレーム → WebSocket配信）

**テストコード**:
```rust
#[tokio::test]
async fn test_partial_text_latency() -> Result<()> {
    let sidecar = PythonSidecarManager::start().await?;
    let audio = load_test_audio("test_audio_short.wav")?;

    // 1. 音声フレーム送信開始
    let start = Instant::now();
    sidecar.send_audio_frames(&audio).await?;

    // 2. 部分テキスト受信
    let event = sidecar.wait_for_event_with_filter(
        "transcription",
        |e| e["isPartial"] == true,
        Duration::from_secs(1)
    ).await?;

    let latency = start.elapsed();
    assert!(latency < Duration::from_millis(500), "Partial text latency: {:?}", latency);

    Ok(())
}

#[tokio::test]
async fn test_audio_callback_latency() -> Result<()> {
    let ring_buffer = RingBuffer::new(8000); // 5-second buffer
    let audio_frame = vec![0i16; 320]; // 20ms @ 16kHz

    // 1. ring buffer push時間測定（1000回平均）
    let mut latencies = Vec::new();
    for _ in 0..1000 {
        let start = Instant::now();
        ring_buffer.push(&audio_frame)?;
        latencies.push(start.elapsed());
    }

    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    assert!(avg_latency < Duration::from_micros(10), "Audio callback latency: {:?}", avg_latency);

    Ok(())
}
```

**要件**:
- STT-NFR-001.1: 部分テキスト応答時間 <0.5s
- STT-NFR-001.2: 確定テキスト応答時間 <2s
- STT-NFR-002.1: IPC latency <5ms
- ADR-013: Audio callback latency <10μs（lock-free ring buffer）

**推定時間**: 3時間

---

### 13.1.6 Task 10.7: IPC/WebSocket後方互換性E2E

**目的**: プロトコルバージョン不一致時の挙動確認

**実装内容**:
- [ ] IPC protocol major version不一致検証（`1.0.0` vs `2.0.0` → エラー）
- [ ] IPC protocol minor version不一致検証（`1.0.0` vs `1.1.0` → 警告）
- [ ] IPC protocol patch version不一致検証（`1.0.0` vs `1.0.1` → 互換）
- [ ] WebSocket protocol version検証（古いクライアント接続時の警告ログ）

**テストコード**:
```rust
#[tokio::test]
async fn test_ipc_version_mismatch_major() -> Result<()> {
    // 1. Python sidecar起動（version 1.0.0）
    let sidecar = PythonSidecarManager::start().await?;

    // 2. Rust側でversion 2.0.0を送信
    sidecar.send_message(json!({
        "version": "2.0.0",
        "type": "ping"
    })).await?;

    // 3. エラーイベント受信検証
    let event = sidecar.wait_for_event("error", Duration::from_secs(5)).await?;
    assert_eq!(event["error_type"], "version_incompatible");
    assert!(event["message"].as_str().unwrap().contains("major"));

    Ok(())
}

#[tokio::test]
async fn test_ipc_version_mismatch_minor() -> Result<()> {
    // 1. Python sidecar起動（version 1.0.0）
    let sidecar = PythonSidecarManager::start().await?;

    // 2. Rust側でversion 1.1.0を送信
    sidecar.send_message(json!({
        "version": "1.1.0",
        "type": "ping"
    })).await?;

    // 3. 警告ログ確認（エラーにはならない）
    let logs = sidecar.capture_logs(Duration::from_secs(1)).await?;
    assert!(logs.contains("version mismatch: 1.1.0 vs 1.0.0 (backward compatible)"));

    // 4. 通常処理継続確認
    let pong = sidecar.wait_for_event("pong", Duration::from_secs(1)).await?;
    assert!(pong.is_ok());

    Ok(())
}
```

**要件**:
- STT-REQ-007.1: Major version不一致 → エラー
- STT-REQ-007.2: Minor version不一致 → 警告、後方互換性維持
- STT-REQ-007.3: Patch version不一致 → 完全互換
- ADR-003: IPC Versioning

**推定時間**: 3時間

---

## 13.2 長時間稼働テスト（Task 11.3）

**目的**: 2時間連続録音でのメモリリーク・クラッシュ検証

### 13.2.1 2時間連続録音テスト

**実装内容**:
- [ ] 2時間連続録音実行（7200秒、実音声またはtest fixtureループ再生）
- [ ] メモリ使用量監視（30分ごとにスナップショット、`ps`コマンド）
- [ ] CPU使用率監視（平均・最大値記録、`top`コマンド）
- [ ] フレームドロップ率測定（0%目標、ring buffer overflow検出）
- [ ] ログ記録（`logs/platform/stability-<timestamp>/`に保存）

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

**成功基準**:
- [ ] 2時間完走（クラッシュなし）
- [ ] メモリ使用量 <2GB、±10%以内で安定
- [ ] CPU使用率平均 <50%
- [ ] フレームドロップ率 0%

**要件**:
- STT-NFR-004.1: メモリリークなし（2時間連続録音）

**推定時間**: 1日（実行時間2時間 + 準備・分析時間）

---

### 13.2.2 メモリリーク検証

**実装内容**:
- [ ] 開始時メモリ使用量記録（baseline）
- [ ] 2時間後メモリ使用量確認（baseline±10%以内）
- [ ] Valgrind/LeakSanitizer実行（オプション、Linux環境）
- [ ] メモリプロファイリング結果分析

**Valgrind実行例**（Linux）:
```bash
# Rustバイナリでメモリリーク検出
valgrind --leak-check=full --show-leak-kinds=all \
  ./target/debug/meeting-minutes-automator 2>&1 | tee valgrind.log

# 結果確認
grep "definitely lost" valgrind.log
```

**成功基準**:
- [ ] "definitely lost: 0 bytes in 0 blocks"
- [ ] 2時間後のメモリ使用量が開始時の±10%以内

**推定時間**: 3時間

---

### 13.2.3 長時間稼働ログ分析

**実装内容**:
- [ ] `python3 scripts/performance_report.py <burnin.log>` 実行
- [ ] メトリクス平均・P50・P95・P99算出
- [ ] 結果を`target/performance_reports/`に保存
- [ ] `docs/platform-verification.md`に結果追記

**分析項目**:
- 部分テキスト応答時間（平均・P95）
- 確定テキスト応答時間（平均・P95）
- IPC latency（平均・P95）
- メモリ使用量推移
- CPU使用率推移

**推定時間**: 2時間

---

## 13.3 セキュリティ修正（SEC-001〜005）

**目的**: Task 11.5で検出した5件の脆弱性を修正

**詳細**: `.kiro/specs/meeting-minutes-stt/security-test-report.md`参照

### 13.3.1 SEC-001: pip 25.0脆弱性修正

**問題**: pip 25.0にMedium脆弱性（GHSA-4xh5-x5gv-qwph）

**修正内容**:
- [ ] `python-stt/requirements.txt`で`pip>=25.2`に更新
- [ ] `.venv`再構築テスト（`rm -rf .venv && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt`）
- [ ] `pip-audit`再実行で脆弱性0件確認

**修正コード**:
```diff
# python-stt/requirements.txt
- pip==25.0
+ pip>=25.2
```

**検証**:
```bash
cd python-stt
.venv/bin/pip-audit
# Expected: No known vulnerabilities found
```

**要件**: SEC-001
**深刻度**: 🟡 Medium
**優先度**: P0

**推定時間**: 30分

---

### 13.3.2 SEC-002: CSP設定

**問題**: `tauri.conf.json`で`csp: null`（開発環境用、本番では危険）

**修正内容**:
- [ ] `src-tauri/tauri.conf.json`で本番CSPポリシー設定
- [ ] `script-src 'self'`, `connect-src 'self' ws://localhost:9001-9100`設定
- [ ] 開発環境（`npm run tauri dev`）でCSP無効化維持（条件分岐）
- [ ] 本番ビルド（`npm run tauri build`）でCSP有効化確認

**修正コード**:
```json
// src-tauri/tauri.conf.json
{
  "tauri": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; connect-src 'self' ws://localhost:9001-9100; img-src 'self' data:; style-src 'self' 'unsafe-inline'"
    }
  }
}
```

**環境別CSP設定**:
```rust
// src-tauri/src/main.rs
#[cfg(debug_assertions)]
const CSP: Option<&str> = None; // 開発環境: CSP無効

#[cfg(not(debug_assertions))]
const CSP: Option<&str> = Some("default-src 'self'; ..."); // 本番: CSP有効
```

**検証**:
```bash
# 開発環境: CSP無効確認
npm run tauri dev
# ブラウザDevToolsでCSPエラーがないことを確認

# 本番ビルド: CSP有効確認
npm run tauri build
# ブラウザDevToolsでCSPが適用されていることを確認
```

**要件**: SEC-002
**深刻度**: 🟡 Medium
**優先度**: P0

**推定時間**: 1時間

---

### 13.3.3 SEC-003: ファイル権限強制

**問題**: 音声ファイル（audio.wav）のパーミッションがumask依存（644、誰でも読める）

**修正内容**:
- [ ] `src-tauri/src/storage.rs`で`OpenOptions::mode(0o600)`追加
- [ ] 音声ファイル（audio.wav）のパーミッション600検証
- [ ] 文字起こしファイル（transcription.jsonl）のパーミッション600検証
- [ ] セッションファイル（session.json）のパーミッション600検証

**修正コード**:
```rust
// src-tauri/src/storage.rs
use std::os::unix::fs::OpenOptionsExt; // Unix系のみ

pub async fn create_audio_file(session_id: &str) -> Result<File> {
    let path = format!("sessions/{}/audio.wav", session_id);

    #[cfg(unix)]
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o600) // rw------- (owner only)
        .open(&path)?;

    #[cfg(not(unix))]
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)?;

    Ok(file)
}
```

**検証**:
```bash
# ファイル作成後、パーミッション確認
ls -la sessions/test-session/audio.wav
# Expected: -rw------- (600)

# 他ユーザーからの読み取り試行（失敗を期待）
sudo -u nobody cat sessions/test-session/audio.wav
# Expected: Permission denied
```

**要件**: SEC-003
**深刻度**: 🟡 Medium
**優先度**: P0

**推定時間**: 1時間

---

### 13.3.4 SEC-005: TLS 1.0/1.1接続失敗テスト

**問題**: TLS 1.2+強制は主張のみ、実証テストなし

**修正内容**:
- [ ] TLS 1.0/1.1エンドポイントへの接続失敗テスト実装
- [ ] Python `ssl.create_default_context()`のminimum_version検証
- [ ] HuggingFace Hub接続時のTLS version確認

**テストコード**:
```python
# python-stt/tests/test_tls_version.py
import ssl
import pytest
from urllib.request import urlopen, Request

def test_tls_1_0_rejected():
    """TLS 1.0エンドポイントへの接続が失敗することを確認"""
    context = ssl.create_default_context()

    # TLS 1.0エンドポイント（テスト用）
    # 注: 実際のエンドポイントはCI環境で用意する必要がある
    with pytest.raises(ssl.SSLError) as exc_info:
        req = Request("https://tls-v1-0.badssl.com:1010/")
        urlopen(req, context=context, timeout=5)

    assert "UNSUPPORTED_PROTOCOL" in str(exc_info.value) or \
           "TLSV1_ALERT" in str(exc_info.value)

def test_tls_1_2_minimum_version():
    """ssl.create_default_context()がTLS 1.2を最小バージョンとすることを確認"""
    context = ssl.create_default_context()
    assert context.minimum_version == ssl.TLSVersion.TLSv1_2
```

**検証**:
```bash
cd python-stt
.venv/bin/python -m pytest tests/test_tls_version.py -v
```

**要件**: SEC-005
**深刻度**: 🟡 Medium
**優先度**: P0

**推定時間**: 2時間

---

### 13.3.5 SEC-004: cargo-audit実施（Blocked）

**問題**: `cargo audit`がRust 1.85未対応（edition2024機能使用）

**修正内容**:
- [ ] Rust 1.85リリース待機（2025年2月予定）
- [ ] リリース後即座に`cargo audit`実行
- [ ] 脆弱性検出時は即座に修正
- [ ] GitHub Actions CIに`cargo audit`ステップ追加

**暫定対応**:
```bash
# 代替手段: cargo tree -dで重複依存確認
cargo tree -d
# Known vulnerabilities: None detected
```

**Rust 1.85リリース後の対応**:
```bash
# 1. Rust 1.85へアップデート
rustup update

# 2. cargo-auditインストール
cargo install cargo-audit

# 3. 脆弱性スキャン実行
cargo audit
# Expected: No vulnerabilities found

# 4. CI/CDに追加
# .github/workflows/rust-tests.yml
- name: Security audit
  run: cargo audit
```

**要件**: SEC-004
**深刻度**: 🔴 Blocked → 🔴 High（Rust 1.85リリース後）
**優先度**: P0（リリース後即実施）

**推定時間**: 30分（Rust 1.85リリース後）

---

## 完了基準

### Phase 13全体
- [ ] 13.1: Task 10.2-10.7のRust E2Eテスト全合格（7テスト）
- [ ] 13.2: 2時間連続録音成功、メモリリークなし
- [ ] 13.3: SEC-001/002/003/005修正完了、SEC-004待機中
- [ ] Windows/Linux実機検証完了（`platform-verification.md`更新）
- [ ] 全テスト合格（Rust 78テスト, Python 143テスト = 221テスト）

### リリース判定基準
- [ ] Phase 13完了
- [ ] セキュリティ脆弱性0件（SEC-004除く、Rust 1.85待ち）
- [ ] クロスプラットフォーム動作確認（macOS/Windows/Linux）
- [ ] 2時間以上の連続録音成功
- [ ] `docs/platform-verification.md`全プラットフォーム更新完了

---

## 推定作業量

| サブタスク | 推定時間 |
|-----------|---------|
| 13.1.1 (オフラインフォールバック) | 4時間 |
| 13.1.2 (動的ダウングレード) | 6時間 |
| 13.1.3 (デバイス切断/再接続) | 5時間 |
| 13.1.4 (クロスプラットフォーム) | 6時間 |
| 13.1.5 (非機能要件) | 3時間 |
| 13.1.6 (後方互換性) | 3時間 |
| 13.2.1-13.2.3 (長時間稼働) | 1日 |
| 13.3.1-13.3.5 (セキュリティ) | 5時間 |
| **合計** | **5-7日** |

---

## 実装順序

**優先度順**:
1. **13.3 セキュリティ修正**（最優先、本番リリース前必須）
   - 13.3.1 (SEC-001): 30分
   - 13.3.2 (SEC-002): 1時間
   - 13.3.3 (SEC-003): 1時間
   - 13.3.4 (SEC-005): 2時間
   - 13.3.5 (SEC-004): Rust 1.85待ち

2. **13.2 長時間稼働テスト**（リリース前必須）
   - 13.2.1: 2時間実行
   - 13.2.2-13.2.3: 分析

3. **13.1 Rust E2Eテスト**（品質保証、並行作業可能）
   - 13.1.1 → 13.1.2 → 13.1.3 → 13.1.5 → 13.1.6
   - 13.1.4（クロスプラットフォーム）は最後（実機環境必要）

---


## Post-MVP1 Cleanup Tasks

MVP1実装完了後の技術的負債とコードクリーンアップタスク。これらは機能動作には影響しないが、コード品質とメンテナンス性を向上させます。

- [ ] 14. レガシーIPCプロトコルの削除
- [ ] 14.1 LegacyIpcMessage完全削除の検討
  - **状況**: `python_sidecar.rs` の `LegacyIpcMessage` enum が deprecated 警告を大量出力（9件）
  - **現状**: MVP0互換レイヤとして保持中。新プロトコル（`ipc_protocol::IpcMessage`）への完全移行済み
  - **選択肢**:
    1. **完全削除（推奨）**: MVP0互換性が不要なら、`LegacyIpcMessage` 定義と変換ロジックを削除
       - `src/python_sidecar.rs` L76-138: `impl LegacyIpcMessage` ブロック全削除
       - `ProtocolMessage::from_legacy()` ヘルパー削除
       - すべて `ipc_protocol::IpcMessage` に統一
    2. **局所抑制**: 互換性維持が必要なら `#[allow(deprecated)]` を付けて警告抑制
       ```rust
       #[allow(deprecated)]
       impl LegacyIpcMessage {
           pub fn to_protocol_message(self) -> ProtocolMessage { ... }
       }
       ```
  - **判断基準**: Python側（`python-stt/main.py`）とテストがすべて新プロトコル使用済み → 完全削除可能
  - **作業ステップ**（完全削除の場合）:
    1. `grep -r "LegacyIpcMessage" src/` で全参照箇所を確認
    2. `src/python_sidecar.rs` から `LegacyIpcMessage` enum定義を削除
    3. 変換ロジック（`to_protocol_message()`, `from_legacy()`）を削除
    4. `cargo check` でコンパイルエラーがないことを確認
    5. `cargo test --all` で全テスト通過確認（MVP0互換テストが失敗する場合は削除）
  - _Requirements: STT-REQ-007 (IPCバージョニング), コード品質向上_
  - _Priority: P2（機能影響なし、警告ノイズ削減）_

- [ ] 14.2 未使用コード削除
  - **src/commands.rs** の dead code 警告（2件）対応:
    1. `use crate::audio_device_adapter::AudioDeviceAdapter;` - 未使用import削除
       - 静的列挙実装（Task 2.2）でtrait使用を廃止したため
       - `AudioDeviceEvent` は使用中のため残す
    2. `async fn start_ipc_reader_task(...)` - 未使用関数の削除または保留判断
       - フェーズ10で使用予定なら `#[allow(dead_code)]` を付ける
       - 使用予定がないなら削除
  - **作業ステップ**:
    1. `src/commands.rs:7` の import を修正:
       ```rust
       // Before
       use crate::audio_device_adapter::{AudioDeviceAdapter, AudioDeviceEvent};
       // After
       use crate::audio_device_adapter::AudioDeviceEvent;
       ```
    2. `start_ipc_reader_task()` の扱いを判断:
       - 削除: フェーズ10で不要と確定した場合
       - 保留: `#[allow(dead_code)] async fn start_ipc_reader_task(...) { ... }`
    3. `cargo check` で警告が消えたことを確認
  - _Requirements: コード品質向上_
  - _Priority: P2（機能影響なし、警告ノイズ削減）_

- [ ] 14.3 クリーンビルド検証
  - **目的**: 上記2タスク完了後、警告ゼロでビルド通過することを確認
  - **作業ステップ**:
    1. `cargo clean` でクリーンビルド
    2. `cargo check --all-targets` で警告が11件 → 0件に減少することを確認
    3. `cargo test --all` で全テスト通過確認（44テスト以上）
    4. `cargo clippy -- -D warnings` で Clippy警告もゼロに
  - _Requirements: コード品質向上_
  - _Priority: P2（MVP1機能完成後の品質改善）_

**Note**: これらのタスクはMVP1機能に影響を与えません。優先度P2として、MVP1完了後またはリファクタリングフェーズで実施することを推奨します。


## 次のステップ

Phase 13完了後:
1. `spec.json`の`phase`を`verification` → `completed`に更新
2. `meeting-minutes-docs-sync`（MVP2本体）spec初期化
3. Google Docs同期機能実装開始
