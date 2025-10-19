# Meeting Minutes Automator ユーザーガイド

## 目次

1. [はじめに](#はじめに)
2. [システム要件](#システム要件)
3. [インストール](#インストール)
4. [初回セットアップ](#初回セットアップ)
5. [音声デバイス設定](#音声デバイス設定)
6. [faster-whisperモデル設定](#faster-whisperモデル設定)
7. [基本的な使い方](#基本的な使い方)
8. [トラブルシューティング](#トラブルシューティング)
9. [よくある質問（FAQ）](#よくある質問faq)

---

## はじめに

Meeting Minutes Automatorは、Google Meetなどのオンライン会議音声をリアルタイムで文字起こしし、議事録を自動生成するデスクトップアプリケーションです。

### 主な機能（MVP1 Core Implementation）

- ✅ **リアルタイム音声文字起こし**: faster-whisperによる高精度STT
- ✅ **音声活動検出（VAD）**: webrtcvadで発話区間を自動検出
- ✅ **オフライン動作**: ネットワーク不要（bundled baseモデル使用時）
- ✅ **リソース自動最適化**: CPU/メモリ使用率に応じたモデル自動切替
- ✅ **プライバシー保護**: すべての処理がローカル環境で完結

---

## システム要件

### 最小要件

- **OS**: macOS 12.0以降 / Windows 10 22H2以降 / Ubuntu 22.04以降
- **CPU**: Intel Core i5以上（x86_64）
- **メモリ**: 4GB RAM（8GB推奨）
- **ディスク空き容量**: 2GB以上（faster-whisperモデルキャッシュ用）
- **Python**: 3.9以降（自動的にバンドル版を使用）

### 推奨要件

- **メモリ**: 8GB RAM以上（large-v3モデル使用時）
- **GPU**: CUDA対応GPU（NVIDIA）またはMPS対応（Apple Silicon）- より高速な文字起こし
- **ネットワーク**: インターネット接続（初回モデルダウンロード用、オフライン動作も可能）

---

## インストール

### macOS

1. **dmgファイルをダウンロード**:
   ```bash
   # GitHub Releasesから最新版をダウンロード
   # （リリース準備中）
   ```

2. **インストール**:
   - dmgファイルをダブルクリック
   - `Meeting Minutes Automator.app`を`アプリケーション`フォルダにドラッグ

3. **初回起動時の権限許可**:
   - `システム設定` → `プライバシーとセキュリティ`
   - 「開発元を確認できないため...」→ `このまま開く`

### Windows

1. **msixファイルをダウンロード**:
   ```bash
   # GitHub Releasesから最新版をダウンロード
   # （リリース準備中）
   ```

2. **インストール**:
   - msixファイルをダブルクリック
   - セキュリティ警告が表示された場合は `詳細情報` → `実行`

### Linux

```bash
# debパッケージ（Ubuntu/Debian）
sudo dpkg -i meeting-minutes-automator_*.deb

# AppImage（ディストリビューション非依存）
chmod +x meeting-minutes-automator_*.AppImage
./meeting-minutes-automator_*.AppImage
```

---

## 初回セットアップ

### 1. アプリケーション起動

アプリを起動すると、自動的にPythonサイドカープロセスが起動します。

**初回起動時のログ出力例**:
```
[INFO] Python sidecar starting...
[INFO] faster-whisper model detection...
[INFO] HuggingFace Hub: Downloading Systran/faster-whisper-base...
[INFO] Model loaded: base (CPU mode)
[INFO] Ready for audio input
```

### 2. faster-whisperモデルの初回ダウンロード

初回起動時、以下のいずれかが実行されます:

1. **オンライン環境**: HuggingFace Hubから最適なモデルを自動ダウンロード（10秒タイムアウト）
2. **オフライン環境**: バンドルされたbaseモデルを使用

**モデルダウンロード進捗**:
- ダウンロード中は画面右下に通知が表示されます
- 初回ダウンロード時間: 約1〜5分（モデルサイズとネットワーク速度に依存）

---

## 音声デバイス設定

### macOS: BlackHoleのインストール（システム音声録音用）

Google Meetのシステム音声を録音するには、仮想オーディオデバイス「BlackHole」が必要です。

#### BlackHoleインストール手順

1. **Homebrewでインストール**:
   ```bash
   brew install blackhole-2ch
   ```

2. **Audio MIDI設定**:
   - `アプリケーション` → `ユーティリティ` → `Audio MIDI設定`を起動
   - 左下の `+` → `機器セットを作成`
   - 機器セット名: `BlackHole + Built-in`
   - 以下の2つのデバイスを選択:
     - ✅ `BlackHole 2ch`
     - ✅ `内蔵出力`（または使用中のスピーカー）
   - マスターデバイス: `内蔵出力`

3. **システム音声出力をBlackHoleに設定**:
   - `システム設定` → `サウンド` → `出力`
   - `BlackHole + Built-in`を選択

4. **Meeting Minutes Automatorで録音デバイスを選択**:
   - アプリ内 `設定` → `音声デバイス`
   - `BlackHole 2ch`を選択

#### 注意事項

- BlackHoleを使用中は、スピーカーから音が出ます（機器セット設定により）
- 録音停止後は、システム音声出力を元に戻してください

### Windows: WASAPIループバック

Windows 10/11では、WASAPIループバック機能によりシステム音声を直接録音できます。

#### 設定手順

1. **Meeting Minutes Automatorを起動**
2. `設定` → `音声デバイス`
3. デバイス一覧から `スピーカー (Loopback)` または `ヘッドフォン (Loopback)` を選択

**確認方法**:
- デバイス名に`(Loopback)`が含まれていることを確認
- 通常のマイクデバイスは`マイク (Realtek ...)`のように表示されます

### Linux: PulseAudioモニター

#### PulseAudio設定（Ubuntu 22.04/24.04）

```bash
# モニターデバイスの確認
pactl list sources | grep -E "Name:|Description:"

# 出力例:
#   Name: alsa_output.pci-0000_00_1f.3.analog-stereo.monitor
#   Description: Built-in Audio Analog Stereo Monitor
```

#### Meeting Minutes Automatorでの選択

1. `設定` → `音声デバイス`
2. `Built-in Audio Analog Stereo Monitor`を選択

---

## faster-whisperモデル設定

### モデル選択の基本

Meeting Minutes Automatorは、システムリソースに応じて自動的に最適なモデルを選択します。

| モデルサイズ | 精度 | メモリ使用量 | 処理速度 | 推奨環境 |
|------------|------|------------|---------|---------|
| **tiny** | ⭐⭐ | 500MB | 最速 | メモリ2GB未満 |
| **base** | ⭐⭐⭐ | 1GB | 高速 | メモリ2〜4GB |
| **small** | ⭐⭐⭐⭐ | 2GB | 標準 | メモリ4GB以上（CPU） |
| **medium** | ⭐⭐⭐⭐⭐ | 5GB | やや遅い | メモリ4GB以上（GPU） |
| **large-v3** | ⭐⭐⭐⭐⭐⭐ | 10GB | 遅い | メモリ8GB以上（GPU） |

### 自動モデル選択ルール

起動時に以下のルールで自動選択されます:

```
IF GPU利用可能 AND メモリ8GB以上:
  → large-v3モデル

ELSE IF GPU利用可能 AND メモリ4GB以上:
  → mediumモデル

ELSE IF CPU AND メモリ4GB以上:
  → smallモデル

ELSE IF メモリ2GB以上:
  → baseモデル

ELSE:
  → tinyモデル
```

### 手動モデル変更

設定ファイルで手動指定も可能です:

```bash
# macOS/Linux
nano ~/.config/meeting-minutes-automator/config.json

# Windows
notepad %APPDATA%\meeting-minutes-automator\config.json
```

**config.json**:
```json
{
  "whisper_model_size": "small",
  "offline_mode": false
}
```

### オフラインモード設定

企業ネットワークやインターネット接続なしで使用する場合:

```json
{
  "offline_mode": true
}
```

この設定により:
- HuggingFace Hub接続を完全にスキップ
- ローカルキャッシュまたはバンドルモデルのみ使用
- プロキシ認証エラーを回避

### モデルキャッシュ場所

ダウンロードされたモデルは以下に保存されます:

```bash
# macOS/Linux
~/.cache/huggingface/hub/models--Systran--faster-whisper-<size>/

# Windows
%USERPROFILE%\.cache\huggingface\hub\models--Systran--faster-whisper-<size>\
```

### モデルの事前ダウンロード

オフライン環境で使用する前に、オンライン環境でモデルを事前ダウンロード:

```bash
# Python仮想環境を有効化
cd python-stt
source .venv/bin/activate  # macOS/Linux
# .venv\Scripts\activate    # Windows

# モデルダウンロードスクリプト実行
python -c "from faster_whisper import WhisperModel; WhisperModel('small', device='cpu')"
```

---

## 基本的な使い方

### 1. 音声デバイスを選択

1. アプリ左下の `設定`アイコンをクリック
2. `音声デバイス`タブ
3. ドロップダウンから録音デバイスを選択
   - **マイク録音**: `内蔵マイク`または外部マイク
   - **システム音声**: `BlackHole 2ch`（macOS）/ `スピーカー (Loopback)`（Windows）

### 2. 録音開始

1. Google Meetに参加
2. Meeting Minutes Automatorで`録音開始`ボタンをクリック
3. リアルタイム文字起こしが画面に表示されます

**表示される情報**:
- 🟢 **部分テキスト**（グレー）: 発話中の暫定結果（<0.5s応答）
- ✅ **確定テキスト**（黒）: 無音検出後の最終結果（<2s応答）
- ⏱️ **タイムスタンプ**: 各発話の開始時刻

### 3. 録音停止

1. `録音停止`ボタンをクリック
2. 文字起こし結果が自動的にローカル保存されます

**保存場所**:
```bash
# macOS/Linux
~/Documents/MeetingMinutes/YYYY-MM-DD_HH-MM-SS.txt

# Windows
%USERPROFILE%\Documents\MeetingMinutes\YYYY-MM-DD_HH-MM-SS.txt
```

---

## トラブルシューティング

### 問題1: 「faster-whisperモデルが見つかりません」エラー

**症状**:
```
[ERROR] faster-whisperモデルが見つかりません。インストールを確認してください
```

**原因**:
- HuggingFace Hub接続失敗（ネットワークエラー、プロキシ認証）
- バンドルモデルが破損または欠落

**解決方法**:

1. **インターネット接続確認**:
   ```bash
   # HuggingFace Hub疎通確認
   curl -I https://huggingface.co
   ```

2. **プロキシ設定**（企業ネットワーク環境）:
   ```bash
   # macOS/Linux
   export HTTPS_PROXY=http://proxy.example.com:8080
   export HTTP_PROXY=http://proxy.example.com:8080

   # Windows（PowerShell）
   $env:HTTPS_PROXY="http://proxy.example.com:8080"
   $env:HTTP_PROXY="http://proxy.example.com:8080"
   ```

3. **手動モデルダウンロード**:
   ```bash
   cd python-stt
   source .venv/bin/activate
   pip install faster-whisper
   python -c "from faster_whisper import WhisperModel; WhisperModel('base', device='cpu')"
   ```

4. **オフラインモード強制**:
   - `config.json`で`"offline_mode": true`を設定
   - バンドルbaseモデルを使用

### 問題2: 「音声デバイスの初期化に失敗しました」エラー

**症状**:
```
[ERROR] 音声デバイスの初期化に失敗しました: BlackHole 2ch
```

**原因**:
- デバイスが他のアプリケーションで使用中
- デバイスドライバーの問題
- デバイスが物理的に接続されていない

**解決方法**:

1. **デバイス使用状況確認**:
   ```bash
   # macOS
   lsof | grep -i blackhole

   # Linux
   lsof /dev/snd/*
   ```

2. **他のアプリを終了**:
   - Zoom、OBS Studio、Audacityなど音声録音アプリを終了

3. **デバイス再選択**:
   - Meeting Minutes Automatorの設定で別のデバイスを試す
   - `内蔵マイク`で動作確認

4. **デバイスドライバー再インストール**（macOS BlackHole）:
   ```bash
   brew uninstall blackhole-2ch
   brew install blackhole-2ch
   # システム再起動
   ```

### 問題3: 文字起こし精度が低い

**症状**:
- 誤変換が多い
- 発話を認識しない
- 無音区間で途切れる

**原因**:
- モデルサイズが小さい（tiny/base）
- 音声品質が低い（ノイズ、エコー）
- 非英語話者の英語発話

**解決方法**:

1. **モデルサイズを上げる**:
   ```json
   // config.json
   {
     "whisper_model_size": "medium"  // small → medium
   }
   ```

2. **音声品質向上**:
   - マイクを口元に近づける
   - ノイズキャンセリング機能付きマイク使用
   - 静かな環境で録音

3. **VAD感度調整**（今後のバージョンで実装予定）:
   ```json
   // config.json (MVP2以降)
   {
     "vad_aggressiveness": 2  // 0-3, 高いほど無音検出が厳しい
   }
   ```

### 問題4: CPU使用率が高い

**症状**:
```
[WARNING] CPU使用率が高いため、モデルをsmall→baseへダウングレードしました
```

**原因**:
- モデルサイズが大きすぎる（medium/large-v3）
- GPU未使用でCPU処理

**解決方法**:

1. **自動ダウングレードを受け入れる**:
   - アプリは自動的に最適なモデルを選択します
   - 60秒間CPU使用率85%超過で自動ダウングレード

2. **手動でモデルサイズを下げる**:
   ```json
   // config.json
   {
     "whisper_model_size": "base"
   }
   ```

3. **GPU使用を有効化**（NVIDIA GPU搭載機のみ）:
   ```bash
   # CUDA Toolkit 11.8インストール確認
   nvidia-smi

   # faster-whisperをGPU版で再インストール
   cd python-stt
   pip uninstall faster-whisper
   pip install faster-whisper[gpu]
   ```

### 問題5: メモリ不足で録音が停止

**症状**:
```
[ERROR] メモリ不足のため録音を一時停止しました。(使用量: 3.95GB/4GB)
```

**原因**:
- メモリ使用率95%超過
- 他のアプリケーションがメモリを大量消費

**解決方法**:

1. **他のアプリを終了**:
   - ブラウザタブを閉じる（特にChrome）
   - 使用していないアプリを終了

2. **モデルサイズを下げる**:
   ```json
   // config.json
   {
     "whisper_model_size": "tiny"  // 最小メモリ使用量
   }
   ```

3. **システムメモリ増設**（ハードウェア対応）:
   - 推奨: 8GB RAM以上

### 問題6: Pythonサイドカープロセスが起動しない

**症状**:
```
[ERROR] Python sidecar failed to start
```

**原因**:
- Python環境が破損
- 必要な依存パッケージ未インストール

**解決方法**:

1. **Python環境確認**:
   ```bash
   # macOS/Linux
   which python3
   python3 --version  # 3.9以降

   # Windows
   where python
   python --version
   ```

2. **依存パッケージ再インストール**:
   ```bash
   cd python-stt
   python3 -m venv .venv
   source .venv/bin/activate  # macOS/Linux
   # .venv\Scripts\activate    # Windows

   pip install -r requirements.txt
   ```

3. **ログ確認**:
   ```bash
   # macOS/Linux
   tail -f ~/Library/Logs/meeting-minutes-automator/python-sidecar.log

   # Windows
   type %APPDATA%\meeting-minutes-automator\logs\python-sidecar.log
   ```

---

## よくある質問（FAQ）

### Q1: オフライン環境で使用できますか？

**A**: はい、可能です。

- **初回起動時**: インターネット接続が必要（モデルダウンロード）
- **2回目以降**: 完全オフラインで動作（キャッシュモデル使用）
- **完全オフライン**: `config.json`で`"offline_mode": true`を設定

### Q2: Google Meet以外の会議ツールでも使えますか？

**A**: はい、システム音声を録音できるすべてのアプリケーションで使用可能です。

- Zoom
- Microsoft Teams
- Slack Huddles
- Discord
- その他のビデオ会議ツール

### Q3: どのモデルサイズを選べばよいですか？

**A**: 以下を参考にしてください。

- **日常会議**: `base`または`small`（精度と速度のバランス）
- **高精度重要**: `medium`または`large-v3`（メモリ8GB以上推奨）
- **低スペックPC**: `tiny`（精度は劣るが動作軽快）

### Q4: GPUは必須ですか？

**A**: いいえ、CPU環境でも動作します。

- **CPU**: small/baseモデルで実用的な速度
- **GPU**: medium/large-v3モデルで高速処理

### Q5: 文字起こし結果をエクスポートできますか？

**A**: MVP1では自動的にテキストファイルで保存されます。

- **保存形式**: プレーンテキスト（`.txt`）
- **MVP2以降**: Google Docs自動同期機能を追加予定

### Q6: セキュリティは大丈夫ですか？

**A**: すべての処理がローカル環境で完結します。

- ✅ 音声データはインターネットに送信されません
- ✅ faster-whisperモデルはローカルで推論
- ⚠️ **既知の問題**: セキュリティ修正5件（SEC-001〜005）がMVP2 Phase 0で対応予定
  - 詳細: `.kiro/specs/meeting-minutes-stt/security-test-report.md`

### Q7: 動作が重い場合はどうすればよいですか？

**A**: 以下を試してください。

1. モデルサイズを下げる（`medium` → `small` → `base`）
2. 他のアプリケーションを終了
3. 自動リソース最適化に任せる（60秒監視で自動ダウングレード）

### Q8: 複数言語の文字起こしはできますか？

**A**: faster-whisperは99言語に対応していますが、MVP1では自動言語検出のみです。

- **MVP1**: 自動言語検出（日本語/英語混在対応）
- **MVP2以降**: 手動言語選択機能を追加予定

---

## サポート

### フィードバック・バグ報告

GitHub Issuesでバグ報告やフィードバックをお願いします:
- **Repository**: （リリース時に追加）
- **Issue Template**: Bug Report / Feature Request

### ログファイル

問題報告時は以下のログファイルを添付してください:

```bash
# macOS
~/Library/Logs/meeting-minutes-automator/

# Windows
%APPDATA%\meeting-minutes-automator\logs\

# Linux
~/.local/share/meeting-minutes-automator/logs/
```

---

**ドキュメントバージョン**: 1.0.0（MVP1 Core Implementation）
**最終更新**: 2025-10-19
