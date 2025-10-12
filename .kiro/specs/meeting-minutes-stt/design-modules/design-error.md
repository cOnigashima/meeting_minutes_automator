## Error Handling

### Error Strategy

システム全体で統一されたエラー処理パターンを採用し、各レイヤーでの適切な回復メカニズムを実装します。

**エラー分類と対応戦略**:
- **回復可能エラー**: 自動リトライと代替手段の提供 (IPC通信エラー、モデルロード失敗等)
- **設定エラー**: ユーザーガイダンスと修正支援 (デバイス選択エラー、許可拒否等)
- **致命的エラー**: 安全な状態への移行とデータ保護 (ディスク容量不足、バンドルモデル欠落等)

### Error Categories and Responses

#### AudioDeviceError (音声デバイスエラー)

**エラー種別**:
- `DeviceNotFound`: デバイス一覧更新と代替デバイス提案
- `PermissionDenied`: システム設定ガイダンスとアクセス権限の説明
- `DeviceBusy`: 他アプリケーション終了案内と排他制御
- `InvalidConfiguration`: 設定検証とデフォルト値への復帰
- `DeviceDisconnected`: 自動再接続試行 (5秒間隔、最大3回)

**実装例**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AudioDeviceError {
    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("Microphone access permission denied")]
    PermissionDenied,

    #[error("Audio device is busy (used by another application)")]
    DeviceBusy,

    #[error("Audio device disconnected: {0}")]
    DeviceDisconnected(String),
}

impl AudioDeviceError {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AudioDeviceError::DeviceBusy | AudioDeviceError::DeviceDisconnected(_)
        )
    }

    pub fn user_message(&self) -> String {
        match self {
            AudioDeviceError::DeviceNotFound(device) => {
                format!("音声デバイス「{}」が見つかりません。デバイス一覧を更新してください。", device)
            }
            AudioDeviceError::PermissionDenied => {
                "マイクアクセスが拒否されました。システム設定から許可してください。".to_string()
            }
            AudioDeviceError::DeviceBusy => {
                "音声デバイスが他のアプリケーションに使用されています。他のアプリを終了してください。".to_string()
            }
            AudioDeviceError::DeviceDisconnected(device) => {
                format!("音声デバイス「{}」が切断されました。再接続を試行しています...", device)
            }
        }
    }
}
```

**エラー処理フロー**:
1. **デバイス切断検出**: `RealAudioDevice` がデバイス切断イベントを検出 (STT-REQ-004.9)
2. **エラーログ記録**: ERRORレベルでログ記録 (STT-NFR-005.2)
3. **ユーザー通知**: 「音声デバイスが切断されました」通知を表示 (STT-REQ-004.10)
4. **録音停止**: 音声ストリームを安全に停止 (5秒以内) (STT-REQ-004.10, STT-NFR-002.2)
5. **自動再接続**: 5秒間隔で最大3回まで再試行 (STT-REQ-004.11)

---

#### SttProcessingError (音声処理エラー)

**エラー種別**:
- `ModelLoadFailed`: 代替モデルの自動選択 (large → medium → ... → tiny)
- `TranscriptionTimeout`: 部分結果の保存と継続処理
- `InsufficientResources`: 品質レベル調整と負荷軽減 (動的ダウングレード)
- `InvalidAudioData`: エラー応答 `{"type": "error", "errorCode": "INVALID_AUDIO"}`

**実装例**:

```python
class SttProcessingError(Exception):
    """STT処理エラー基底クラス"""
    pass

class ModelLoadError(SttProcessingError):
    """faster-whisperモデルロード失敗"""
    def __init__(self, model_size: str, reason: str):
        self.model_size = model_size
        self.reason = reason
        super().__init__(f"Failed to load {model_size} model: {reason}")

class InvalidAudioDataError(SttProcessingError):
    """音声データ不正エラー"""
    def __init__(self, reason: str):
        self.reason = reason
        super().__init__(f"Invalid audio data: {reason}")
```

**エラー処理フロー**:
1. **モデルロード失敗**: tinyモデルへのフォールバック試行 (STT-REQ-002.13, STT-NFR-002.1)
2. **音声データ不正**: エラー応答 `{"type": "error", "errorCode": "INVALID_AUDIO"}` を返す (STT-REQ-002.14)
3. **リソース不足**: ResourceMonitorが動的ダウングレードを実行 (STT-REQ-006.7, STT-REQ-006.8)

---

#### NetworkError (ネットワークエラー)

**エラー種別**:
- `HuggingFaceHubTimeout`: バンドルbaseモデルにフォールバック (STT-REQ-002.4)
- `ProxyAuthError`: バンドルbaseモデルにフォールバック (STT-REQ-002.4)
- `OfflineMode`: HuggingFace Hub接続をスキップし、ローカルモデルのみ使用 (STT-REQ-002.6)

**実装例**:

```python
class NetworkError(Exception):
    """ネットワーク関連エラー基底クラス"""
    pass

class HuggingFaceHubTimeoutError(NetworkError):
    """HuggingFace Hubタイムアウト"""
    def __init__(self, timeout_sec: int):
        self.timeout_sec = timeout_sec
        super().__init__(f"HuggingFace Hub download timeout ({timeout_sec}s)")

class ProxyAuthError(NetworkError):
    """プロキシ認証エラー"""
    def __init__(self, proxy_url: str):
        self.proxy_url = proxy_url
        super().__init__(f"Proxy authentication failed: {proxy_url}")
```

**エラー処理フロー**:
1. **HuggingFace Hubタイムアウト** (10秒): バンドルbaseモデルにフォールバック (STT-REQ-002.4)
2. **プロキシ認証エラー**: バンドルbaseモデルにフォールバック (STT-REQ-002.4)
3. **オフラインモード強制**: ユーザー設定でHuggingFace Hub接続を完全スキップ (STT-REQ-002.6)
4. **ログ記録**: 「オフラインモードで起動: バンドルbaseモデル使用」をINFOレベルで記録 (STT-REQ-002.4)

---

#### StorageError (ストレージエラー)

**エラー種別**:
- `DiskSpaceWarning`: 警告ログ記録 + ユーザー通知 (1GB未満)
- `DiskSpaceCritical`: 新規録音開始を拒否 (500MB未満)
- `SessionSaveError`: データ保存失敗時のリトライと一時保存

**実装例**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Disk space warning: {available_mb}MB remaining")]
    DiskSpaceWarning { available_mb: u64 },

    #[error("Disk space critical: {available_mb}MB remaining (minimum 500MB required)")]
    DiskSpaceCritical { available_mb: u64 },

    #[error("Failed to save session: {0}")]
    SessionSaveError(String),
}

impl StorageError {
    pub fn user_message(&self) -> String {
        match self {
            StorageError::DiskSpaceWarning { available_mb } => {
                format!("ディスク容量が不足しています (残り{}MB)。不要なファイルを削除してください。", available_mb)
            }
            StorageError::DiskSpaceCritical { available_mb } => {
                format!("ディスク容量が不足しているため録音できません (残り{}MB、最低500MB必要)。", available_mb)
            }
            StorageError::SessionSaveError(reason) => {
                format!("セッションの保存に失敗しました: {}", reason)
            }
        }
    }
}
```

**エラー処理フロー**:
1. **警告閾値** (1GB未満): 警告ログ記録 + ユーザー通知「ディスク容量が不足しています」(STT-REQ-005.7)
2. **制限閾値** (500MB未満): 新規録音開始を拒否 + エラーメッセージ「ディスク容量が不足しているため録音できません」(STT-REQ-005.8)
3. **録音中のディスク容量不足**: 録音を安全に停止し、既存データの破損を防ぐ (STT-NFR-002.4)

---

#### IpcProtocolError (IPC通信エラー)

**エラー種別**:
- `IncompatibleProtocolVersion`: バージョン不一致
- `MessageParseError`: JSON解析失敗
- `MethodNotSupported`: 未サポートメソッド

**実装例**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum IpcProtocolError {
    #[error("Incompatible IPC protocol version: Rust={rust}, Python={python}")]
    IncompatibleProtocolVersion { rust: String, python: String },

    #[error("Failed to parse IPC message: {0}")]
    MessageParseError(String),

    #[error("Method not supported: {0}")]
    MethodNotSupported(String),
}

impl IpcProtocolError {
    pub fn user_message(&self) -> String {
        match self {
            IpcProtocolError::IncompatibleProtocolVersion { rust, python } => {
                format!(
                    "PythonサイドカーとRustアプリのバージョンが一致しません (Rust: {}, Python: {})。アプリを再インストールしてください。",
                    rust, python
                )
            }
            IpcProtocolError::MessageParseError(reason) => {
                format!("IPC通信エラー: メッセージ解析に失敗しました ({})", reason)
            }
            IpcProtocolError::MethodNotSupported(method) => {
                format!("サポートされていない操作です: {}", method)
            }
        }
    }
}
```

**エラー処理フロー**:
1. **バージョン不一致検出**: 起動時またはメッセージ受信時
2. **エラーログ記録**: ERRORレベルでログ記録
3. **ユーザー通知**: トースト通知「バージョン不一致、再インストールが必要」
4. **Graceful Shutdown**: Pythonサイドカーを停止し、録音機能を無効化

---

### Error Handling Flow

```mermaid
flowchart TD
    A[エラー発生] --> B[エラー分類]

    B --> C{回復可能?}
    C -->|Yes| D[自動回復試行]
    C -->|No| E[ユーザー通知]

    D --> F{回復成功?}
    F -->|Yes| G[処理継続]
    F -->|No| H[代替手段実行]

    H --> I{代替手段成功?}
    I -->|Yes| G
    I -->|No| E

    E --> J[エラーログ記録 (ERROR)]
    J --> K[ユーザーガイダンス表示]
    K --> L[安全な状態へ移行]
```

**主要ステップ**:
1. **エラー分類**: AudioDeviceError, SttProcessingError, NetworkError, StorageError, IpcProtocolError
2. **回復可能性判定**: `is_recoverable()` メソッドで判定
3. **自動回復試行**: リトライロジック (最大3回、指数バックオフ)
4. **代替手段実行**: フォールバック処理 (バンドルモデル、tinyモデル等)
5. **ユーザー通知**: トースト通知 + エラーメッセージ表示
6. **エラーログ記録**: 構造化JSON形式でERRORレベル記録 (STT-NFR-005.2)

---

