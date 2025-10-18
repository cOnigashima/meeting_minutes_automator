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

**開発環境（テスト含む）:**
```bash
pip install -r requirements-dev.txt
```

**本番環境のみ:**
```bash
pip install -r requirements.txt
```

## テストの実行

```bash
# 仮想環境が有効化されていることを確認
pytest tests/ -v
```

**非同期テストも実行:**
```bash
pytest tests/ -v --asyncio-mode=auto
```

## プロジェクト構造

```
python-stt/
├── main.py                     # エントリーポイント
├── stt_engine/                 # STTエンジンモジュール
│   ├── __init__.py
│   ├── ipc_handler.py         # Rust IPC通信
│   ├── fake_processor.py      # Fakeプロセッサ（MVP0）
│   └── lifecycle_manager.py   # ライフサイクル管理
├── tests/                      # テスト
│   └── test_integration.py
├── requirements.txt            # 本番依存関係
├── requirements-dev.txt        # 開発依存関係
└── README.md                   # このファイル
```

## 開発ワークフロー

### Walking Skeleton (MVP0) - 現在のフェーズ
- ✅ スケルトン実装完了
- ✅ TDD Red状態確立（全テストが NotImplementedError で失敗）
- ⏭️ 次: Task 2 で FakeAudioDevice 実装

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

- **Task 2 (MVP1)**: faster-whisper統合
- **Task 2 (MVP1)**: webrtcvad統合
- **Task 3 (MVP2)**: リソースベースモデル選択
