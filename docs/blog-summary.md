# Meeting Minutes Automator プロジェクトサマリー

## プロジェクト概要

**目的**: Google Meetの音声をリアルタイムで文字起こしし、議事録を自動生成するデスクトップアプリ

**期間**: 2025年10月〜2026年1月（約3ヶ月）

**ステータス**: MVP1完了、実用には課題あり

---

## アーキテクチャ

### 3層構成

```
┌─────────────────────────────────────────────────────────────┐
│                    Chrome Extension                          │
│  ・Google Meetタブ検出                                       │
│  ・音声キャプチャ（tabCapture API）                          │
│  ・WebSocket通信                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket (port 19006)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Tauri (Rust)                            │
│  ・音声デバイス管理（cpal）                                  │
│  ・リングバッファ / InputMixer                               │
│  ・Python sidecar管理                                        │
│  ・React UI提供                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ stdin/stdout IPC (JSON)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Python Sidecar                             │
│  ・VAD（webrtcvad）                                          │
│  ・Whisper文字起こし（faster-whisper）                       │
│  ・リソースモニタリング                                      │
└─────────────────────────────────────────────────────────────┘
```

### なぜこの構成？

| 選択 | 理由 |
|------|------|
| Tauri | Electron比で軽量、Rustで音声処理が高速 |
| Chrome拡張 | tabCapture APIでMeet音声を直接取得 |
| Python sidecar | Whisperエコシステムへのアクセス、ML処理 |
| プロセス分離 | 安定性（Python crash時もUIは生存） |

---

## 開発フェーズ

### MVP0: Walking Skeleton（2025年10月）
- Tauri + Chrome拡張 + Pythonの最小疎通確認
- Fake実装でパイプライン検証

### MVP1: Real STT（2025年10月〜2026年1月）
- faster-whisper統合
- webrtcvad統合
- リソースベースモデル選択
- 音声デバイス管理
- Multi-Input Mode（マイク + システム音声）

### 未実装
- MVP2: Google Docs同期
- MVP3: LLM要約

---

## 技術的チャレンジ

### 1. システム音声キャプチャ（macOS）

**問題**: macOSはシステム音声の直接キャプチャを許可しない

**解決策**: BlackHole仮想オーディオデバイス
```
System Audio → Multi-Output Device → BlackHole 2ch
                                   → 実際のスピーカー
```

**課題**: 設定が複雑、ユーザーに Audio MIDI Setup での手動設定が必要

### 2. リアルタイム処理のボトルネック

**問題**:
- 音声バッチ到着: 250ms毎
- Whisper処理（CPU）: 2-5秒/バッチ
- 結果: バックログ蓄積、処理遅延

**試した解決策**:
1. モデルサイズ調整（tiny/small/medium）
2. VAD前処理で無音区間スキップ
3. MLX-Whisper（Apple Silicon GPU）← 7.5x高速化するも精度問題

### 3. Multi-Input Mode

**目的**: 自分の声（マイク）+ 相手の声（システム音声）を同時取得

**実装**:
```rust
// InputMixer: 2つの音声ストリームをリアルタイム合成
pub struct InputMixer {
    mic_buffer: RingBuffer,
    loopback_buffer: RingBuffer,
    resampler: Resampler,  // 異なるサンプルレート対応
}
```

**課題**: 音声品質、同期、エコー

### 4. プロセス間通信（IPC）

**設計**: JSON over stdin/stdout
```json
// Request
{"id": "1", "type": "request", "method": "process_audio_stream", "audio_data": [...]}

// Response (streaming events)
{"type": "event", "eventType": "partial_text", "data": {"text": "こんにちは"}}
{"type": "event", "eventType": "final_text", "data": {"text": "こんにちは、今日は"}}
```

**学び**:
- シンプルなプロトコルが安定
- バイナリ転送はBase64よりu8配列が効率的

---

## Kiro/Claude Code開発体験

### Spec-Driven Development

```
.kiro/
├── steering/           # プロジェクト全体のガイド
│   ├── product.md      # 製品コンテキスト
│   ├── tech.md         # 技術スタック
│   ├── structure.md    # コード構造
│   └── principles.md   # 設計原則
└── specs/
    └── meeting-minutes-stt/
        ├── requirements.md  # 要件（EARS構文）
        ├── design.md        # 設計
        └── tasks.md         # タスク分解
```

### 使用したコマンド

| コマンド | 用途 |
|----------|------|
| `/kiro:spec-init` | 仕様初期化 |
| `/kiro:spec-requirements` | 要件生成 |
| `/kiro:spec-design` | 設計生成 |
| `/kiro:spec-tasks` | タスク分解 |
| `/kiro:validate-design` | 設計検証 |

### カスタムエージェント

- **kiro-spec-implementer**: TDD実装 + Serena統合
- **kiro-spec-guardian**: 仕様整合性チェック
- **docs-gardener**: ドキュメント同期

### 良かった点

1. **構造化された開発**: 要件→設計→タスクの流れが明確
2. **トレーサビリティ**: REQ-001 → タスク → コード → テスト
3. **ADR（Architecture Decision Records）**: 設計判断の記録

### 課題

1. **オーバーヘッド**: 小さな変更でも仕様更新が必要
2. **コンテキスト制限**: 長いセッションでメモリ圧迫
3. **ツール間連携**: Serena + cc-sddの組み合わせは複雑

---

## 学んだこと

### 技術

1. **音声処理は難しい**: サンプルレート、バッファサイズ、レイテンシのトレードオフ
2. **リアルタイムMLは難しい**: 処理速度 vs 精度のバランス
3. **macOS音声制限**: システム音声キャプチャには回避策が必要
4. **MLX-Whisper**: 速度は良いがtinyモデルはハルシネーション多発

### プロセス

1. **Spec-Drivenは大規模向け**: 小さなプロジェクトにはオーバーキル
2. **早期プロトタイプ重要**: 音声品質問題は早く発見すべきだった
3. **ユーザー設定最小化**: BlackHole設定はハードル高すぎ

---

## 統計

| 項目 | 数値 |
|------|------|
| コミット数 | 約50 |
| ファイル数 | 約100 |
| Rust LOC | 約3,000 |
| Python LOC | 約2,000 |
| TypeScript LOC | 約1,500 |
| テスト数 | 267（18失敗） |

---

## 今後の可能性

### 短期
- [ ] 音声入力品質の調査・改善
- [ ] HuggingFace認証でMLX smallモデル使用
- [ ] テスト失敗18件の修正

### 中期
- [ ] Google Docs連携（MVP2）
- [ ] LLM要約機能（MVP3）

### 長期
- [ ] クラウドSTTオプション（Whisper API等）
- [ ] 話者分離（Speaker Diarization）
- [ ] 多言語対応

---

## リポジトリ

- **GitHub**: [meeting_minutes_automator](https://github.com/cOnigashima/meeting_minutes_automator)
- **技術スタック**: Tauri 2.0 + React + Rust + Python + Chrome Extension

---

## 謝辞

- Claude Code / Kiro Spec-Driven Development
- faster-whisper / mlx-whisper
- BlackHole (Existential Audio)
