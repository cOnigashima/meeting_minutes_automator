# Meeting Minutes Automator プロジェクトサマリー

## TL;DR（結論）

**やろうとしたこと**: Google Meetの音声をローカルWhisperでリアルタイム文字起こし → Google Docsに自動書き込み

**結果**: 断念

**理由**: 音声品質 × Whisper精度 × リアルタイム性 = 全部同時に満たせなかった

---

## プロジェクト概要

**目的**: Google Meetの音声をリアルタイムで文字起こしし、議事録を自動生成するデスクトップアプリ

**期間**: 2025年10月〜2026年1月（約3ヶ月）

**ステータス**: MVP1完了後に断念

**開発手法**: Claude Code + Kiro Spec-Driven Development

---

## なぜ断念したか（詳細）

### 問題1: 音声入力の品質問題

**症状**:
- 文字起こし精度が大きくばらつく
- 「hashashasha...」「設設設設...」「トッキー トッキー...」などの意味不明な出力
- 同じ設定でも動いたり動かなかったり

**原因（推定）**:
- BlackHole経由の音声データに問題？
- サンプルレートの不一致？
- 音量レベルの問題？

**切り分けできなかった理由**:
- 音声データ自体の問題か、Whisperの問題か判別困難
- デバッグ用に音声を保存して聞いても、人間には普通に聞こえる
- でもWhisperは意味不明な出力をする

### 問題2: リアルタイム処理の限界

```
音声バッチ到着: 250ms毎
Whisper処理時間: 2-5秒/バッチ（CPU）
結果: 処理が追いつかない → バックログ蓄積 → 遅延増大
```

**試した解決策**:

| 方法 | 結果 |
|------|------|
| tinyモデル | 速いけど精度が壊滅的 |
| smallモデル | バランス良いけどまだ遅い |
| VAD前処理 | 無音スキップで多少改善 |
| MLX-Whisper（GPU） | 7.5x高速化！でも... |

### 問題3: MLX-Whisperの罠

**期待**: Apple Silicon GPUで高速化 → リアルタイム処理可能に

**現実**:
```
MLX + tinyモデル = 速い（668ms/5秒音声）けどハルシネーション地獄
MLX + smallモデル = HuggingFace認証必要（ダウンロードできず）
```

**ハルシネーションの例**:
```
実際の発話: 「こんにちは、今日の議題は...」
Whisper出力: 「hashashashashashashashashashasha...」
```

### 問題4: BlackHole設定の複雑さ

macOSでシステム音声をキャプチャするにはBlackHole仮想オーディオデバイスが必要。

**ユーザーに求める設定**:
1. BlackHoleをインストール
2. Audio MIDI Setupを開く
3. Multi-Output Deviceを作成
4. BlackHole 2ch + 実際のスピーカーを追加
5. システム出力をMulti-Output Deviceに変更

→ 一般ユーザーには無理

---

## やったことの全体像

### アーキテクチャ（3層構成）

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
│  ・VAD（webrtcvad）- 音声区間検出                            │
│  ・Whisper文字起こし（faster-whisper / mlx-whisper）         │
│  ・リソースモニタリング                                      │
└─────────────────────────────────────────────────────────────┘
```

### なぜこの構成にしたか

| 選択 | 理由 | 結果 |
|------|------|------|
| Tauri | Electron比で軽量、Rustで音声処理が高速 | ◎ 良かった |
| Chrome拡張 | tabCapture APIでMeet音声を直接取得 | ○ 動いた |
| Python sidecar | Whisperエコシステムへのアクセス | △ IPC複雑 |
| プロセス分離 | 安定性（Python crash時もUIは生存） | ◎ 良かった |

### 開発フェーズ

```
MVP0: Walking Skeleton（2025年10月）✅ 完了
  └─ Tauri + Chrome拡張 + Pythonの最小疎通確認

MVP1: Real STT（2025年10月〜2026年1月）✅ 完了（でも実用レベルに達せず）
  ├─ faster-whisper統合
  ├─ webrtcvad統合
  ├─ リソースベースモデル選択
  ├─ 音声デバイス管理
  └─ Multi-Input Mode（マイク + システム音声）

MVP2: Google Docs同期 ❌ 未着手
MVP3: LLM要約 ❌ 未着手
```

---

## 技術詳細

### システム音声キャプチャ（macOS）

**問題**: macOSはシステム音声の直接キャプチャを許可しない

**解決策**: BlackHole仮想オーディオデバイス
```
System Audio → Multi-Output Device → BlackHole 2ch（キャプチャ用）
                                   → 実際のスピーカー（聞く用）
```

### Multi-Input Mode

**目的**: 自分の声（マイク）+ 相手の声（システム音声）を同時取得

**実装**:
```rust
// InputMixer: 2つの音声ストリームをリアルタイム合成
pub struct InputMixer {
    mic_buffer: RingBuffer,
    loopback_buffer: RingBuffer,
    resampler: Resampler,  // 異なるサンプルレート対応
}

// 合成処理
fn mix_audio(&mut self) -> Vec<i16> {
    let mic = self.mic_buffer.read();
    let loopback = self.loopback_buffer.read();
    // 単純加算（オーバーフロー注意）
    mic.iter().zip(loopback.iter())
       .map(|(a, b)| a.saturating_add(*b))
       .collect()
}
```

### プロセス間通信（IPC）

**設計**: JSON over stdin/stdout

```json
// Rust → Python（リクエスト）
{
  "id": "req-001",
  "type": "request",
  "method": "process_audio_stream",
  "audio_data": [128, 127, 129, ...]  // u8配列
}

// Python → Rust（ストリーミングイベント）
{"type": "event", "eventType": "speech_start", "data": {"timestamp": 1234567890}}
{"type": "event", "eventType": "partial_text", "data": {"text": "こんにちは", "is_final": false}}
{"type": "event", "eventType": "final_text", "data": {"text": "こんにちは、今日は", "is_final": true}}
{"type": "event", "eventType": "speech_end", "data": {"timestamp": 1234567895}}
```

**学び**:
- バイナリ転送はBase64より生のu8配列が効率的
- ストリーミングイベントでリアルタイム感を出せる

### MLX-Whisper実験

**インストール**:
```bash
pip install mlx-whisper
```

**使い方**:
```python
import mlx_whisper

result = mlx_whisper.transcribe(
    audio_float,  # numpy array
    path_or_hf_repo="mlx-community/whisper-tiny",
    language="ja",
    verbose=False,
    condition_on_previous_text=False,  # ループ防止
    compression_ratio_threshold=2.0,   # 繰り返し検出
)
```

**性能比較**:
| バックエンド | モデル | 5秒音声処理時間 | リアルタイム比 |
|-------------|--------|----------------|---------------|
| faster-whisper (CPU) | small | 2-5秒 | 0.4-1.0x |
| mlx-whisper (GPU) | tiny | 668ms | 7.5x |

**問題**: tinyモデルのハルシネーション
```
入力: 普通の日本語音声
出力: "hashashashashasha..." / "設設設設設..." / "トッキー トッキー..."
```

---

## Claude Code / Kiro開発体験

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
| `/kiro:spec-requirements` | 要件生成（EARS構文） |
| `/kiro:spec-design` | 設計生成 |
| `/kiro:spec-tasks` | タスク分解 |
| `/kiro:validate-design` | 設計検証 |
| `/kiro:spec-status` | 進捗確認 |

### EARS構文の例

要件定義に使った構文：
```
Ubiquitous: The Recorder shall persist raw audio locally before any network transfer.
Event: When network connectivity is restored, the Syncer shall upload queued minutes within 60s.
State: While free disk space < 500 MB, the system shall block new recordings.
Optional: Where Google Docs integration is enabled, the system shall append minutes.
Unwanted: If OAuth token validation fails, then the system shall abort upload.
```

### カスタムエージェント

```markdown
# .claude/agents/kiro-spec-implementer.md

Kiro仕様駆動開発 + Serena（シンボリックコード解析）+ TDD実装の統合エージェント
- 要件IDトレーサビリティ維持
- 設計原則9項目の自動チェック
- RED → GREEN → REFACTOR サイクル
```

### 良かった点

1. **構造化された開発**: 要件→設計→タスクの流れが明確
2. **トレーサビリティ**: REQ-001 → タスク → コード → テストの紐付け
3. **ADR（Architecture Decision Records）**: なぜその設計にしたか記録が残る
4. **大規模変更に強い**: 仕様から追えるので影響範囲がわかる

### 課題

1. **オーバーヘッド**: 小さな変更でも仕様更新が必要になりがち
2. **コンテキスト制限**: 長いセッションでトークン上限に達する
3. **ツール間連携**: Serena + Kiro + Claude Codeの組み合わせは複雑
4. **学習コスト**: EARS構文やADRの書き方を覚える必要あり

### 個人プロジェクトには...

**正直オーバーキルだった**

- 一人で開発するなら仕様書なくても頭の中にある
- でも「AIに開発させる」なら仕様書は必要
- AIは忘れるので、コンテキストとして仕様を渡す必要がある

---

## 統計

| 項目 | 数値 |
|------|------|
| 開発期間 | 約3ヶ月 |
| コミット数 | 約50 |
| ファイル数 | 約100 |
| Rust LOC | 約3,000 |
| Python LOC | 約2,000 |
| TypeScript LOC | 約1,500 |
| テスト数 | 267（18失敗） |
| 仕様ファイル | 15+ |
| ADR数 | 17 |

---

## 学んだこと

### 技術面

1. **ローカルSTTは難しい**
   - リアルタイム性と精度のトレードオフがキツい
   - クラウドSTT（Whisper API、Google STT）のほうが現実的かも

2. **音声処理は沼**
   - サンプルレート、バッファサイズ、レイテンシ...変数が多すぎ
   - 「動いた」と「安定して動く」の間に深い溝

3. **macOS音声キャプチャは面倒**
   - システム音声を取るにはBlackHoleが必要
   - ユーザーにAudio MIDI Setup設定させるのは無理筋

4. **MLX-Whisperは可能性あり**
   - Apple Silicon GPUで7.5x高速化は魅力的
   - でもモデル認証問題とハルシネーション問題がある

### プロセス面

1. **早期プロトタイプ重要**
   - 音声品質問題は最初に気づくべきだった
   - 仕様書書く前に動くもの作って検証すべき

2. **Spec-Drivenは大規模/チーム向け**
   - 個人開発には重い
   - でもAI開発ではコンテキスト維持に有用

3. **ユーザー設定は最小化すべき**
   - BlackHole設定は一般ユーザーには無理
   - 「インストールして即使える」が理想

---

## もし続けるなら

### 方向性A: クラウドSTTに切り替え
```
ローカルWhisper → Whisper API / Google Speech-to-Text
```
- 精度・速度問題が解決
- でもコストと通信が発生

### 方向性B: リアルタイムを諦める
```
リアルタイム文字起こし → 録音後バッチ処理
```
- ローカルWhisperでも精度出せる
- でも「リアルタイム議事録」のコンセプトが崩れる

### 方向性C: 音声品質問題を解決
```
BlackHole → 別の方法？ / 設定自動化？
```
- 根本原因を特定できれば...
- でも時間かかりそう

---

## リポジトリ

- **GitHub**: [meeting_minutes_automator](https://github.com/cOnigashima/meeting_minutes_automator)
- **技術スタック**: Tauri 2.0 + React + Rust + Python + Chrome Extension

---

## 関連ファイル

プロジェクト内の参考になるファイル：

| ファイル | 内容 |
|----------|------|
| `.kiro/specs/meeting-minutes-stt/requirements.md` | EARS構文の要件定義例 |
| `.kiro/specs/meeting-minutes-stt/design.md` | 設計ドキュメント |
| `.kiro/steering/principles.md` | 設計原則（5つ） |
| `docs/experiments/mlx-whisper-experiment.md` | MLX実験ログ |
| `src-tauri/src/input_mixer.rs` | Multi-Input実装 |
| `python-stt/stt_engine/transcription/whisper_client.py` | Whisperクライアント |

---

## 謝辞

- **Claude Code / Kiro**: AIペアプログラミング
- **faster-whisper / mlx-whisper**: Whisper実装
- **BlackHole**: 仮想オーディオデバイス
- **Tauri**: 軽量デスクトップフレームワーク
