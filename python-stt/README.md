# Meeting Minutes Automator - Python STT Sidecar

Pythonサイドカープロセスで音声認識（STT）処理を担当します。

## セットアップ

### 1. Python仮想環境の作成

```bash
cd python-stt
python3 -m venv .venv
```
> プロジェクト全体の設定（`.cargo/config.toml`など）が `.venv` を前提にしています。  
> `venv` など別名のディレクトリを作るとRustテストが失敗するので注意。

### 2. 仮想環境の有効化

**macOS/Linux:**
```bash
source .venv/bin/activate
```

**Windows:**
```cmd
.venv\Scripts\activate
```

### 3. 依存関係のインストール

**依存関係のインストール（順番通り実行）:**
```bash
pip install -r requirements.txt      # faster-whisper / webrtcvad / numpy / psutil など本番依存
pip install -r requirements-dev.txt  # pytest / pytest-asyncio など開発依存
pip install --no-build-isolation -e .  # stt_engine をパッケージとして登録
```
> 初回は faster-whisper が Hugging Face からモデルをダウンロードします（既定: `small`）。事前に `~/.cache/huggingface` を用意すると高速になります。

## テストの実行

```bash
# 仮想環境が有効化されていることを確認
.venv/bin/python -m pytest tests/ -v
```

**非同期テストや特定モジュールの検証:**
```bash
.venv/bin/python -m pytest tests/test_audio_integration.py -v --asyncio-mode=auto
```
> `test_audio_integration.py::test_audio_recording_to_transcription_full_flow` など一部テストは Whisper モデルを読み込みます。CPU/GPUリソース状況に応じて数分かかる場合があります。

## プロジェクト構造

```
python-stt/
├── main.py                     # AudioProcessor（VAD→Whisper→IPCイベント）
├── stt_engine/
│   ├── audio_pipeline.py
│   ├── ipc_handler.py          # stdin/stdout JSON IPC
│   ├── lifecycle_manager.py
│   ├── resource_monitor.py     # モデル自動ダウングレード/アップグレード
│   ├── transcription/
│   │   ├── voice_activity_detector.py
│   │   └── whisper_client.py
│   └── fake_processor.py       # MVP0互換用のレガシースタブ
├── tests/
│   ├── test_audio_integration.py
│   ├── test_audio_pipeline.py
│   ├── test_whisper_client.py
│   └── ...（計11ファイル、RED→GREENを担保）
├── requirements.txt            # 本番依存関係
├── requirements-dev.txt        # 開発依存関係
└── README.md                   # このファイル
```

## 開発ワークフロー

### MVP1 Real STT - 現在のフェーズ
- ✅ `AudioPipeline` + `VoiceActivityDetector` + `WhisperSTTEngine` によるリアルタイム推論（`main.py`, `stt_engine/audio_pipeline.py`）
- ✅ `ResourceMonitor` によるモデル自動ダウングレード・アップグレード提案（`stt_engine/resource_monitor.py`）
- ✅ IPCプロトコル v1.0（`process_audio_stream` / partial_text / final_text / speech_end / model_change）
- ✅ pytestベースの統合テスト（`tests/test_audio_integration.py`, `tests/test_whisper_client.py` など）と Rust 側統合テストの連携
- 🔄 Rust `AudioDeviceAdapter` との接続テストは進行中（現状はテストフィクスチャから音声フレームを供給）

### 環境依存バグ防止
- **必ず仮想環境を使用すること**
- システムPythonへの直接インストールは避ける
- チーム開発では全員が同じ依存関係バージョンを使用

## トラブルシューティング

### ModuleNotFoundError
仮想環境が有効化されているか確認:
```bash
which python3  # macOS/Linux
where python   # Windows
```

`.venv/bin/python3` や `.venv\Scripts\python.exe` が表示されればOK

**AI Coding Agents**: 仮想環境なしで実行する場合:
```bash
.venv/bin/python -m pytest tests/ -v
```

### pytest が見つからない
```bash
pip install -r requirements-dev.txt
```

## 今後の拡張予定

- **AudioDeviceAdapter統合**: Rust側で実装済みのマルチプラットフォーム録音アダプターと接続し、FakeAudioDeviceを置き換える（MVP1完了条件）
- **構造化ログ/メトリクス統合**: `structlog` ベースのJSONログとパフォーマンスメトリクスを Rust 側 `logger` と揃える（STT-NFR-LOG系要件）
- **セッション永続化フロー**: transcriptionイベントをローカルストレージ書き出し／Docs同期（MVP2）へ連携するためのAPI整備
