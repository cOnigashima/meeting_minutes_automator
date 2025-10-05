# ADR-001: 録音責務の一元化

## Status
Accepted

## Context
meeting-minutes-stt では、Rust側 `AudioDeviceAdapter` とPython側 `WhisperSTTEngine` の2つのプロセスが音声データを扱います。両方のプロセスが録音機能を持つと、以下の問題が発生します:

1. **レース条件**: 同一デバイスへの同時アクセスによるデバイスロック競合
2. **デバイス排他制御**: OS音声APIが排他アクセスを要求する環境 (Windows WASAPI, macOS CoreAudio) でのエラー
3. **音声データの重複/欠落**: 2つのプロセスが独立して録音すると、同期が困難

## Decision
**録音責務はRust側 `AudioDeviceAdapter` に一元化し、Python側は録音を禁止する。**

## Consequences

### Positive
- レース条件の完全回避
- デバイス排他制御の単純化
- 音声データフローの明確化 (Rust → Python 一方向)

### Negative
- Python側の開発者がこの制約を理解する必要がある
- 既存のPython音声処理ライブラリ (`sounddevice`, `pyaudio`) が使用不可

## Implementation

### 静的解析
- CI/CD パイプラインで禁止パッケージ検出
- `flake8-forbidden-imports` プラグイン使用
- pre-commit フックで自動チェック

**禁止パッケージリスト**:
- `sounddevice`
- `pyaudio`
- `portaudio`
- `soundcard`
- `PySndHdr`

### 依存関係ロック
- `requirements.txt` に録音関連パッケージを含めない
- `pip-compile` で許可リスト (allowlist) 方式

### 違反時の対応
1. CI/CDでビルド失敗
2. エラーメッセージに本ADRへのリンク表示
3. コードレビューで人的確認

### 実装ファイル
- `.pre-commit-config.yaml`: pre-commitフック設定
- `scripts/check_forbidden_imports.py`: 静的解析スクリプト
- `python-stt/requirements.txt`: 録音関連パッケージを除外

## References
- `.kiro/steering/principles.md` - プロセス境界の明確化原則
- `design.md` L153-190 - RealAudioDevice / AudioStreamBridge アーキテクチャ
- `tech.md` L350-352 - Python側の音声処理制約

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成 (録音責務の一元化) |
