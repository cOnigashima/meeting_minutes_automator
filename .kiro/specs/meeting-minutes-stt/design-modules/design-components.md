## Components and Interfaces

### 音声処理ドメイン

#### RealAudioDevice (Rust)

**責任と境界**
- **主要責任**: OS固有の音声APIからの音声データ取得とフォーマット変換 (**システム唯一の録音責任者**)
- **ドメイン境界**: 音声デバイス抽象化レイヤー
- **データ所有**: 音声デバイス設定、音声バッファ
- **トランザクション境界**: 単一音声セッションスコープ

**重要な設計決定**:
- **録音責務の一元化（ADR-001準拠）**: 音声録音はRust側AudioDeviceAdapterのみが担当、Python側での録音は静的解析により禁止
- **Python側の制約**: Pythonサイドカーは録音を行わず、Rustから送信されたバイナリストリームの受信とSTT処理のみを実施
- **レース条件の防止**: 複数箇所での録音開始を防ぎ、単一の音声ソースを保証

**依存関係**
- **インバウンド**: AudioStreamBridge、UI設定コンポーネント
- **アウトバウンド**: なし (リーフノード)
- **外部依存**: OS音声API (WASAPI、CoreAudio、ALSA)

**契約定義**

```rust
pub trait AudioDeviceAdapter: Send + Sync {
    async fn list_devices(&self) -> Result<Vec<AudioDevice>>;
    async fn start_capture(&mut self, device_id: &str, config: AudioConfig) -> Result<AudioStream>;
    async fn stop_capture(&mut self) -> Result<()>;
}

// OS別実装
pub struct WasapiAdapter { /* Windows WASAPI実装 */ }
pub struct CoreAudioAdapter { /* macOS CoreAudio実装 */ }
pub struct AlsaAdapter { /* Linux ALSA実装 */ }

impl AudioDeviceAdapter for WasapiAdapter {
    async fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        // WASAPI経由でデバイス列挙
    }
    async fn start_capture(&mut self, device_id: &str, config: AudioConfig) -> Result<AudioStream> {
        // WASAPI loopback modeでシステム音声キャプチャ
    }
    async fn stop_capture(&mut self) -> Result<()> {
        // デバイスリソース解放
    }
}

// 以下、CoreAudioAdapter、AlsaAdapterも同様に実装
```

**AudioDevice型定義**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub is_loopback: bool,  // ループバックデバイスフラグ
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,  // 16000 Hz固定
    pub channels: u8,      // 1 (mono)固定
    pub chunk_size: usize, // 320サンプル (10ms @ 16kHz)
}
```

**クロスプラットフォーム対応**:
- **macOS**: Core Audioフレームワーク経由、BlackHole等の仮想デバイス認識 (STT-REQ-004.3, STT-REQ-004.6)
- **Windows**: WASAPI経由、WASAPI loopback modeでシステム音声キャプチャ (STT-REQ-004.4, STT-REQ-004.7)
- **Linux**: ALSA/PulseAudio経由、PulseAudio monitorデバイスでシステム音声キャプチャ (STT-REQ-004.5, STT-REQ-004.8)

**OS許可ダイアログ**: 録音開始前にOSのマイクアクセス許可を確認し、許可されていない場合は明示的な許可ダイアログを表示 (STT-REQ-004.1, STT-NFR-004.4)

**エラーハンドリング**:
- デバイス初期化失敗: 3秒間隔で最大3回まで再試行 (STT-REQ-001.9)
- デバイス切断: 5秒後に自動再接続を試行 (最大3回) (STT-REQ-004.11)

**参照**: `docs/uml/meeting-minutes-stt/cls/CLS-001_Audio-Device-Adapter.puml`

---

#### AudioStreamBridge (Rust)

**責任と境界**
- **主要責任**: Rust→Python間の音声データIPC転送
- **ドメイン境界**: プロセス間通信層
- **データ所有**: 音声バッファキュー、IPC接続状態
- **トランザクション境界**: 音声チャンク単位

**依存関係**
- **インバウンド**: オーディオ録音コントローラー
- **アウトバウンド**: PythonSidecarManager、RealAudioDevice
- **外部依存**: なし (Rustプロセス内完結)

**契約定義**

```rust
pub struct AudioStreamBridge {
    device_adapter: Box<dyn AudioDeviceAdapter>,
    sidecar_manager: Arc<PythonSidecarManager>,
    buffer_queue: Arc<Mutex<VecDeque<AudioChunk>>>,
}

impl AudioStreamBridge {
    pub async fn start_streaming(&mut self, device_id: &str) -> Result<()> {
        // 1. デバイスアダプターから音声ストリーム取得
        let mut stream = self.device_adapter.start_capture(device_id, default_config()).await?;

        // 2. 音声チャンクをバッファキューに追加
        while let Some(chunk) = stream.next().await {
            self.buffer_queue.lock().await.push_back(chunk);
        }

        // 3. 別タスクでPythonへ非同期送信
        self.flush_to_python().await?;

        Ok(())
    }

    async fn flush_to_python(&self) -> Result<()> {
        while let Some(chunk) = self.buffer_queue.lock().await.pop_front() {
            self.sidecar_manager.send_audio_chunk(&chunk).await?;
        }
        Ok(())
    }
}
```

**バッファ管理戦略**:
- **キューサイズ上限**: 10秒分 (1000チャンク @ 10ms/chunk)
- **オーバーフロー時**: 古いチャンクをドロップ (警告ログ)
- **バックプレッシャー**: Python処理遅延時にサンプルレート削減

**依存関係図**:

```
┌─────────────────────────────┐
│  Audio Recording Controller │
└────────────┬────────────────┘
             │
             ▼
      ┌──────────────────┐
      │ AudioStreamBridge │
      └─────┬────────┬────┘
            │        │
            ▼        ▼
   ┌──────────────┐  ┌───────────────────┐
   │ RealAudio    │  │ PythonSidecar     │
   │ Device       │  │ Manager           │
   └──────┬───────┘  └─────────┬─────────┘
          │                    │
          ▼                    ▼
   ┌──────────────┐     ┌──────────────┐
   │ OS Audio API │     │ Python STT   │
   │ (WASAPI/     │     │ Process      │
   │  CoreAudio)  │     │              │
   └──────────────┘     └──────────────┘
```

---

#### VoiceActivityDetector (Python)

**責任と境界**
- **主要責任**: リアルタイム音声活動検出と発話セグメンテーション
- **ドメイン境界**: 音声解析と発話境界検出
- **データ所有**: VAD設定パラメータと検出履歴
- **トランザクション境界**: 音声チャンク単位の処理

**依存関係**
- **インバウンド**: AudioStreamBridge (IPC経由)
- **アウトバウンド**: WhisperSTTEngine
- **外部依存**: webrtcvad ≥2.0.0、NumPy ≥1.24.0

**契約定義**

```python
class VoiceActivityDetector:
    def __init__(self, aggressiveness: int = 2):
        """
        Args:
            aggressiveness: webrtcvadの積極性 (0-3、デフォルト2=中程度)
        """
        self.vad = webrtcvad.Vad(aggressiveness)
        self.speech_threshold = 0.3  # 発話開始: 0.3秒連続音声
        self.silence_threshold = 0.5  # 発話終了: 0.5秒無音

    async def detect_activity(self, chunk: AudioChunk) -> VadResult:
        """
        音声活動検出を実行

        Args:
            chunk: 10msの音声チャンク (320サンプル @ 16kHz mono)

        Returns:
            VadResult: 音声/無音判定結果
        """
        # 音声データを10ms単位のフレームに分割
        frames = self._split_to_frames(chunk.data, frame_duration_ms=10)

        # 各フレームでVAD判定
        is_speech = self.vad.is_speech(frames[0], sample_rate=16000)

        return VadResult(
            is_speech=is_speech,
            confidence=self._calculate_confidence(frames),
            segment_id=self._get_current_segment_id() if is_speech else None
        )

    async def on_speech_start(self, segment_id: str) -> None:
        """発話開始イベント (0.3秒連続音声検出時)"""
        logger.info(f"Speech started: segment_id={segment_id}")

    async def on_speech_end(self, segment: SpeechSegment) -> None:
        """発話終了イベント (0.5秒無音検出時)"""
        logger.info(f"Speech ended: segment_id={segment.segment_id}, duration={segment.duration}s")
        # WhisperSTTEngineに確定テキスト生成を要求
        await self.stt_engine.transcribe_final(segment)
```

**VadResult型定義**:

```python
from dataclasses import dataclass
from typing import Optional

@dataclass
class VadResult:
    is_speech: bool
    confidence: float
    segment_id: Optional[str]

@dataclass
class SpeechSegment:
    segment_id: str
    start_time: float
    end_time: float
    audio_data: bytes  # 16-bit PCM
```

**発話境界検出ロジック**:
- **発話開始**: 音声フレームが連続して0.3秒以上検出される (STT-REQ-003.4)
- **発話終了**: 無音フレームが連続して0.5秒以上検出される (STT-REQ-003.5)
- **部分テキスト生成**: 発話継続中、1秒間隔でWhisperSTTEngineに累積音声データを送信 (STT-REQ-003.7)

**パフォーマンス要件**:
- 10msフレームごとの判定を1ms以内に完了 (STT-NFR-001.3)

---

#### WhisperSTTEngine (Python)

**責任と境界**
- **主要責任**: 音声セグメントの文字起こしと部分結果生成
- **ドメイン境界**: 音声認識とテキスト変換
- **データ所有**: faster-whisperモデルとトランスクリプション履歴
- **トランザクション境界**: 発話セグメント単位

**依存関係**
- **インバウンド**: VoiceActivityDetector
- **アウトバウンド**: LocalStorageService、WebSocketServer (IPC経由)
- **外部依存**: faster-whisper ≥0.10.0、torch、CTranslate2 ≥3.0

**契約定義**

```python
from faster_whisper import WhisperModel
from typing import Literal

ModelSize = Literal["tiny", "base", "small", "medium", "large-v3"]

class WhisperSTTEngine:
    def __init__(self, model_size: ModelSize = "small"):
        self.model_size = model_size
        self.model: Optional[WhisperModel] = None

    async def initialize(self) -> None:
        """
        faster-whisperモデルのロード (オフラインファースト)

        モデル検出優先順位:
        1. ユーザー設定パス (~/.config/meeting-minutes-automator/whisper_model_path)
        2. HuggingFace Hubキャッシュ (~/.cache/huggingface/hub/models--Systran--faster-whisper-*)
        3. インストーラーバンドルモデル ([app_resources]/models/faster-whisper/base)
        """
        # 1. ユーザー設定確認
        user_model_path = self._get_user_model_path()
        if user_model_path and os.path.exists(user_model_path):
            self.model = WhisperModel(user_model_path, device="cpu")
            logger.info(f"Loaded model from user config: {user_model_path}")
            return

        # 2. HuggingFace Hubキャッシュ確認
        cache_path = self._get_hf_cache_path(self.model_size)
        if cache_path and os.path.exists(cache_path):
            self.model = WhisperModel(cache_path, device="cpu")
            logger.info(f"Loaded model from HuggingFace cache: {cache_path}")
            return

        # 3. HuggingFace Hubからダウンロード試行 (タイムアウト10秒)
        try:
            self.model = await asyncio.wait_for(
                self._download_from_hf_hub(self.model_size),
                timeout=10.0
            )
            logger.info(f"Downloaded model from HuggingFace Hub: {self.model_size}")
            return
        except (asyncio.TimeoutError, NetworkError, ProxyAuthError) as e:
            logger.warning(f"HuggingFace Hub download failed: {e}")

        # 4. バンドルbaseモデルにフォールバック
        bundled_path = self._get_bundled_model_path()
        if bundled_path and os.path.exists(bundled_path):
            self.model = WhisperModel(bundled_path, device="cpu")
            logger.info(f"Loaded bundled base model (offline fallback): {bundled_path}")
            return

        # 5. すべて失敗
        raise ModelLoadError("faster-whisperモデルが見つかりません。インストールを確認してください。")

    async def transcribe_partial(self, segment: SpeechSegment) -> PartialTranscription:
        """部分テキスト生成 (発話継続中、1秒間隔)"""
        segments, info = self.model.transcribe(
            segment.audio_data,
            language="ja",
            beam_size=1,  # 高速化のためビームサイズ削減
        )

        text = " ".join([s.text for s in segments])

        return PartialTranscription(
            segment_id=segment.segment_id,
            text=text,
            confidence=info.language_probability,
            timestamp=time.time(),
            is_partial=True,
        )

    async def transcribe_final(self, segment: SpeechSegment) -> FinalTranscription:
        """確定テキスト生成 (発話終了後)"""
        segments, info = self.model.transcribe(
            segment.audio_data,
            language="ja",
            beam_size=5,  # 精度重視のためビームサイズ拡大
            word_timestamps=True,
        )

        text = " ".join([s.text for s in segments])
        word_timestamps = [
            {"word": w.word, "start": w.start, "end": w.end}
            for s in segments for w in s.words
        ]

        return FinalTranscription(
            segment_id=segment.segment_id,
            text=text,
            confidence=info.language_probability,
            timestamp=time.time(),
            is_partial=False,
            word_timestamps=word_timestamps,
        )
```

**Transcription型定義**:

```python
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class PartialTranscription:
    segment_id: str
    text: str
    confidence: float
    timestamp: float
    is_partial: bool = True

@dataclass
class FinalTranscription:
    segment_id: str
    text: str
    confidence: float
    timestamp: float
    is_partial: bool = False
    word_timestamps: Optional[List[dict]] = None
```

**オフラインファースト実装詳細**:
- **モデル検出優先順位**: ユーザー設定 → HuggingFace Hubキャッシュ → インストーラーバンドルモデル (STT-REQ-002.1)
- **HuggingFace Hubタイムアウト**: 10秒 (STT-REQ-002.3)
- **オフラインフォールバック**: ダウンロード失敗時、バンドルbaseモデル使用 (STT-REQ-002.4)
- **プロキシ環境対応**: 環境変数 `HTTPS_PROXY` / `HTTP_PROXY` を認識 (STT-REQ-002.7)
- **オフラインモード強制**: ユーザー設定で HuggingFace Hub接続を完全スキップ (STT-REQ-002.6)

---

#### モデル配布戦略

**ハイブリッド戦略**: オンデマンドダウンロード + ローカルモデル優先

**初回起動時のモデル検出フロー**:

```
1. ユーザー設定パス (~/.config/meeting-minutes-automator/whisper_model_path)
   ↓ 存在しない
2. システム共有パス
   - macOS: /usr/local/share/faster-whisper/
   - Windows: C:\ProgramData\faster-whisper\
   - Linux: /usr/share/faster-whisper/
   ↓ 存在しない
3. HuggingFace Hubキャッシュ (~/.cache/huggingface/hub/models--Systran--faster-whisper-*)
   ↓ 存在しない
4. ユーザー選択肢提示:
   a. 「今すぐダウンロード (39MB)」→ HuggingFace Hub接続 (タイムアウト10秒)
   b. 「後でダウンロード」→ オフライン機能無効化、UI通知表示
   c. 「ローカルモデルを指定」→ ファイル選択ダイアログ
   ↓ ユーザー選択
5. バックグラウンドダウンロード (非ブロッキング)
   - ダウンロード進捗UI表示 (進捗バー + 残り時間)
   - 一時停止/再開機能
   - ダウンロード失敗時: 自動リトライ (3回、指数バックオフ)
```

**インストーラーサイズ制約**:
- **目標**: 50MB以下 (モデルバンドルなし)
- **Full版オプション**: 企業向けに baseモデル同梱版を提供 (89MB)

**システム共有パス活用**:
- 企業環境での事前配布: IT部門がシステムパスにモデルを配置
- 複数ユーザー間でのモデル共有: ディスク容量節約

**量子化モデル検討** (将来拡張):
- int8量子化: サイズ25%削減 (39MB → 10MB)
- 精度低下: 5%以内 (許容範囲)

**参照**: `.kiro/specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md`

---

**パフォーマンス目標** (STT-NFR-001.1):
- tiny/base: 0.2秒以内 (1秒の音声データに対して)
- small: 0.5秒以内
- medium: 1秒以内
- large: 2秒以内 (GPU使用時)

**エラーハンドリング**:
- モデルロード失敗: tinyモデルへのフォールバック試行 (STT-REQ-002.13, STT-NFR-002.1)
- 音声データ不正: エラー応答 `{"type": "error", "errorCode": "INVALID_AUDIO"}` (STT-REQ-002.14)

---

#### ResourceMonitor (Python)

**責任と境界**
- **主要責任**: システムリソース監視と動的モデル選択/ダウングレード
- **ドメイン境界**: リソース管理とモデルライフサイクル制御
- **データ所有**: リソース使用履歴、モデル切り替え履歴
- **トランザクション境界**: リソース監視サイクル (30秒間隔)

**依存関係**
- **インバウンド**: PythonSidecarManager (起動時)
- **アウトバウンド**: WhisperSTTEngine、WebSocketServer (UI通知)
- **外部依存**: psutil (システムリソース取得)

**契約定義**

```python
import psutil
from typing import Optional

class ResourceMonitor:
    def __init__(self, stt_engine: WhisperSTTEngine):
        self.stt_engine = stt_engine
        self.cpu_threshold = 85  # CPU使用率閾値 (%)
        self.memory_threshold = 4 * 1024 * 1024 * 1024  # メモリ閾値 (4GB)
        self.monitoring_interval = 30  # 監視間隔 (秒)

    async def detect_startup_model(self) -> ModelSize:
        """
        起動時のシステムリソース検出とモデル選択

        Returns:
            ModelSize: 選択されたモデルサイズ
        """
        cpu_count = psutil.cpu_count()
        memory_total = psutil.virtual_memory().total
        gpu_available = self._check_gpu_availability()

        if gpu_available:
            gpu_memory = self._get_gpu_memory()
            if memory_total >= 8 * 1024**3 and gpu_memory >= 10 * 1024**3:
                return "large-v3"  # 最高精度優先
            elif memory_total >= 4 * 1024**3 and gpu_memory >= 5 * 1024**3:
                return "medium"  # 精度とリソースのバランス

        if memory_total >= 4 * 1024**3:
            return "small"  # CPU推論の現実的な上限
        elif memory_total >= 2 * 1024**3:
            return "base"  # 低リソース環境対応
        else:
            return "tiny"  # 最低限動作保証

    async def request_model_downgrade(self, new_model: ModelSize) -> None:
        """
        音声セグメント境界でのモデル切り替えを要求

        Args:
            new_model: 切り替え先のモデルサイズ

        Note:
            STT-REQ-006.9準拠: 現在処理中の音声セグメントは既存モデルで完了し、
            次のセグメントから新モデルを適用（処理中断時間0秒）
        """
        # VADで現在処理中のセグメントがあるか確認
        if self.stt_engine.is_processing_segment():
            # 現在のセグメント処理完了を待機
            await self.stt_engine.wait_for_segment_completion()

        # 次のセグメントから新モデルを適用
        await self.stt_engine.switch_model(new_model)

        # UI通知
        self._notify_model_change(self.current_model, new_model)

        # 切り替え履歴をログに記録
        log.info(f"Model switched at segment boundary: {self.current_model} → {new_model}")
        self.current_model = new_model

    async def start_monitoring(self) -> None:
        """
        リソース監視開始 (30秒間隔)
        """
        while True:
            await asyncio.sleep(self.monitoring_interval)

            cpu_usage = psutil.cpu_percent(interval=1)
            memory_usage = psutil.virtual_memory().used

            # DEBUG: メモリ/CPU使用量をログ記録
            logger.debug(f"Resource usage: CPU={cpu_usage}%, Memory={memory_usage / 1024**3:.2f}GB")

            # CPU 85%を60秒以上持続 → ダウングレード
            if self._is_cpu_high_sustained(cpu_usage, duration_sec=60):
                await self._downgrade_model(reason="CPU負荷軽減")

            # メモリ4GB到達 → 即座にbaseモデル
            if memory_usage >= self.memory_threshold:
                await self._emergency_downgrade(reason="メモリ不足")

            # リソース回復 → アップグレード提案
            if self._is_resource_recovered(cpu_usage, memory_usage):
                await self._suggest_upgrade()

    async def _downgrade_model(self, reason: str) -> None:
        """
        1段階モデルダウングレード

        ダウングレード順序: large-v3 → medium → small → base → tiny
        """
        current_model = self.stt_engine.model_size
        downgrade_order = ["large-v3", "medium", "small", "base", "tiny"]

        current_index = downgrade_order.index(current_model)
        if current_index >= len(downgrade_order) - 1:
            # tinyモデルでもリソース不足 → 録音一時停止
            await self._pause_recording(reason="システムリソース不足")
            return

        new_model = downgrade_order[current_index + 1]

        # 進行中セグメント完了まで待機
        await self._wait_for_segment_completion()

        # モデル切り替え
        await self.stt_engine.set_model(new_model)

        # UI通知
        await self._notify_ui(f"{reason}のためモデルを{current_model}→{new_model}に変更しました")

        # ログ記録
        logger.info(f"Model downgraded: {current_model} → {new_model} (reason: {reason})")

    async def _emergency_downgrade(self, reason: str) -> None:
        """メモリ不足時の緊急ダウングレード (即座にbaseモデル)"""
        current_model = self.stt_engine.model_size

        if current_model == "base" or current_model == "tiny":
            return  # すでにbase/tinyモデル

        # 進行中セグメント完了まで待機
        await self._wait_for_segment_completion()

        # baseモデルに切り替え
        await self.stt_engine.set_model("base")

        # UI通知
        await self._notify_ui(f"{reason}のためbaseモデルに変更しました")

        # ログ記録
        logger.warning(f"Emergency downgrade: {current_model} → base (reason: {reason})")

    async def _suggest_upgrade(self) -> None:
        """リソース回復時のモデルアップグレード提案"""
        current_model = self.stt_engine.model_size
        upgrade_order = ["tiny", "base", "small", "medium", "large-v3"]

        current_index = upgrade_order.index(current_model)
        if current_index >= len(upgrade_order) - 1:
            return  # すでに最上位モデル

        suggested_model = upgrade_order[current_index + 1]

        # UI通知 (ユーザー承認待機)
        await self._notify_ui(
            f"リソースが回復しました。モデルをアップグレードしますか？ ({current_model} → {suggested_model})",
            action="upgrade_approval"
        )
```

**リソース監視ポリシー**:
- **監視間隔**: 30秒 (STT-NFR-001.6)
- **CPU閾値**: 85% を 60秒以上持続 (STT-REQ-006.7)
- **メモリ閾値**: 4GB 到達 (STT-REQ-006.8)
- **リソース回復条件**: メモリ2GB未満 AND CPU 50%未満が5分継続 (STT-REQ-006.10)

**モデル選択ルール** (STT-REQ-006.2):

| 条件 | 選択モデル | 理由 |
|------|-----------|------|
| GPU利用可能 AND システムメモリ≥8GB AND GPUメモリ≥10GB | large-v3 | 最高精度優先 |
| GPU利用可能 AND システムメモリ≥4GB AND GPUメモリ≥5GB | medium | 精度とリソースのバランス |
| CPU AND メモリ≥4GB | small | CPU推論の現実的な上限 |
| CPU AND メモリ≥2GB | base | 低リソース環境対応 |
| メモリ<2GB | tiny | 最低限動作保証 |

**手動モデル選択**: ユーザーが設定画面で手動モデル選択可能 (自動選択をオーバーライド) (STT-REQ-006.4)

**最終手段**: tinyモデルでもリソース不足が継続する場合、録音一時停止 (STT-REQ-006.11)

---

#### LocalStorageService (Rust)

**責任と境界**
- **主要責任**: 録音セッションのローカルストレージへの永続化
- **ドメイン境界**: データ永続化とセッション管理
- **データ所有**: セッションメタデータ、録音ファイル、文字起こし結果
- **トランザクション境界**: セッション単位

**依存関係**
- **インバウンド**: WhisperSTTEngine (IPC経由)、WebSocketServer
- **アウトバウンド**: ファイルシステム
- **外部依存**: tokio::fs (非同期ファイルI/O)

**契約定義**

```rust
use tokio::fs;
use serde::{Deserialize, Serialize};

pub struct LocalStorageService {
    app_data_dir: PathBuf,
}

impl LocalStorageService {
    pub async fn create_session(&self, session_id: &str) -> Result<()> {
        // セッションディレクトリ作成: [app_data_dir]/recordings/[session_id]/
        let session_dir = self.app_data_dir.join("recordings").join(session_id);
        fs::create_dir_all(&session_dir).await?;

        Ok(())
    }

    pub async fn save_audio(&self, session_id: &str, audio_data: &[u8]) -> Result<()> {
        // audio.wavファイルとして保存 (16kHz, モノラル, 16bit PCM)
        let audio_path = self.get_session_dir(session_id).join("audio.wav");

        // WAVヘッダー追加
        let wav_data = self.create_wav_file(audio_data, sample_rate: 16000, channels: 1)?;

        fs::write(&audio_path, wav_data).await?;

        Ok(())
    }

    pub async fn append_transcription(
        &self,
        session_id: &str,
        transcription: &Transcription,
    ) -> Result<()> {
        // transcription.jsonl に JSON Lines形式で追記
        let transcription_path = self.get_session_dir(session_id).join("transcription.jsonl");

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&transcription_path)
            .await?;

        let json_line = serde_json::to_string(transcription)? + "\n";
        file.write_all(json_line.as_bytes()).await?;

        Ok(())
    }

    pub async fn save_session_metadata(
        &self,
        session_id: &str,
        metadata: SessionMetadata,
    ) -> Result<()> {
        // session.json として保存
        let session_path = self.get_session_dir(session_id).join("session.json");
        let json = serde_json::to_string_pretty(&metadata)?;

        fs::write(&session_path, json).await?;

        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        // recordings/ ディレクトリ内の全セッションメタデータを読み込み
        let recordings_dir = self.app_data_dir.join("recordings");

        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&recordings_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let session_path = entry.path().join("session.json");
            if session_path.exists() {
                let json = fs::read_to_string(&session_path).await?;
                let metadata: SessionMetadata = serde_json::from_str(&json)?;
                sessions.push(metadata);
            }
        }

        // 日時降順でソート
        sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok(sessions)
    }

    pub async fn check_disk_space(&self) -> Result<DiskSpaceStatus> {
        // ディスク容量確認
        let available = self.get_available_disk_space()?;

        if available < 500 * 1024 * 1024 {  // 500MB未満
            Ok(DiskSpaceStatus::Critical)
        } else if available < 1024 * 1024 * 1024 {  // 1GB未満
            Ok(DiskSpaceStatus::Warning)
        } else {
            Ok(DiskSpaceStatus::Ok)
        }
    }
}
```

**SessionMetadata型定義**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: u64,
    pub audio_device: String,
    pub model_size: String,
    pub total_segments: usize,
    pub total_characters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiskSpaceStatus {
    Ok,
    Warning,  // 1GB未満
    Critical, // 500MB未満
}
```

**ディレクトリ構造**:

```
[app_data_dir]/recordings/[session_id]/
├── audio.wav               # 16kHz mono 16bit PCM
├── transcription.jsonl     # JSON Lines形式 (部分+確定テキスト)
└── session.json           # セッションメタデータ
```

**transcription.jsonl形式例**:

```jsonl
{"segment_id": "seg-001", "text": "こんにちは", "is_partial": false, "confidence": 0.95, "timestamp": 1696234567890}
{"segment_id": "seg-002", "text": "今日は会議です", "is_partial": false, "confidence": 0.92, "timestamp": 1696234570000}
```

**ディスク容量管理**:
- **警告閾値** (1GB未満): 警告ログ記録 + ユーザー通知 (STT-REQ-005.7)
- **制限閾値** (500MB未満): 新規録音開始を拒否 (STT-REQ-005.8)

**セキュリティ要件**:
- アプリケーション専用ディレクトリ (ユーザーのホームディレクトリ配下) にのみ書き込む (STT-NFR-004.3)

---

### 7.9 IPC Event Distribution System (ADR-013 Supersession)

**目的**: Sender/Receiver並行実行により、デッドロックを根本的に解決します。

**関連ADR**:
- ❌ ADR-008 (Rejected - 構造的デッドロック欠陥)
- ❌ ADR-009 (Rejected - Mutex共有問題 + blocking_send問題)
- ❌ ADR-011 (Superseded - IPC Stdin/Stdout Mutex Separation)
- ❌ ADR-012 (Superseded - Audio Callback Backpressure Redesign)
- ✅ ADR-013 (Sidecar Full-Duplex IPC Final Design)
- ✅ ADR-013 P0 Bug Fixes (Post-approval critical fixes)

#### 7.9.1 アーキテクチャ概要

**ADR-008/009の致命的欠陥**:
1. **構造的デッドロック（P0）**: 1フレーム送信→speech_end待ち→次フレーム送信の順序で、Whisperが複数フレームなしでspeech_endを出せず永久デッドロック（ADR-008）
2. **Mutex共有によるシリアライゼーション（P0）**: `Arc<Mutex<PythonSidecarManager>>`共有により、Sender/Receiver並行実行が実質シリアライズされ、問題1が解消されない（ADR-009）
3. **blocking_send()によるCPALストリーム停止（P0）**: Python異常時にオーディオコールバックが最大2秒ブロック → CPALのOSバッファ（128ms）オーバーラン → ストリーム停止（ADR-009）
4. **Python偽no_speech検出（P1）**: イベント発行の有無だけで判定するため、発話継続中でもイベント間にno_speechを誤送信（ADR-008/009共通）

**ADR-013の最終設計**: Stdin/Stdout分離 + try_send() Backpressure + Sidecar Facade  
（詳細は `.kiro/specs/meeting-minutes-stt/adrs/ADR-013-sidecar-full-duplex-final-design.md`、P0フォローアップは `ADR-013-P0-bug-fixes.md` を参照）
- **ADR-011**: stdin/stdoutを独立したMutexに分離 → 真の全二重通信実現（ADR-013に統合）
- **ADR-012**: blocking_send() → try_send() + UI Notification → CPAL保護（ADR-013に統合）
- **ADR-013**: Facade API化、LDJSONフレーミング、バッファポリシー明確化
- **Sender Task**: フレームを連続的にPythonへ送信（stdinのみロック）
- **Receiver Task**: Pythonからイベントを連続的に受信（stdoutのみロック）
- 両タスクは完全に独立、互いにブロックしない

```
Audio Callback (10ms)       Recording Session Task                    Python Sidecar
      |                           |                                            |
      |                    ┌──────┴────────┐                                  |
      |-- frame (try_send) │ Sender Task   │                                  |
      |   (mpsc 500 buf)   │ (Independent) │                                  |
      |                    │               │                                  |
      |                    │ loop {        │                                  |
      |                    │   frame = rx  │                                  |
      |                    │   stdin.send  │-- send (stdin mutex) ----------->|
      |                    │ }             │  (NO stdout mutex!)              |
      |                    └───────────────┘                                  |
      |                                                                        |
      |                    ┌───────────────┐                                  |
      |                    │ Receiver Task │                                  |
      |                    │ (Independent) │                                  |
      |                    │               │                                  |
      |                    │ loop {        │                                  |
      |                    │   event = rcv │<-- recv (stdout mutex) ----------|
      |                    │   broadcast   │  (NO stdin mutex!)               |
      |                    │ }             │-- events --------> WebSocket/UI  |
      |                    └───────────────┘                                  |
      |                                                                        |
      |                    ┌───────────────┐                                  |
      |                    │ UI Notify Task│                                  |
      |                    │               │                                  |
      |                    │ monitor drop  │-- stt_error ------> Frontend     |
      |                    └───────────────┘  (Python異常検出)                 |
```

**キーイノベーション（ADR-011/012）**:
- **真の全二重通信**: stdin/stdout独立Mutex → Sender/Receiverが互いにブロックしない
- **CPAL保護**: try_send() → コールバックは常に即座にreturn（blocking操作なし）
- **Python異常検出**: バッファ満杯（500フレーム = 5秒）時、UI通知で録音再起動を促す
- **Whisper要件満足**: Sender連続送信 → 必要なだけフレーム蓄積可能
- mpsc channel: 500フレーム（5秒）バッファ（ADR-012）
- broadcast channel: 1000イベントバッファ
- Mutexスコープ最小化: send時はstdinのみ、receive時はstdoutのみ

#### 7.9.2 Concurrent Sender/Receiver Implementation (ADR-011)

**起動タイミング**: `start_recording()` コマンド実行時

**ライフサイクル**:
1. 録音開始 → Sender/Receiver/UI Notify 3タスク起動
2. 音声コールバック → フレームをmpsc channelにpush（**try_send** - ADR-012）
3. **Sender Task**: frame_rxからフレーム受信 → Pythonへ送信（**stdinのみロック**）
4. **Receiver Task**: Pythonからイベント受信 → broadcast配信（**stdoutのみロック**）
5. **UI Notify Task**: ドロップ検出 → stt_error UI通知
6. 録音停止 → frame_rx close → Sender終了 → Receiver abort → メトリクスレポート

**PythonSidecarManager構造体（ADR-011）**:
```rust
pub struct PythonSidecarManager {
    /// Stdin for sending JSON messages (独立したMutex)
    stdin: Arc<tokio::Mutex<ChildStdin>>,

    /// Stdout for receiving JSON messages (独立したMutex)
    stdout: Arc<tokio::Mutex<BufReader<ChildStdout>>>,

    /// Child process handle (監視のみ)
    child_handle: Arc<tokio::Mutex<Child>>,
}

impl PythonSidecarManager {
    /// Send JSON message to Python (stdinのみロック)
    pub async fn send_message(&self, msg: &serde_json::Value) -> Result<(), IpcError> {
        let json_line = serde_json::to_string(msg)? + "\n";

        let mut stdin = self.stdin.lock().await;  // ← stdin専用Mutex
        stdin.write_all(json_line.as_bytes()).await?;
        stdin.flush().await?;
        // ← Mutex即座に解放

        Ok(())
    }

    /// Receive JSON message from Python (stdoutのみロック)
    pub async fn receive_message(&self) -> Result<serde_json::Value, IpcError> {
        let mut stdout = self.stdout.lock().await; // ← stdout専用Mutex
        let mut line = String::new();

        let n = stdout.read_line(&mut line).await?;
        if n == 0 {
            return Err(IpcError::ProcessExited);
        }

        let msg = serde_json::from_str(&line)?;
        // ← Mutex即座に解放

        Ok(msg)
    }
}
```

**実装例（ADR-011準拠）**:
```rust
fn spawn_recording_session_task(
    python_sidecar: Arc<PythonSidecarManager>,  // ← Arc<Mutex<T>>ではない！
    websocket_server: Arc<tokio::Mutex<WebSocketServer>>,
    mut frame_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<serde_json::Value>,
    frame_drop_detected: Arc<AtomicBool>,
) -> SessionHandle {
    let python_sender = Arc::clone(&python_sidecar);
    let python_receiver = Arc::clone(&python_sidecar);

    let metrics = Arc::new(SessionMetrics::new());
    let metrics_send = Arc::clone(&metrics);
    let metrics_recv = Arc::clone(&metrics);

    // Sender Task: Continuously send frames
    let sender_handle = tokio::spawn(async move {
        while let Some(audio_data) = frame_rx.recv().await {
            let request = serde_json::json!({
                "type": "audio_frame",
                "data": audio_data,
            });

            // Send with stdin mutex only (NO stdout mutex!)
            python_sender.send_message(&request).await.ok();

            metrics_send.frames_sent.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Receiver Task: Continuously receive events
    let receiver_handle = tokio::spawn(async move {
        loop {
            // Receive with stdout mutex only (NO stdin mutex!)
            let event_result = python_receiver.receive_message().await;

            match event_result {
                Ok(event) => {
                    // Broadcast to subscribers
                    let _ = event_tx.send(event.clone());

                    metrics_recv.events_received.fetch_add(1, Ordering::Relaxed);

                    // Forward to WebSocket (if final_text)
                    // NO BREAK on speech_end! Continue receiving
                }
                Err(e) => {
                    metrics_recv.ipc_errors.fetch_add(1, Ordering::Relaxed);
                    // Exponential backoff, terminate after 10 errors
                }
            }
        }
    });

    SessionHandle {
        sender_handle,
        receiver_handle,
        metrics,
    }
}

struct SessionHandle {
    sender_handle: tokio::task::JoinHandle<()>,
    receiver_handle: tokio::task::JoinHandle<()>,
    metrics: Arc<SessionMetrics>,
}
```

**重要な違い**:
- **ADR-008（Rejected）**: 1フレーム送信 → speech_end待ち → 次フレーム（デッドロック）
- **ADR-009（Rejected）**: `Arc<Mutex<PythonSidecarManager>>`共有 → Mutex競合でシリアライズ
- **ADR-011（Adopted）**: stdin/stdout独立Mutex → 真の並行実行

#### 7.9.3 Audio Callback Integration (ADR-012: try_send() + UI Notification)

**変更点**: blocking_send → try_send + Large Ring Buffer + UI Notification

**ADR-008（Rejected - Frameドロップ）**:
```rust
device.start_with_callback(move |audio_data| {
    // BAD: Drops frames unconditionally on full buffer
    if let Err(e) = frame_tx.try_send(audio_data) {
        eprintln!("[Audio Callback] Frame dropped: {:?}", e);  // 音声破損！
    }
});
```

**ADR-009（Rejected - blocking_send問題）**:
```rust
device.start_with_callback(move |audio_data| {
    // BAD: Blocks up to 2 seconds if buffer full (Python hang)
    // → CPAL OS buffer (128ms) overruns → stream停止!
    if let Err(e) = frame_tx.blocking_send(audio_data) {
        eprintln!("[Audio Callback] Failed to send frame: {:?}", e);
    }
});
```

**ADR-012（Adopted - try_send + UI Notification）**:
```rust
// Drop detection flag
let frame_drop_detected = Arc::new(AtomicBool::new(false));

// Audio Callback (CPAL real-time context)
let drop_flag = Arc::clone(&frame_drop_detected);
let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_frame = AudioFrame {
        data: data.to_vec(),
        timestamp: Instant::now(),
    };

    // Non-blocking try_send - returns immediately
    match frame_tx.try_send(audio_frame) {
        Ok(_) => { /* success */ },
        Err(mpsc::error::TrySendError::Full(_)) => {
            // Python異常検出 → UI通知フラグ
            drop_flag.store(true, Ordering::Relaxed);
            metrics.frames_dropped.fetch_add(1, Ordering::Relaxed);
            // ← ここでreturn（コールバックは即座に戻る）
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            // Channel閉じている（正常終了）
        }
    }
};

// UI Notification Task (別タスクで監視)
tokio::spawn({
    let drop_flag = Arc::clone(&frame_drop_detected);
    let app_handle = app_handle.clone();
    async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            if drop_flag.load(Ordering::Relaxed) {
                // UIにPython異常を通知
                app_handle.emit_all("stt_error", serde_json::json!({
                    "error": "Python STT process not responding",
                    "action": "Please restart recording",
                    "severity": "critical"
                })).ok();

                metrics.python_hangs_detected.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }
});
```

**Backpressure戦略（ADR-012）**:
- mpsc channel: **500フレーム（5秒）バッファ**（一時的負荷に耐性）
- バッファフル時: **フレームドロップ + UI通知** → ユーザーが録音再起動
- **CPAL保護**: コールバックは常に即座にreturn（blocking操作禁止）
- **Python異常検出**: バッファ満杯 = Python hang/crash → UI通知
- 品質保証: 正常動作時のフレームドロップ率 < 0.01%

**フロントエンドエラーハンドリング**:
```typescript
// src/lib/stores/sttStore.ts
listen<SttError>('stt_error', (event) => {
  const { error, action, severity } = event.payload;

  if (severity === 'critical') {
    // 録音を強制停止
    stopRecording();

    // ユーザーにエラー通知
    notifications.error(error, {
      description: action,
      duration: 10000,  // 10秒表示
      actions: [
        { label: 'Restart Recording', onClick: () => startRecording() }
      ]
    });
  }
});
```

#### 7.9.4 Python VAD-Based no_speech Detection (ADR-008/009共通問題 - 実装済み)

**問題（P1 Bug）**: イベント発行の有無だけで判定 → 発話中にno_speech誤送信

**解決策**: VAD状態ベース判定

**AudioPipeline新規メソッド**:
```python
# python-stt/stt_engine/audio_pipeline.py
def is_in_speech(self) -> bool:
    """VADが音声検出中かチェック"""
    return self.vad.silence_duration == 0 and self.vad.speech_active

def has_buffered_speech(self) -> bool:
    """STT処理待ちの音声バッファがあるかチェック"""
    return len(self._current_speech_buffer) > 0
```

**main.py修正**:
```python
# python-stt/main.py:420-436
if not speech_detected:
    # VAD状態確認（ADR-009要件）
    if not self.pipeline.is_in_speech() and not self.pipeline.has_buffered_speech():
        # VADが無音確認 → no_speech送信
        await self.ipc.send_message({'eventType': 'no_speech'})
    else:
        # 発話継続中だがイベント未発行 → no_speech送信しない
        # Rust Receiverは次イベント待機
        logger.debug("Speech in progress (VAD active, no event yet)")
```

**重要**: 発話継続中は`no_speech`を送らず、Receiver Taskが次イベントを待ち続ける。

#### 7.9.5 Error Handling & Graceful Shutdown (ADR-011)

**Exponential Backoff**: Receiver Taskでのみ実施（100ms * 2^n、max 3.2s、10回で終了）

**Graceful Shutdown**:
```rust
// stop_recording()
frame_tx.close();  // Sender Task終了シグナル
sender_handle.await;  // Sender完了待ち
receiver_handle.abort();  // Receiver強制終了
metrics.report();  // 最終レポート
```

**stdin/stdout独立エラーハンドリング（ADR-011）**:
- stdin書き込みエラー: Sender Taskのみ影響、Receiver継続
- stdout読み込みエラー: Receiver Taskのみ影響、Sender継続
- Child process終了: 両タスクとも検出、録音停止

#### 7.9.6 Performance & Testing (ADR-011/012)

**Performance**:
- Latency: Frame送信 <1ms, Event配信 <1ms, Audio callback <10μs
- Throughput: 95-100 frames/sec（正常動作時フレームドロップなし）
- Memory: mpsc 960KB（500フレーム）+ broadcast 500KB = ~1.5MB/session
- Mutex競合: stdin/stdout独立により、競合時間 <1ms

**Success Criteria** (ADR-011/012):
- ✅ 長時間発話（120秒）デッドロックなし（ADR-011）
- ✅ Mutex競合なし（Sender/Receiver並行実行確認）（ADR-011）
- ✅ CPAL保護（Audio callback <10μs、blocking操作なし）（ADR-012）
- ✅ Python異常検出（バッファ満杯時、100ms以内にUI通知）（ADR-012）
- ✅ 正常動作時のフレームdrop率 < 0.01%（ADR-012）
- ✅ 偽no_speech解消（VAD状態判定）（ADR-008/009共通問題 - 実装済み）
- ✅ 既存テスト合格（Rust 26 + Python 143）

**E2E Tests** (必須 - ADR-011/012対応):
```rust
// ADR-011: Concurrent Send/Receive Tests
#[tokio::test]
async fn test_concurrent_send_receive() {
    // 100フレーム送信中に50イベント受信
    // Mutex競合なく並行実行確認
}

#[tokio::test]
async fn test_long_utterance_no_deadlock() {
    // 1200フレーム（120秒）送信
    // Whisper >10s処理でもタイムアウトなし確認
}

#[tokio::test]
async fn test_stdin_error_independence() {
    // stdin書き込みエラー時もreceive継続確認
}

#[tokio::test]
async fn test_stdout_error_independence() {
    // stdout読み込みエラー時もsend継続確認
}

// ADR-012: Audio Callback Backpressure Tests
#[tokio::test]
async fn test_python_hang_detection() {
    // Python processを故意にsleep（10秒）
    // 500フレーム送信後、501フレーム目でFull
    // UI通知が100ms以内に発行されることを確認
}

#[tokio::test]
async fn test_normal_operation_no_drop() {
    // 10000フレーム送信（100秒相当）
    // フレームドロップ率 < 0.01%確認
}

#[tokio::test]
async fn test_temporary_load_no_drop() {
    // Python processに3秒処理遅延
    // 500フレーム（5秒バッファ）内でドロップなし確認
}

#[tokio::test]
async fn test_no_false_no_speech_during_utterance() {
    // 発話継続中のno_speech誤送信がないことを確認（Python側実装済み）
}
```

#### 7.9.7 Metrics and Observability (ADR-011/012)

**Session Metrics**:
```rust
struct SessionMetrics {
    frames_sent: AtomicU64,
    frames_dropped: AtomicU64,
    events_received: AtomicU64,
    parse_errors: AtomicU64,              // JSONパースエラー数
    ipc_errors: AtomicU64,                 // IPC受信エラー数
    mutex_contention_count: AtomicU64,     // Mutex競合回数（ADR-011）
    stdin_lock_duration_us: AtomicU64,     // stdin lock保持時間（ADR-011）
    stdout_lock_duration_us: AtomicU64,    // stdout lock保持時間（ADR-011）
    concurrent_operations_count: AtomicU64, // 並行send+receive回数（ADR-011）
    python_hangs_detected: AtomicU64,      // Python hang検出回数（ADR-012）
    callback_duration_us: AtomicU64,       // Audio callback処理時間（ADR-012）
    start_time: Instant,
}

impl SessionMetrics {
    fn report(&self) {
        let duration = self.start_time.elapsed();
        let drop_rate = (self.frames_dropped.load(Ordering::Relaxed) as f64)
            / (self.frames_sent.load(Ordering::Relaxed) as f64) * 100.0;

        eprintln!(
            "[Session Metrics] Duration: {:?}, Frames: sent={}, dropped={} ({:.2}%), \
             Events: received={}, Errors: parse={}, ipc={}, \
             Mutex: contention={}, stdin_lock={}μs, stdout_lock={}μs, concurrent_ops={}, \
             Python: hangs={}, callback={}μs",
            duration,
            self.frames_sent.load(Ordering::Relaxed),
            self.frames_dropped.load(Ordering::Relaxed),
            drop_rate,
            self.events_received.load(Ordering::Relaxed),
            self.parse_errors.load(Ordering::Relaxed),
            self.ipc_errors.load(Ordering::Relaxed),
            self.mutex_contention_count.load(Ordering::Relaxed),
            self.stdin_lock_duration_us.load(Ordering::Relaxed),
            self.stdout_lock_duration_us.load(Ordering::Relaxed),
            self.concurrent_operations_count.load(Ordering::Relaxed),
            self.python_hangs_detected.load(Ordering::Relaxed),
            self.callback_duration_us.load(Ordering::Relaxed),
        );
    }
}
```

**Alert Conditions** (ADR-011/012):
- 🚨 **mutex_contention_count > 100/秒**: Mutex設計を再検証（ADR-011想定では0）
- 🚨 **stdin_lock_duration_us > 10000** (10ms): 異常な長時間保持
- 🚨 **stdout_lock_duration_us > 50000** (50ms): 異常な読み込み遅延
- 🚨 **frames_dropped > 100**: Python異常（UI通知発行済み）
- 🚨 **python_hangs_detected > 1**: Python頻繁なhang（再起動推奨）
- 🚨 **callback_duration_us > 100**: Audio callback遅延異常（CPALストリーム停止リスク）

**Python Process Health Monitoring** (ADR-008 v1.1):
```rust
fn spawn_python_health_monitor(
    python_sidecar: Arc<tokio::Mutex<PythonSidecarManager>>,
    app: AppHandle,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut check_count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let is_alive = {
                let sidecar = python_sidecar.lock().await;
                sidecar.is_process_alive()  // Check via std::process::Child::try_wait()
            };

            if !is_alive {
                eprintln!("[Health Monitor] Python process died");

                // Notify UI
                let _ = app.emit("python-process-error", serde_json::json!({
                    "type": "process_died",
                    "message": "Python音声処理プロセスが異常終了しました",
                }));

                // Attempt restart (max 3 retries)
                check_count += 1;
                if check_count <= 3 {
                    let mut sidecar = python_sidecar.lock().await;
                    if sidecar.restart().await.is_ok() {
                        check_count = 0;  // Reset on success
                    }
                } else {
                    break;  // Give up after 3 failures
                }
            }
        }
    })
}
```

**必要な実装**:
- `PythonSidecarManager::is_process_alive() -> bool`: `self.child_handle.try_wait()` で確認（ADR-011）
- `PythonSidecarManager::restart() -> Result<()>`: 既存プロセスをkillして新規起動

**Success Criteria** (ADR-011/012):
- フレームdrop率 < 0.01%（正常動作時）（ADR-012）
- JSONパースエラー率 < 1%
- デッドロック発生率 = 0%（ADR-011）
- Mutex競合発生率 = 0%（ADR-011）
- Audio callback遅延 < 10μs（ADR-012）
- Python hang検出時間 < 100ms（ADR-012）
- Pythonプロセス再起動成功率 > 90%

---
