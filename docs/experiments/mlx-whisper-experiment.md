# MLX-Whisper実験ログ

**日付**: 2026-01-10
**ステータス**: 中断（音声入力品質の調査が必要）

## 目的

Apple Silicon（M1 Max）でGPU加速されたWhisper処理を実現し、リアルタイム文字起こしの性能を向上させる。

## 背景

- faster-whisper（CPU）は2-5秒/バッチの処理時間
- Multi-Input Modeでは250msごとにバッチが到着
- 処理が追いつかずバックログが蓄積

## 実装

### 1. インストール

```bash
pip install mlx-whisper
```

### 2. whisper_client.py の変更

```python
# インポート追加
import mlx_whisper

class WhisperSTTEngine:
    def __init__(
        self,
        model_size: Optional[ModelSize] = None,
        auto_select_model: bool = False,
        offline_mode: bool = False,
        use_mlx: bool = False  # MLX backend for Apple Silicon GPU acceleration
    ):
        self.use_mlx: bool = use_mlx
        # ...

    def initialize(self) -> None:
        if self.use_mlx:
            mlx_model_name = f"mlx-community/whisper-{self.model_size}"
            self.model_path = mlx_model_name
            logger.info(f"MLX backend enabled, model will be loaded on first use: {mlx_model_name}")
            # Output ready message for Tauri sidecar protocol
            print(json.dumps({
                "type": "system",
                "event": "whisper_model_ready",
                "model_size": self.model_size,
                "model_path": mlx_model_name
            }), flush=True)
            return
        # ... faster-whisper initialization

    async def transcribe(self, audio_data: bytes, sample_rate: int = 16000, is_final: bool = True) -> dict:
        # ... audio preprocessing ...

        if self.use_mlx:
            mlx_model_name = f"mlx-community/whisper-{self.model_size}"
            result = mlx_whisper.transcribe(
                audio_float,
                path_or_hf_repo=mlx_model_name,
                language="ja",  # Force Japanese
                verbose=False,
                condition_on_previous_text=False,  # Prevent repetition loops
                compression_ratio_threshold=2.0,   # Stricter repetition detection
            )
            full_text = result.get("text", "").strip()
            segment_count = len(result.get("segments", []))
            total_logprob = -0.3  # Approximate
            detected_language = result.get("language", "ja")
        else:
            # ... faster-whisper implementation
```

### 3. main.py の変更

```python
# MLX backend for Apple Silicon GPU acceleration
self.stt_engine = WhisperSTTEngine(
    model_size="tiny",  # or "small" if authenticated
    auto_select_model=False,
    use_mlx=True
)
```

## 結果

### 性能テスト（tinyモデル）

| メトリクス | faster-whisper (CPU) | mlx-whisper (GPU) |
|-----------|---------------------|-------------------|
| 5秒音声処理 | 2-5秒 | 668ms |
| リアルタイム比 | 0.4-1.0x | 7.5x |

### 発見された問題

1. **ハルシネーション**: tinyモデルはノイズや不明瞭な音声で繰り返しパターンを生成
   - 例: "hashashasha...", "設設設設...", "トッキー トッキー..."
   - `condition_on_previous_text=False` で軽減するも完全には解決せず

2. **モデル認証**: smallモデル以上はHuggingFace認証が必要
   - `mlx-community/whisper-tiny` は公開（認証不要）
   - `mlx-community/whisper-small` は認証必要

3. **音声入力品質**: 文字起こし精度が全般的に低下
   - BlackHole経由の音声に問題がある可能性
   - サンプルレートやフォーマットの不一致の可能性

## 利用可能なMLX Whisperモデル

| モデル | パラメータ | 認証 |
|--------|-----------|------|
| tiny | 39M | 不要 |
| base | 74M | 要確認 |
| small | 244M | 必要 |
| medium | 769M | 必要 |
| large-v3 | 1.5B | 必要 |

## 次のステップ

1. **音声入力の調査**: BlackHole設定、サンプルレート確認
2. **HuggingFace認証**: `huggingface-cli login` でsmallモデルを使用可能に
3. **ハイブリッド方式**:
   - MLX small（認証後）で高精度処理
   - faster-whisper をフォールバックとして維持

## 参考

- [mlx-whisper GitHub](https://github.com/ml-explore/mlx-examples/tree/main/whisper)
- [MLX Community Models](https://huggingface.co/mlx-community)
