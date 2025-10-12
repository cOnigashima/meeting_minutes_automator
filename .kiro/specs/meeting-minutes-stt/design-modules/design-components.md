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

