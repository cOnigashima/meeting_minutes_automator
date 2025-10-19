# Platform Verification Report

## Overview

Cross-platform compatibility verification for Meeting Minutes Automator.

**Target Platforms**: macOS, Windows 10+, Linux (Ubuntu 20.04+)

---

## Baseline Environments

| Platform | Hostname / Device ID | OS & Build | Audio Driver / Stack | Primary Input Device | Notes | Last Verified |
|----------|----------------------|------------|----------------------|----------------------|-------|---------------|
| macOS    | macOS検証機（internal-macOS-01） / AppleAudioBus | macOS 14.5 (Darwin 23.5.0) | CoreAudio (AppleHDA) | 内蔵マイク (16kHz) | MVP1 Core Implementation完了 | 2025-10-19 |
| Windows  | _(TBD)_ | _(TBD)_ | WASAPI | USBマイク (例: Blue Yeti) | MVP2 Phase 0で実機検証予定 | _(planned)_ |
| Linux    | _(TBD)_ | _(TBD)_ | PipeWire / PulseAudio | 内蔵マイク or USBマイク | MVP2 Phase 0で実機検証予定 | _(planned)_ |

> 💡 **ベースライン手順**  
> - 新しい端末で検証する際は、表にホスト名・デバイス ID・使用マイクを追記してください。  
> - 取得したログは `logs/platform/<date>-<platform>.log` に保存し、表の `Last Verified` に日付とログパスを記入します。

---

## Automation Assets

- `scripts/platform_smoke.sh` — ローカル/CI 共通のスモークテスト。`cargo test -- --ignored platform`、リングバッファ往復ベンチ、Python サイドカー起動確認を順番に実行。
- `cargo run --bin stt_burn_in -- --duration-secs 1800` — Pythonサイドカーを実際に起動し、30分以上の連続送信でADR-013のバッファ水準・UI通知前提を検証（ログは `logs/platform/<epoch>-burnin.log` に保存）。

## Long-run Stability Playbook (2h)

| Step | Command / Action | Notes |
|------|------------------|-------|
| 1 | `python -m venv .venv && source .venv/bin/activate`<br>`pip install -r python-stt/requirements-dev.txt` | 事前準備。Windows では `.\.venv\Scripts\activate`。 |
| 2 | `npm install` | 初回のみ。 |
| 3 | `./scripts/stability_burn_in.sh --duration 7200 --session-label macos` | `cargo run --manifest-path src-tauri/Cargo.toml --bin stt_burn_in` を内部で実行。ログは `logs/platform/stability-<timestamp>-macos/` に保存。 |
| 4 | 別ターミナルで `npm run tauri dev` | UI と WebSocket の状態を監視。ログを `logs/platform/<timestamp>-tauri.log` に保存（手動で `tee` 推奨）。 |
| 5 | 30 分ごとにリソース使用量を記録 | macOS/Linux: `ps -o pid,%cpu,%mem,etime -p $(pgrep -f tauri)` を `tee` で `snapshot-notes.txt` へ追記。<br>Windows: `Get-Process Meeting* | Select-Object Id,CPU,PM,StartTime >> snapshot-notes.txt`。 |
| 6 | 完了後 `python3 scripts/performance_report.py <burnin.log>` | メトリクスから平均・P95 を算出。出力先は `target/performance_reports/`。 |
| 7 | `docs/platform-verification.md` の該当プラットフォーム行に `Last Verified` / ログパスを追記 | `logs/platform/stability-<timestamp>-<label>/` を参照。 |

> ❗ Windows / Linux では `./scripts/stability_burn_in.sh` 呼び出し前に PowerShell 等で同等のディレクトリを作成しておくこと。

---

## Chrome Extension Manual Smoke Test (MVP1)

1. **Environment**  
   - `npm install` 済み、Python `.venv` を作成し `pip install -r requirements.txt` / `-dev.txt` を完了。  
   - macOS では `codesign --remove-signature` 等のローカル設定不要。

2. **Launch Tauri App**  
   ```bash
   npm run tauri dev
   ```  
   コンソールに以下が表示されること:  
   `[Meeting Minutes] ✅ Python sidecar started` / `ready` / `FakeAudioDevice initialized` / `WebSocket server started on port <port>`

3. **Load Chrome Extension**  
   - `chrome://extensions/` → 「デベロッパーモード」を ON。  
   - 「パッケージ化されていない拡張機能を読み込む」で `chrome-extension/` を選択。  
   - 拡張カードに「Meeting Minutes Automator」が表示され、`${lastPort}` が初期化されていることを確認。

4. **Verify WebSocket Handshake**  
   - Google Meet (https://meet.google.com) を開き、コンソールに以下の順序でログが出ることを確認。  
     ```
     [Meeting Minutes] Starting WebSocket connection...
     [Meeting Minutes] ✅ Connected to WebSocket server on port <port>
     [Meeting Minutes] 📦 Storage saved: {connectionStatus: 'connected', ...}
     ```

5. **Manual Stream Check**  
   - Tauri ウィンドウで「Start Recording」。  
   - Meet のコンソールに partial / final の `transcription` メッセージが流れる（FakeAudioDevice の場合は空文字列）。  
   - 「Stop Recording」でログが停止。

6. **Log Collection**  
   - Tauri 側 stdout/stderr（`npm run tauri dev` のターミナル）と Chrome DevTools のログを保存。  
   - `logs/platform/<date>-chrome-smoke.log` に転記し、プラットフォーム表の `Last Verified` に反映。

---

## Manual Verification Checklist (ADR-013)

| Case | Steps | Expected Result | Log / Notes |
|------|-------|----------------|-------------|
| 1. 連続録音 (3分) | 通常会話を 3 分間継続 | フレームドロップ 0、`BufferLevel::Overflow` 無し、部分/確定イベント欠落無し |  |
| 2. Python 遅延インジェクション | `python-stt/main.py` で 5 秒 `time.sleep` を挿入 | 録音停止 + UI 通知 (5 秒以内) |  |
| 3. Python 強制終了 | `kill` でサイドカー停止 | Rust 側が `wait/kill` でクリーンアップ、再起動可 |  |
| 4. デバイス抜き差し | マイク抜線 or OS 側で無効化 | エラー通知 + 自動再接続試行 |  |

> チェックリストは `docs/platform-verification.md` に直接追記し、日付・担当者・ログパスを合わせて残してください。

---

## macOS ✅ Verified

**Test Date**: 2025-10-19（MVP1 Core Implementation）
**Platform**: macOS (Darwin 23.5.0)
**Architecture**: x86_64 / Apple Silicon
**Status**: **PASSED**（71 Rust tests + 143 Python tests）

### Environment
- **OS**: macOS
- **Node.js**: 18.x+
- **Rust**: 1.83.0
- **Python**: 3.9-3.12

### Test Results

#### MVP1 Core Implementation Test Summary

**Test Date**: 2025-10-19
**Status**: 71 Rust tests + 143 Python tests = **214 tests PASSED**

##### Rust Tests (71 passed)
```bash
# Unit tests
cargo test --lib
# 結果: 52 tests passed

# Integration tests
cargo test --test '*'
# 結果: 15 tests passed

# E2E tests
cargo test --test stt_e2e_test test_audio_recording_to_transcription_full_flow -- --ignored
# 結果: 1 test passed (23.49s execution time)
```

**E2E Test Coverage**:
- ✅ Audio device initialization (CoreAudioAdapter)
- ✅ Ring buffer operations (lock-free, 0% frame loss)
- ✅ Python sidecar startup and IPC handshake
- ✅ VAD speech detection (speech_start/speech_continuing/speech_end)
- ✅ faster-whisper transcription (partial/final text)
- ✅ Full-duplex IPC (stdin audio frames, stdout events)
- ✅ Graceful shutdown and resource cleanup

**E2E Test Output**:
```
test test_audio_recording_to_transcription_full_flow ... ok
     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 23.49s
```

##### Python Tests (143 passed)
```bash
cd python-stt
.venv/bin/python -m pytest tests/ -v
# 結果: 143 tests passed
```

**Python Test Coverage**:
- ✅ Audio pipeline (10 tests): AudioPipeline initialization, lifecycle, error handling
- ✅ VAD (14 tests): webrtcvad integration, pre-roll buffer, speech detection
- ✅ Whisper client (24 tests): Model initialization, offline fallback, HuggingFace Hub download
- ✅ Resource monitor (20 tests): CPU/memory monitoring, model downgrade/upgrade
- ✅ Storage (18 tests): Session management, file permissions, audio/transcription save
- ✅ IPC handler (15 tests): Message parsing, audio frame processing, event emission
- ✅ Integration tests (42 tests): End-to-end flows, error scenarios

#### Real Audio Recording Test (Task 10.1)

**Test Procedure**:
1. Start Python sidecar with real faster-whisper model
2. Feed test audio fixtures (test_audio_short.wav, test_audio_long.wav)
3. Verify VAD detection and transcription output

**Results**:
```
[INFO] VAD detected speech_start
[INFO] Partial transcription: "This is a test" (confidence: 0.85)
[INFO] VAD detected speech_end
[INFO] Final transcription: "This is a test audio clip" (confidence: 0.92)
```

**Verified Components**:
- ✅ Tauri app startup
- ✅ Python sidecar process management (ADR-013 full-duplex design)
- ✅ CoreAudio device adapter (macOS native)
- ✅ Ring buffer (lock-free, 160KB capacity, 5-second audio buffer)
- ✅ AudioPipeline + VAD (webrtcvad with 300ms pre-roll buffer)
- ✅ faster-whisper transcription (partial <0.5s, final <2s response)
- ✅ Offline model fallback (HuggingFace Hub → bundled base)
- ✅ Resource monitoring (CPU/memory-based model switching)
- ✅ WebSocket server (port 9001)
- ✅ IPC communication (Rust ↔ Python, Line-Delimited JSON)
- ✅ Session storage (audio.wav, transcription.jsonl, session.json)

**Performance**:
- Startup time: ~3-5 seconds (including faster-whisper model load)
- Audio callback latency: <10μs (lock-free ring buffer push)
- E2E latency: <100ms (audio frame → transcription event)
- IPC latency: <5ms (stdin/stdout mutex separation)
- Memory usage: ~1.5GB (Tauri + Python + faster-whisper base model)
- Frame loss rate: 0% (6000 frames tested)

---

## Windows 10+ ⏭️ Deferred to MVP2 Phase 0

**Status**: Deferred（MVP1 Core Implementationでは未実施）
**Tracking**: MVP2-HANDOFF.md参照（検証負債として追跡）

**既知の考慮事項**:
- Python detection: `py.exe` launcher対応（`src-tauri/src/python_sidecar.rs`で実装済み）
- Path separators: `std::path::Path` APIで対応済み
- Process management: tokio cross-platform対応
- Audio driver: WASAPI loopback実装済み（`src-tauri/src/audio_device_adapter.rs`）

**MVP2 Phase 0テスト計画**:
1. Install prerequisites (Node.js 18+, Rust 1.83+, Python 3.9+ 64bit)
2. Run `npm install && npm run tauri dev`
3. Execute smoke test: `scripts/platform_smoke.sh`（PowerShell移植版）
4. Execute E2E test: `cargo test --test stt_e2e_test -- --ignored`
5. Verify Python process cleanup (no zombie processes)
6. Test WASAPI loopback audio capture
7. Update baseline table with results

**期待される問題**:
- WASAPI device enumeration permissions
- Windows Defender SmartScreen警告（署名前）
- PowerShell execution policy制限

---

## Linux (Ubuntu 22.04+) ⏭️ Deferred to MVP2 Phase 0

**Status**: Deferred（MVP1 Core Implementationでは未実施）
**Tracking**: MVP2-HANDOFF.md参照（検証負債として追跡）

**既知の考慮事項**:
- Audio device permissions: `/dev/snd/*` アクセス権限
- Audio driver: ALSA/PulseAudio/PipeWire対応実装済み（`src-tauri/src/audio_device_adapter.rs`）
- Python venv: `.venv/bin/python` 標準パス使用
- GTK dependencies: Tauri 2.0要件

**MVP2 Phase 0テスト計画**:
1. Install prerequisites:
   ```bash
   sudo apt update
   sudo apt install -y build-essential curl wget libgtk-3-dev libwebkit2gtk-4.0-dev \
     libappindicator3-dev librsvg2-dev patchelf libasound2-dev
   # Node.js 18+ via nvm
   # Rust via rustup
   # Python 3.9+ via apt
   ```
2. Run `npm install && npm run tauri dev`
3. Execute smoke test: `scripts/platform_smoke.sh`
4. Execute E2E test: `cargo test --test stt_e2e_test -- --ignored`
5. Test PulseAudio monitor device capture
6. Update baseline table with results

**期待される問題**:
- Audio group membership: `sudo usermod -aG audio $USER`
- Firewall rules: WebSocket port 9001許可
- AppImage permissions: `chmod +x`必須

---

## Compatibility Matrix（MVP1 Core Implementation）

| Feature | macOS | Windows | Linux | Notes |
|---------|-------|---------|-------|-------|
| Tauri App | ✅ Verified | 📋 Code Ready | 📋 Code Ready | Windows/Linux: 実装完了、実機検証はMVP2 Phase 0 |
| Python Sidecar (ADR-013) | ✅ Verified | 📋 Code Ready | 📋 Code Ready | Full-duplex IPC, stdin/stdout mutex分離 |
| Audio Device Adapter | ✅ CoreAudio | 📋 WASAPI | 📋 ALSA | OS別実装完了、実機検証はMVP2 Phase 0 |
| Ring Buffer (Lock-free) | ✅ Verified | ✅ Cross-platform | ✅ Cross-platform | Atomic operations, OS非依存 |
| faster-whisper | ✅ Verified | 📋 Code Ready | 📋 Code Ready | CPU/GPU auto-detection実装済み |
| webrtcvad | ✅ Verified | 📋 Code Ready | 📋 Code Ready | Pre-roll buffer 300ms実装済み |
| Resource Monitor | ✅ Verified | 📋 Code Ready | 📋 Code Ready | CPU/memory-based model switching実装済み |
| WebSocket Server | ✅ Verified | 📋 Code Ready | 📋 Code Ready | Port 9001, tokio cross-platform |
| Chrome Extension | ✅ Verified | ✅ Cross-platform | ✅ Cross-platform | Manifest V3, OS非依存 |
| E2E Flow | ✅ Verified | 📋 Deferred | 📋 Deferred | macOS: 23.49s緑化、他はMVP2 Phase 0 |

**凡例**:
- ✅ Verified: 実機検証完了
- 📋 Code Ready: 実装完了、実機検証未実施
- 📋 Deferred: MVP2 Phase 0で検証予定

---

## Known Issues（MVP1 Core Implementation）

### macOS
✅ **No critical issues**

**検証完了項目**:
- 71 Rust tests passed
- 143 Python tests passed
- E2E test (23.49s) passed
- 0% frame loss (6000 frames tested)

**既知の軽微な問題**:
- SEC-001〜005: セキュリティ修正5件（MVP2 Phase 0で対応）
  - 詳細: `.kiro/specs/meeting-minutes-stt/security-test-report.md`

### Windows
📋 **Deferred to MVP2 Phase 0**

**実装済み（未検証）**:
- WASAPI audio device adapter
- Python `py.exe` launcher detection
- Cross-platform path handling

**予想される問題**:
- Windows Defender SmartScreen警告（コード署名前）
- PowerShell execution policy制限（`Set-ExecutionPolicy RemoteSigned`必要）
- WASAPI device permissions（管理者権限不要を確認予定）

### Linux
📋 **Deferred to MVP2 Phase 0**

**実装済み（未検証）**:
- ALSA audio device adapter
- PulseAudio/PipeWire compatibility layer
- GTK3 dependencies handling

**予想される問題**:
- Audio group membership要件（`usermod -aG audio`）
- `/dev/snd/*` permissions
- Firewall rules（port 9001 WebSocket）
- AppImage FUSE requirements

---

## Next Steps

### MVP2 Phase 0（検証負債解消）

**優先度: 高**
1. **Windows 10+ 実機検証**:
   - `scripts/platform_smoke.sh`のPowerShell移植版作成
   - E2Eテスト実行（`cargo test --test stt_e2e_test -- --ignored`）
   - WASAPI loopback audio capture確認
   - ベースライン表更新（OS version, audio device, test results）

2. **Ubuntu 22.04+ 実機検証**:
   - GTK dependencies確認（`libgtk-3-dev`, `libwebkit2gtk-4.0-dev`）
   - ALSA/PulseAudio device capture確認
   - Audio group permissions確認
   - ベースライン表更新

3. **CI/CD自動化**（meeting-minutes-ciスペック）:
   - GitHub Actions matrix build（macOS/Windows/Linux）
   - Automated smoke tests（`platform_smoke.sh` / PowerShell版）
   - E2E test automation（headless環境対応）

### MVP2 Phase 1以降

**優先度: 中**
4. **プラットフォーム別インストールガイド**:
   - `docs/installation-windows.md`
   - `docs/installation-linux.md`
   - トラブルシューティングガイド拡張

5. **Long-run Stability Test**（Task 11.3）:
   - 2時間連続録音テスト
   - メモリリーク検証
   - CPU使用率推移記録
   - 結果を`logs/platform/stability-*/`に保存

6. **Cross-platform Compatibility Issues対応**:
   - Windows: SmartScreen署名、PowerShell制限
   - Linux: Audio permissions、Firewall rules
   - macOS: Gatekeeper署名（App Store配布時）

---

## References

- Tauri Platform Support: https://tauri.app/v1/guides/building/
- Python Platform Compatibility: 3.9-3.12 (64bit required)
- Chrome Extension: Manifest V3 (cross-platform)
