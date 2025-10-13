# Platform Verification Report

## Overview

Cross-platform compatibility verification for Meeting Minutes Automator.

**Target Platforms**: macOS, Windows 10+, Linux (Ubuntu 20.04+)

---

## Baseline Environments

| Platform | Hostname / Device ID | OS & Build | Audio Driver / Stack | Primary Input Device | Notes | Last Verified |
|----------|----------------------|------------|----------------------|----------------------|-------|---------------|
| macOS    | macOS検証機（internal-macOS-01） / AppleAudioBus | macOS 14.5 (Darwin 23.5.0) | CoreAudio (AppleHDA) | 内蔵マイク (16kHz) | 基本検証用 | 2025-10-05 |
| Windows  | _(TBD)_ | _(TBD)_ | WASAPI | USBマイク (例: Blue Yeti) | ADR-013 実装後に追記 | _(planned)_ |
| Linux    | _(TBD)_ | _(TBD)_ | PipeWire / PulseAudio | 内蔵マイク or USBマイク | ADR-013 実装後に追記 | _(planned)_ |

> 💡 **ベースライン手順**  
> - 新しい端末で検証する際は、表にホスト名・デバイス ID・使用マイクを追記してください。  
> - 取得したログは `logs/platform/<date>-<platform>.log` に保存し、表の `Last Verified` に日付とログパスを記入します。

---

## Automation Assets

- `scripts/platform_smoke.sh` — ローカル/CI 共通のスモークテスト。`cargo test -- --ignored platform`、リングバッファ往復ベンチ、Python サイドカー起動確認を順番に実行。
- `cargo run --bin stt_burn_in -- --duration-secs 1800` — Pythonサイドカーを実際に起動し、30分以上の連続送信でADR-013のバッファ水準・UI通知前提を検証（ログは `logs/platform/<epoch>-burnin.log` に保存）。

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

**Test Date**: 2025-10-05
**Platform**: macOS (Darwin 23.5.0)
**Architecture**: x86_64 / Apple Silicon
**Status**: **PASSED**

### Environment
- **OS**: macOS
- **Node.js**: 18.x+
- **Rust**: 1.83.0
- **Python**: 3.9-3.12

### Test Results

#### E2E-9.3.1: Full E2E Flow
✅ **PASSED**

**Test Procedure**:
1. Start Tauri app: `npm run tauri dev`
2. Load Chrome extension
3. Navigate to Google Meet
4. Click "Start Recording"
5. Verify transcription messages in Chrome Console
6. Click "Stop Recording"

**Results**:
```
[Meeting Minutes] ✅ Python sidecar started
[Meeting Minutes] ✅ Python sidecar ready
[Meeting Minutes] ✅ FakeAudioDevice initialized
[Meeting Minutes] ✅ WebSocket server started on port 9001
```

Chrome Console output:
```
[Meeting Minutes] ✅ Connected to WebSocket server on port 9001
[Meeting Minutes] 📝 Transcription: This is a fake transcription result
```

**Verified Components**:
- ✅ Tauri app startup
- ✅ Python sidecar process management
- ✅ FakeAudioDevice (100ms interval timing)
- ✅ WebSocket server (port 9001)
- ✅ Chrome extension connection
- ✅ IPC communication (Rust ↔ Python)
- ✅ WebSocket messaging (Rust ↔ Chrome)
- ✅ Recording start/stop

**Performance**:
- Startup time: ~2-3 seconds
- WebSocket broadcast latency: <10ms (100ms interval maintained)
- Memory usage: ~150MB (Tauri + Python)

---

## Windows 10+ ⏭️ Not Tested

**Status**: Planned for MVP1
**Tracking**: Refer to `.kiro/specs/meeting-minutes-stt/adrs/ADR-history.md` for ADR-013 implementation progress and follow-up fixes.

**Expected Issues**:
- Python detection: May need to handle `py.exe` launcher
- Path separators: Already handled with `Path` API
- Process management: tokio handles platform differences

**Test Plan** (MVP1):
1. Install prerequisites (Node.js, Rust, Python 64bit)
2. Run `npm install && npm run tauri dev`
3. Execute E2E test procedure
4. Verify Python process cleanup on Windows

---

## Linux (Ubuntu 20.04+) ⏭️ Not Tested

**Status**: Planned for MVP1
**Tracking**: Refer to `.kiro/specs/meeting-minutes-stt/adrs/ADR-history.md` for ADR-013 implementation progress and follow-up fixes.

**Expected Issues**:
- Audio device permissions
- WebSocket firewall rules
- Python venv compatibility

**Test Plan** (MVP1):
1. Install prerequisites via apt/dnf
2. Run `npm install && npm run tauri dev`
3. Execute E2E test procedure
4. Verify GTK dependencies for Tauri

---

## Compatibility Matrix

| Feature | macOS | Windows | Linux |
|---------|-------|---------|-------|
| Tauri App | ✅ | ⏭️ | ⏭️ |
| Python Sidecar | ✅ | ⏭️ | ⏭️ |
| WebSocket Server | ✅ | ⏭️ | ⏭️ |
| Chrome Extension | ✅ | ✅* | ✅* |
| E2E Flow | ✅ | ⏭️ | ⏭️ |

*Chrome extension should work cross-platform (not OS-dependent)

---

## Known Issues

### macOS
- None identified

### Windows
- Not yet tested

### Linux
- Not yet tested

---

## Next Steps

1. **MVP1**: Test on Windows 10+ and Ubuntu 20.04+
2. **MVP1**: Add automated CI/CD tests for all platforms (GitHub Actions matrix)
3. **MVP1**: Align platform verification with ADR-013 implementation milestones (stdin/stdout mutex separation, audio backpressure safeguards)
4. **MVP2**: Document platform-specific installation guides

---

## References

- Tauri Platform Support: https://tauri.app/v1/guides/building/
- Python Platform Compatibility: 3.9-3.12 (64bit required)
- Chrome Extension: Manifest V3 (cross-platform)
