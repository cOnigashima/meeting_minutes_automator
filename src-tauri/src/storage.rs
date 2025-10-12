/// LocalStorageService - ローカルストレージ管理
/// Related requirement: STT-REQ-005.1
///
/// 録音セッションのローカルストレージへの永続化を担当。
/// セッションID生成、ディレクトリ作成、ファイル保存を管理。
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone)]
pub struct LocalStorageService {
    app_data_dir: PathBuf,
}

/// セッションハンドル（RAII）
/// 録音セッションのライフサイクル管理と原子的操作を提供
/// Related requirement: STT-REQ-005.1
pub struct SessionHandle {
    pub session_id: String,
    pub session_dir: PathBuf,
    /// ディスク容量ステータス（UI通知用）
    /// Related requirement: STT-REQ-005.7
    pub disk_status: DiskSpaceStatus,
    service: LocalStorageService,
}

impl SessionHandle {
    /// 音声ライター取得
    pub fn audio_writer(&self) -> Result<AudioWriter> {
        self.service.create_audio_writer(&self.session_id)
    }

    /// 文字起こしライター取得
    pub fn transcript_writer(&self) -> Result<TranscriptWriter> {
        self.service.create_transcript_writer(&self.session_id)
    }

    /// セッションメタデータ保存
    pub fn save_metadata(&self, metadata: &SessionMetadata) -> Result<()> {
        self.service.save_session_metadata(metadata)
    }

    /// ディスク容量警告が必要かどうか
    /// Related requirement: STT-REQ-005.7
    pub fn needs_disk_warning(&self) -> bool {
        self.disk_status == DiskSpaceStatus::Warning
    }

    /// ディスク容量警告メッセージ取得
    /// Related requirement: STT-REQ-005.7
    pub fn disk_warning_message(&self) -> Option<String> {
        if self.disk_status == DiskSpaceStatus::Warning {
            Some("ディスク容量が1GB未満です。録音を続けると保存できなくなる可能性があります。".to_string())
        } else {
            None
        }
    }
}

impl LocalStorageService {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// セッション開始（原子的操作）
    /// ID生成 → ディスク容量チェック → ディレクトリ作成をまとめて実行
    /// Related requirement: STT-REQ-005.1, STT-REQ-005.7, STT-REQ-005.8
    ///
    /// # Returns
    /// - `Ok(SessionHandle)`: セッション開始成功（disk_statusフィールドでWarning確認）
    /// - `Err(...)`: ディスク容量不足（Critical時）またはI/Oエラー
    ///
    /// # UI通知の処理例
    /// ```ignore
    /// let handle = storage.begin_session()?;
    /// if handle.needs_disk_warning() {
    ///     // UI通知: handle.disk_warning_message()
    /// }
    /// ```
    pub fn begin_session(&self) -> Result<SessionHandle> {
        // 1. ディスク容量チェック
        let disk_status = self.check_disk_space()?;

        if disk_status == DiskSpaceStatus::Critical {
            anyhow::bail!("ディスク容量が不足しているため録音できません（残り500MB未満）");
        }

        // 2. セッションID生成
        let session_id = self.generate_session_id();

        // 3. セッションディレクトリ作成
        let session_dir = self.create_session(&session_id)?;

        Ok(SessionHandle {
            session_id,
            session_dir,
            disk_status, // UI通知用にステータスを含める
            service: self.clone(),
        })
    }

    /// セッションID生成（UUID v4）
    /// Related requirement: STT-REQ-005.1
    pub fn generate_session_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// セッションディレクトリ作成
    /// Path: [app_data_dir]/recordings/[session_id]/
    /// Related requirement: STT-REQ-005.1, STT-REQ-005.8
    ///
    /// **重要**: ディスク容量チェックを実施
    /// Critical時（500MB未満）は録音開始を拒否
    pub fn create_session(&self, session_id: &str) -> Result<PathBuf> {
        // ディスク容量チェック（P0対応）
        let disk_status = self.check_disk_space()?;
        if disk_status == DiskSpaceStatus::Critical {
            anyhow::bail!(
                "ディスク容量が不足しているため録音できません（残り500MB未満）: {}",
                self.app_data_dir.display()
            );
        }

        let session_dir = self.get_session_dir(session_id);
        std::fs::create_dir_all(&session_dir)?;
        Ok(session_dir)
    }

    /// セッションディレクトリパス取得
    pub fn get_session_dir(&self, session_id: &str) -> PathBuf {
        self.app_data_dir.join("recordings").join(session_id)
    }

    /// WAVファイルライター作成
    /// 16kHz, モノラル, 16bit PCM形式でストリーミング書き込み
    /// Related requirement: STT-REQ-005.2, STT-REQ-005.8
    ///
    /// **重要**: ディスク容量チェックを実施
    /// Critical時（500MB未満）は録音開始を拒否
    pub fn create_audio_writer(&self, session_id: &str) -> Result<AudioWriter> {
        // ディスク容量チェック（P0対応）
        let disk_status = self.check_disk_space()?;
        if disk_status == DiskSpaceStatus::Critical {
            anyhow::bail!(
                "ディスク容量が不足しているため録音できません（残り500MB未満）: {}",
                self.app_data_dir.display()
            );
        }

        let session_dir = self.get_session_dir(session_id);
        let audio_path = session_dir.join("audio.wav");
        AudioWriter::new(audio_path)
    }

    /// 文字起こし結果ライター作成
    /// JSON Lines形式で追記書き込み
    /// Related requirement: STT-REQ-005.3, STT-REQ-005.8
    ///
    /// **重要**: ディスク容量チェックを実施
    /// Critical時（500MB未満）は録音開始を拒否
    pub fn create_transcript_writer(&self, session_id: &str) -> Result<TranscriptWriter> {
        // ディスク容量チェック（P0対応）
        let disk_status = self.check_disk_space()?;
        if disk_status == DiskSpaceStatus::Critical {
            anyhow::bail!(
                "ディスク容量が不足しているため録音できません（残り500MB未満）: {}",
                self.app_data_dir.display()
            );
        }

        let session_dir = self.get_session_dir(session_id);
        let transcript_path = session_dir.join("transcription.jsonl");
        TranscriptWriter::new(transcript_path)
    }

    /// セッションメタデータ保存
    /// session.jsonファイルに保存
    /// Related requirement: STT-REQ-005.4
    pub fn save_session_metadata(&self, metadata: &SessionMetadata) -> Result<()> {
        let session_dir = self.get_session_dir(&metadata.session_id);
        let metadata_path = session_dir.join("session.json");

        let json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&metadata_path, json)?;

        Ok(())
    }

    /// セッション一覧取得
    /// recordings/ディレクトリ内の全セッションメタデータを読み込み、
    /// 日時降順でソートしたリストを返す
    /// Related requirement: STT-REQ-005.5
    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let recordings_dir = self.app_data_dir.join("recordings");

        // recordingsディレクトリが存在しない場合は空リストを返す
        if !recordings_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        // recordingsディレクトリ内の各セッションディレクトリを走査
        for entry in std::fs::read_dir(&recordings_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let metadata_path = path.join("session.json");
            if !metadata_path.exists() {
                continue;
            }

            // session.json読み込み
            let json = std::fs::read_to_string(&metadata_path)?;
            let metadata: SessionMetadata = serde_json::from_str(&json)?;
            sessions.push(metadata);
        }

        // 日時降順ソート（start_timeの降順）
        sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok(sessions)
    }

    /// セッション読み込み
    /// セッションディレクトリからsession.json, transcription.jsonl, audio.wavを読み込む
    /// Related requirement: STT-REQ-005.6
    pub fn load_session(&self, session_id: &str) -> Result<LoadedSession> {
        let session_dir = self.get_session_dir(session_id);

        // session.json読み込み
        let metadata_path = session_dir.join("session.json");
        let json = std::fs::read_to_string(&metadata_path)?;
        let metadata: SessionMetadata = serde_json::from_str(&json)?;

        // transcription.jsonl読み込み
        let transcript_path = session_dir.join("transcription.jsonl");
        let mut transcripts = Vec::new();

        if transcript_path.exists() {
            let content = std::fs::read_to_string(&transcript_path)?;
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let event: TranscriptionEvent = serde_json::from_str(line)?;
                transcripts.push(event);
            }
        }

        // audio.wavパス
        let audio_path = session_dir.join("audio.wav");

        Ok(LoadedSession {
            metadata,
            transcripts,
            audio_path,
        })
    }

    /// ディスク容量チェック
    /// Related requirement: STT-REQ-005.7, STT-REQ-005.8
    ///
    /// app_data_dirが配置されているファイルシステムの空き容量を確認
    /// - 1GB以上: DiskSpaceStatus::Sufficient
    /// - 500MB以上1GB未満: DiskSpaceStatus::Warning（警告ログ・通知）
    /// - 500MB未満: DiskSpaceStatus::Critical（録音開始拒否）
    pub fn check_disk_space(&self) -> Result<DiskSpaceStatus> {
        use fs2::available_space;

        // app_data_dirが配置されているファイルシステムの空き容量取得
        // 外付けHDDや別パーティションでも正確に取得可能
        let free_bytes = available_space(&self.app_data_dir)?;

        const ONE_GB: u64 = 1024 * 1024 * 1024;
        const FIVE_HUNDRED_MB: u64 = 500 * 1024 * 1024;

        let status = if free_bytes >= ONE_GB {
            DiskSpaceStatus::Sufficient
        } else if free_bytes >= FIVE_HUNDRED_MB {
            // 警告ログ記録（STT-REQ-005.7）
            eprintln!(
                "⚠️ ディスク容量警告: 残り容量 {} MB ({})",
                free_bytes / (1024 * 1024),
                self.app_data_dir.display()
            );
            DiskSpaceStatus::Warning
        } else {
            // クリティカルレベル（STT-REQ-005.8）
            eprintln!(
                "❌ ディスク容量クリティカル: 残り容量 {} MB ({})",
                free_bytes / (1024 * 1024),
                self.app_data_dir.display()
            );
            DiskSpaceStatus::Critical
        };

        Ok(status)
    }
}

/// WAVファイルへのストリーミング書き込み
/// 16kHz, モノラル, 16bit PCM形式
pub struct AudioWriter {
    file: std::fs::File,
    samples_written: u32,
}

impl AudioWriter {
    /// 新規WAVファイルライター作成
    fn new(wav_path: PathBuf) -> Result<Self> {
        let file = std::fs::File::create(&wav_path)?;
        let mut writer = Self {
            file,
            samples_written: 0,
        };
        writer.write_wav_header()?;
        Ok(writer)
    }

    /// WAVヘッダー書き込み（44バイト）
    /// 16kHz, モノラル, 16bit PCM
    fn write_wav_header(&mut self) -> Result<()> {
        use std::io::Write;

        // RIFFヘッダー
        self.file.write_all(b"RIFF")?;
        self.file.write_all(&0u32.to_le_bytes())?; // ファイルサイズ（後で更新）
        self.file.write_all(b"WAVE")?;

        // fmtチャンク
        self.file.write_all(b"fmt ")?;
        self.file.write_all(&16u32.to_le_bytes())?; // fmtチャンクサイズ
        self.file.write_all(&1u16.to_le_bytes())?;  // PCM形式
        self.file.write_all(&1u16.to_le_bytes())?;  // モノラル
        self.file.write_all(&16000u32.to_le_bytes())?; // サンプルレート 16kHz
        self.file.write_all(&32000u32.to_le_bytes())?; // バイトレート (16000 * 1 * 2)
        self.file.write_all(&2u16.to_le_bytes())?;  // ブロックアライン (1 * 2)
        self.file.write_all(&16u16.to_le_bytes())?; // ビット深度 16bit

        // dataチャンク
        self.file.write_all(b"data")?;
        self.file.write_all(&0u32.to_le_bytes())?; // データサイズ（後で更新）

        Ok(())
    }

    /// 音声データ書き込み（i16サンプル配列）
    /// Related requirement: STT-REQ-005.2
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<()> {
        use std::io::Write;

        for &sample in samples {
            self.file.write_all(&sample.to_le_bytes())?;
        }
        self.samples_written += samples.len() as u32;
        Ok(())
    }

    /// WAVファイルを閉じる（ヘッダー更新）
    /// Related requirement: STT-REQ-005.2
    pub fn close(mut self) -> Result<()> {
        self.finalize()
    }

    /// ヘッダー更新の内部実装
    fn finalize(&mut self) -> Result<()> {
        use std::io::{Seek, SeekFrom, Write};

        // ファイルサイズとデータサイズを更新
        let data_size = self.samples_written * 2; // 16bit = 2 bytes
        let file_size = data_size + 36; // RIFFヘッダー + fmtチャンク + dataヘッダー

        // RIFFチャンクサイズ更新（オフセット4）
        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&file_size.to_le_bytes())?;

        // dataチャンクサイズ更新（オフセット40）
        self.file.seek(SeekFrom::Start(40))?;
        self.file.write_all(&data_size.to_le_bytes())?;

        self.file.sync_all()?;
        Ok(())
    }
}

/// Drop実装: close()忘れ時の自動ヘッダー更新
/// Related requirement: STT-REQ-005.2（異常終了時のデータ保護）
impl Drop for AudioWriter {
    fn drop(&mut self) {
        // close()されていない場合でもヘッダー更新を試みる
        let _ = self.finalize();
    }
}

/// セッションメタデータ（session.json形式で保存）
/// Related requirement: STT-REQ-005.4
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    /// セッションID
    pub session_id: String,
    /// 録音開始時刻（ISO 8601形式）
    pub start_time: String,
    /// 録音終了時刻（ISO 8601形式）
    pub end_time: String,
    /// 録音時間（秒）
    pub duration_seconds: u64,
    /// 音声デバイス名
    pub audio_device: String,
    /// 使用したWhisperモデルサイズ
    pub model_size: String,
    /// 総セグメント数
    pub total_segments: u64,
    /// 総文字数
    pub total_characters: u64,
}

/// セッション読み込み結果
/// Related requirement: STT-REQ-005.6
#[derive(Debug, Clone)]
pub struct LoadedSession {
    pub metadata: SessionMetadata,
    pub transcripts: Vec<TranscriptionEvent>,
    pub audio_path: PathBuf,
}

/// ディスク容量ステータス
/// Related requirement: STT-REQ-005.7, STT-REQ-005.8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskSpaceStatus {
    /// 十分な容量（1GB以上）
    Sufficient,
    /// 警告レベル（500MB以上1GB未満）
    Warning,
    /// クリティカルレベル（500MB未満）
    Critical,
}

impl std::fmt::Display for DiskSpaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskSpaceStatus::Sufficient => write!(f, "十分な容量があります"),
            DiskSpaceStatus::Warning => write!(f, "ディスク容量が不足しています"),
            DiskSpaceStatus::Critical => {
                write!(f, "ディスク容量が不足しているため録音できません")
            }
        }
    }
}

/// 文字起こし結果イベント（JSON Lines形式で保存）
/// Related requirement: STT-REQ-005.3
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionEvent {
    /// タイムスタンプ（ミリ秒）
    pub timestamp_ms: u64,
    /// テキスト内容
    pub text: String,
    /// 確定テキストかどうか（false = 部分テキスト）
    pub is_final: bool,
}

/// transcription.jsonlへのJSON Lines書き込み
/// Related requirement: STT-REQ-005.3
pub struct TranscriptWriter {
    file: std::fs::File,
}

impl TranscriptWriter {
    /// 新規TranscriptWriter作成（追記モード）
    fn new(transcript_path: PathBuf) -> Result<Self> {
        use std::fs::OpenOptions;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&transcript_path)?;

        Ok(Self { file })
    }

    /// 文字起こし結果を追記
    /// JSON Lines形式（1行1JSONオブジェクト）
    /// Related requirement: STT-REQ-005.3
    ///
    /// **重要**: 各append後にsync_all()でディスク永続化を保証
    /// クラッシュ時のデータ欠損を最小化
    pub fn append_event(&mut self, event: &TranscriptionEvent) -> Result<()> {
        use std::io::Write;

        let json_line = serde_json::to_string(event)?;
        writeln!(self.file, "{}", json_line)?;

        // flush()はカーネルバッファまで、sync_all()でディスク永続化
        self.file.flush()?;
        self.file.sync_all()?;

        Ok(())
    }

    /// ファイルを閉じる
    pub fn close(mut self) -> Result<()> {
        self.finalize()
    }

    /// ファイルの最終同期処理
    fn finalize(&mut self) -> Result<()> {
        self.file.sync_all()?;
        Ok(())
    }
}

/// Drop実装: close()忘れ時の自動sync_all()
/// Related requirement: STT-REQ-005.3（異常終了時のデータ保護）
impl Drop for TranscriptWriter {
    fn drop(&mut self) {
        // close()されていない場合でもsync_all()を試みる
        let _ = self.finalize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_service() -> (LocalStorageService, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let service = LocalStorageService::new(temp_dir.path().to_path_buf());
        (service, temp_dir)
    }

    #[test]
    fn test_generate_session_id() {
        let (service, _temp_dir) = setup_test_service();

        // Act: セッションID生成
        let session_id = service.generate_session_id();

        // Assert: UUIDフォーマット確認（8-4-4-4-12の形式）
        let parts: Vec<&str> = session_id.split('-').collect();
        assert_eq!(
            parts.len(),
            5,
            "UUID should have 5 parts separated by hyphens"
        );
        assert_eq!(parts[0].len(), 8, "First part should be 8 chars");
        assert_eq!(parts[1].len(), 4, "Second part should be 4 chars");
        assert_eq!(parts[2].len(), 4, "Third part should be 4 chars");
        assert_eq!(parts[3].len(), 4, "Fourth part should be 4 chars");
        assert_eq!(parts[4].len(), 12, "Fifth part should be 12 chars");
    }

    #[test]
    fn test_generate_session_id_uniqueness() {
        let (service, _temp_dir) = setup_test_service();

        // Act: 複数回生成
        let id1 = service.generate_session_id();
        let id2 = service.generate_session_id();

        // Assert: 異なるIDが生成される
        assert_ne!(id1, id2, "Generated UUIDs should be unique");
    }

    #[test]
    fn test_create_session_directory() {
        let (service, temp_dir) = setup_test_service();

        // Arrange: セッションID生成
        let session_id = "test-session-12345";

        // Act: セッションディレクトリ作成
        let session_path = service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Assert: ディレクトリが作成されている
        assert!(session_path.exists(), "Session directory should exist");
        assert!(session_path.is_dir(), "Session path should be a directory");

        // Assert: パスが正しい構造
        let expected_path = temp_dir.path().join("recordings").join(session_id);
        assert_eq!(
            session_path, expected_path,
            "Session path should match expected structure"
        );
    }

    #[test]
    fn test_create_session_nested_directory() {
        let (service, temp_dir) = setup_test_service();

        // Arrange: recordings/ディレクトリが存在しない状態
        let recordings_dir = temp_dir.path().join("recordings");
        assert!(
            !recordings_dir.exists(),
            "recordings/ should not exist initially"
        );

        let session_id = "nested-test-session";

        // Act: セッションディレクトリ作成（親ディレクトリも自動作成）
        let session_path = service
            .create_session(session_id)
            .expect("create_session should create parent directories");

        // Assert: 親ディレクトリも作成されている
        assert!(
            recordings_dir.exists(),
            "recordings/ parent directory should be created"
        );
        assert!(session_path.exists(), "Session directory should exist");
    }

    #[test]
    fn test_get_session_dir() {
        let (service, temp_dir) = setup_test_service();
        let session_id = "path-test-session";

        // Act: セッションディレクトリパス取得
        let session_path = service.get_session_dir(session_id);

        // Assert: パスが正しい
        let expected_path = temp_dir.path().join("recordings").join(session_id);
        assert_eq!(session_path, expected_path);
    }

    // === Task 6.2: 音声ファイル保存機能のテスト ===

    #[test]
    fn test_create_audio_writer() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "audio-test-session";

        // Arrange: セッションディレクトリ作成
        service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Act: AudioWriter作成
        let writer = service
            .create_audio_writer(session_id)
            .expect("create_audio_writer should succeed");

        // Assert: audio.wavファイルが作成されている
        let audio_path = service.get_session_dir(session_id).join("audio.wav");
        assert!(audio_path.exists(), "audio.wav should be created");

        // Cleanup: writerを閉じる
        writer.close().expect("close should succeed");
    }

    #[test]
    fn test_audio_writer_write_samples() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "write-samples-session";

        // Arrange: セッションとAudioWriter作成
        service
            .create_session(session_id)
            .expect("create_session should succeed");
        let mut writer = service
            .create_audio_writer(session_id)
            .expect("create_audio_writer should succeed");

        // Act: サンプルデータ書き込み（1秒分 = 16000サンプル）
        let samples: Vec<i16> = (0..16000).map(|i| (i % 1000) as i16).collect();
        writer
            .write_samples(&samples)
            .expect("write_samples should succeed");

        // Act: ファイルを閉じる
        writer.close().expect("close should succeed");

        // Assert: ファイルサイズ確認
        let audio_path = service.get_session_dir(session_id).join("audio.wav");
        let metadata = std::fs::metadata(&audio_path).expect("metadata should succeed");

        // 期待サイズ = WAVヘッダー(44bytes) + サンプルデータ(16000 * 2bytes)
        let expected_size = 44 + (16000 * 2);
        assert_eq!(
            metadata.len(),
            expected_size,
            "WAV file size should match expected"
        );
    }

    #[test]
    fn test_audio_writer_multiple_writes() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "multi-write-session";

        // Arrange
        service
            .create_session(session_id)
            .expect("create_session should succeed");
        let mut writer = service
            .create_audio_writer(session_id)
            .expect("create_audio_writer should succeed");

        // Act: 複数回書き込み（ストリーミング）
        for i in 0..10 {
            let samples: Vec<i16> = vec![i * 100; 1600]; // 0.1秒ずつ
            writer
                .write_samples(&samples)
                .expect("write_samples should succeed");
        }

        writer.close().expect("close should succeed");

        // Assert: 合計1秒分のデータが書き込まれている
        let audio_path = service.get_session_dir(session_id).join("audio.wav");
        let metadata = std::fs::metadata(&audio_path).expect("metadata should succeed");

        let total_samples = 10 * 1600;
        let expected_size = 44 + (total_samples * 2);
        assert_eq!(metadata.len(), expected_size);
    }

    #[test]
    fn test_audio_writer_drop_without_close() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        let session_id = storage.generate_session_id();
        storage.create_session(&session_id).unwrap();

        let audio_path = storage.get_session_dir(&session_id).join("audio.wav");

        // close()を呼ばずにスコープを抜ける（Drop実行）
        {
            let mut writer = storage.create_audio_writer(&session_id).unwrap();
            let samples: Vec<i16> = vec![100, 200, 300, 400, 500];
            writer.write_samples(&samples).unwrap();
            // close()を呼ばずにdrop
        }

        // 検証: ヘッダーが正しく更新されている
        let file_content = std::fs::read(&audio_path).unwrap();

        // RIFFヘッダー確認
        assert_eq!(&file_content[0..4], b"RIFF");

        // ファイルサイズ確認（5サンプル * 2バイト + 36 = 46バイト）
        let file_size = u32::from_le_bytes([
            file_content[4],
            file_content[5],
            file_content[6],
            file_content[7],
        ]);
        assert_eq!(file_size, 46); // 10 bytes data + 36 bytes header

        // dataチャンクサイズ確認
        let data_size = u32::from_le_bytes([
            file_content[40],
            file_content[41],
            file_content[42],
            file_content[43],
        ]);
        assert_eq!(data_size, 10); // 5 samples * 2 bytes
    }

    #[test]
    fn test_audio_writer_wav_header() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "wav-header-session";

        // Arrange
        service
            .create_session(session_id)
            .expect("create_session should succeed");
        let writer = service
            .create_audio_writer(session_id)
            .expect("create_audio_writer should succeed");

        writer.close().expect("close should succeed");

        // Act: WAVヘッダーを読み込む
        let audio_path = service.get_session_dir(session_id).join("audio.wav");
        let wav_data = std::fs::read(&audio_path).expect("read should succeed");

        // Assert: RIFFヘッダー確認
        assert_eq!(&wav_data[0..4], b"RIFF", "RIFF magic should be present");
        assert_eq!(&wav_data[8..12], b"WAVE", "WAVE format should be present");

        // Assert: fmtチャンク確認
        assert_eq!(&wav_data[12..16], b"fmt ", "fmt chunk should be present");
        let audio_format = u16::from_le_bytes([wav_data[20], wav_data[21]]);
        assert_eq!(audio_format, 1, "Audio format should be PCM (1)");

        let num_channels = u16::from_le_bytes([wav_data[22], wav_data[23]]);
        assert_eq!(num_channels, 1, "Should be mono (1 channel)");

        let sample_rate = u32::from_le_bytes([wav_data[24], wav_data[25], wav_data[26], wav_data[27]]);
        assert_eq!(sample_rate, 16000, "Sample rate should be 16kHz");

        let bits_per_sample = u16::from_le_bytes([wav_data[34], wav_data[35]]);
        assert_eq!(bits_per_sample, 16, "Bit depth should be 16bit");

        // Assert: dataチャンク確認
        assert_eq!(&wav_data[36..40], b"data", "data chunk should be present");
    }

    // === Task 6.3: 文字起こし結果保存機能のテスト ===

    #[test]
    fn test_create_transcript_writer() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "transcript-test-session";

        // Arrange: セッションディレクトリ作成
        service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Act: TranscriptWriter作成
        let writer = service
            .create_transcript_writer(session_id)
            .expect("create_transcript_writer should succeed");

        // Assert: transcription.jsonlファイルが作成されている
        let transcript_path = service
            .get_session_dir(session_id)
            .join("transcription.jsonl");
        assert!(
            transcript_path.exists(),
            "transcription.jsonl should be created"
        );

        // Cleanup
        writer.close().expect("close should succeed");
    }

    #[test]
    fn test_transcript_writer_append_event() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "append-event-session";

        // Arrange
        service
            .create_session(session_id)
            .expect("create_session should succeed");
        let mut writer = service
            .create_transcript_writer(session_id)
            .expect("create_transcript_writer should succeed");

        // Act: 部分テキストイベント追記
        let event1 = TranscriptionEvent {
            timestamp_ms: 1000,
            text: "これは部分".to_string(),
            is_final: false,
        };
        writer
            .append_event(&event1)
            .expect("append_event should succeed");

        // Act: 確定テキストイベント追記
        let event2 = TranscriptionEvent {
            timestamp_ms: 2000,
            text: "これは確定テキストです。".to_string(),
            is_final: true,
        };
        writer
            .append_event(&event2)
            .expect("append_event should succeed");

        writer.close().expect("close should succeed");

        // Assert: ファイル内容確認
        let transcript_path = service
            .get_session_dir(session_id)
            .join("transcription.jsonl");
        let content = std::fs::read_to_string(&transcript_path).expect("read should succeed");

        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "Should have 2 JSON lines");

        // Assert: 1行目のJSON解析
        let parsed1: TranscriptionEvent =
            serde_json::from_str(lines[0]).expect("line 1 should be valid JSON");
        assert_eq!(parsed1, event1, "First event should match");

        // Assert: 2行目のJSON解析
        let parsed2: TranscriptionEvent =
            serde_json::from_str(lines[1]).expect("line 2 should be valid JSON");
        assert_eq!(parsed2, event2, "Second event should match");
    }

    #[test]
    fn test_transcript_writer_append_mode() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "append-mode-session";

        // Arrange
        service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Act: 1回目の書き込み
        let mut writer1 = service
            .create_transcript_writer(session_id)
            .expect("create_transcript_writer should succeed");
        writer1
            .append_event(&TranscriptionEvent {
                timestamp_ms: 1000,
                text: "最初のイベント".to_string(),
                is_final: false,
            })
            .expect("append_event should succeed");
        writer1.close().expect("close should succeed");

        // Act: 2回目の書き込み（追記モード）
        let mut writer2 = service
            .create_transcript_writer(session_id)
            .expect("create_transcript_writer should succeed");
        writer2
            .append_event(&TranscriptionEvent {
                timestamp_ms: 2000,
                text: "2番目のイベント".to_string(),
                is_final: true,
            })
            .expect("append_event should succeed");
        writer2.close().expect("close should succeed");

        // Assert: 両方のイベントが保存されている
        let transcript_path = service
            .get_session_dir(session_id)
            .join("transcription.jsonl");
        let content = std::fs::read_to_string(&transcript_path).expect("read should succeed");

        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "Should have 2 JSON lines (append mode)"
        );
    }

    #[test]
    fn test_transcript_writer_drop_without_close() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        let session_id = storage.generate_session_id();
        storage.create_session(&session_id).unwrap();

        let transcript_path = storage.get_session_dir(&session_id).join("transcription.jsonl");

        // close()を呼ばずにスコープを抜ける（Drop実行）
        {
            let mut writer = storage.create_transcript_writer(&session_id).unwrap();
            let event1 = TranscriptionEvent {
                timestamp_ms: 1000,
                text: "Hello".to_string(),
                is_final: false,
            };
            let event2 = TranscriptionEvent {
                timestamp_ms: 2000,
                text: "Hello world".to_string(),
                is_final: true,
            };
            writer.append_event(&event1).unwrap();
            writer.append_event(&event2).unwrap();
            // close()を呼ばずにdrop（sync_all()が自動実行される）
        }

        // 検証: データがディスクに永続化されている
        let content = std::fs::read_to_string(&transcript_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2);

        let parsed1: TranscriptionEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed1.text, "Hello");

        let parsed2: TranscriptionEvent = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed2.text, "Hello world");
    }

    #[test]
    fn test_transcription_event_json_format() {
        // Arrange
        let event = TranscriptionEvent {
            timestamp_ms: 12345,
            text: "テストテキスト".to_string(),
            is_final: true,
        };

        // Act: JSON変換
        let json = serde_json::to_string(&event).expect("serialization should succeed");

        // Assert: JSON形式確認
        assert!(json.contains("\"timestamp_ms\":12345"));
        assert!(json.contains("\"text\":\"テストテキスト\""));
        assert!(json.contains("\"is_final\":true"));

        // Act: JSON逆変換
        let parsed: TranscriptionEvent =
            serde_json::from_str(&json).expect("deserialization should succeed");

        // Assert: 元のデータと一致
        assert_eq!(parsed, event);
    }

    // === Task 6.4: セッションメタデータ保存機能のテスト ===

    #[test]
    fn test_save_session_metadata() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "metadata-test-session";

        // Arrange: セッションディレクトリ作成
        service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Arrange: メタデータ作成
        let metadata = SessionMetadata {
            session_id: session_id.to_string(),
            start_time: "2025-10-02T10:00:00Z".to_string(),
            end_time: "2025-10-02T11:30:00Z".to_string(),
            duration_seconds: 5400,
            audio_device: "MacBook Pro Microphone".to_string(),
            model_size: "small".to_string(),
            total_segments: 150,
            total_characters: 12000,
        };

        // Act: メタデータ保存
        service
            .save_session_metadata(&metadata)
            .expect("save_session_metadata should succeed");

        // Assert: session.jsonファイルが作成されている
        let metadata_path = service.get_session_dir(session_id).join("session.json");
        assert!(metadata_path.exists(), "session.json should be created");

        // Assert: ファイル内容確認
        let content = std::fs::read_to_string(&metadata_path).expect("read should succeed");
        let parsed: SessionMetadata =
            serde_json::from_str(&content).expect("JSON should be valid");

        assert_eq!(parsed, metadata, "Parsed metadata should match original");
    }

    #[test]
    fn test_session_metadata_json_format() {
        // Arrange
        let metadata = SessionMetadata {
            session_id: "test-uuid-1234".to_string(),
            start_time: "2025-10-02T10:00:00Z".to_string(),
            end_time: "2025-10-02T11:30:00Z".to_string(),
            duration_seconds: 5400,
            audio_device: "Test Device".to_string(),
            model_size: "small".to_string(),
            total_segments: 150,
            total_characters: 12000,
        };

        // Act: JSON変換
        let json = serde_json::to_string_pretty(&metadata).expect("serialization should succeed");

        // Assert: JSON形式確認（全フィールド存在）
        assert!(json.contains("\"session_id\""));
        assert!(json.contains("\"test-uuid-1234\""));
        assert!(json.contains("\"start_time\""));
        assert!(json.contains("\"2025-10-02T10:00:00Z\""));
        assert!(json.contains("\"end_time\""));
        assert!(json.contains("\"duration_seconds\""));
        assert!(json.contains("5400"));
        assert!(json.contains("\"audio_device\""));
        assert!(json.contains("\"Test Device\""));
        assert!(json.contains("\"model_size\""));
        assert!(json.contains("\"small\""));
        assert!(json.contains("\"total_segments\""));
        assert!(json.contains("150"));
        assert!(json.contains("\"total_characters\""));
        assert!(json.contains("12000"));

        // Act: JSON逆変換
        let parsed: SessionMetadata =
            serde_json::from_str(&json).expect("deserialization should succeed");

        // Assert: 元のデータと一致
        assert_eq!(parsed, metadata);
    }

    #[test]
    fn test_save_session_metadata_overwrite() {
        let (service, _temp_dir) = setup_test_service();
        let session_id = "overwrite-session";

        // Arrange
        service
            .create_session(session_id)
            .expect("create_session should succeed");

        // Act: 1回目の保存
        let metadata1 = SessionMetadata {
            session_id: session_id.to_string(),
            start_time: "2025-10-02T10:00:00Z".to_string(),
            end_time: "2025-10-02T10:30:00Z".to_string(),
            duration_seconds: 1800,
            audio_device: "Device 1".to_string(),
            model_size: "tiny".to_string(),
            total_segments: 50,
            total_characters: 3000,
        };
        service
            .save_session_metadata(&metadata1)
            .expect("save should succeed");

        // Act: 2回目の保存（上書き）
        let metadata2 = SessionMetadata {
            session_id: session_id.to_string(),
            start_time: "2025-10-02T10:00:00Z".to_string(),
            end_time: "2025-10-02T11:00:00Z".to_string(),
            duration_seconds: 3600,
            audio_device: "Device 2".to_string(),
            model_size: "small".to_string(),
            total_segments: 100,
            total_characters: 8000,
        };
        service
            .save_session_metadata(&metadata2)
            .expect("save should succeed");

        // Assert: 最新のメタデータが保存されている
        let metadata_path = service.get_session_dir(session_id).join("session.json");
        let content = std::fs::read_to_string(&metadata_path).expect("read should succeed");
        let parsed: SessionMetadata =
            serde_json::from_str(&content).expect("JSON should be valid");

        assert_eq!(
            parsed, metadata2,
            "Should have latest metadata (overwrite)"
        );
    }

    #[test]
    fn test_session_metadata_iso8601_timestamps() {
        // Arrange
        let metadata = SessionMetadata {
            session_id: "timestamp-test".to_string(),
            start_time: "2025-10-13T15:30:45.123Z".to_string(),
            end_time: "2025-10-13T16:45:30.456Z".to_string(),
            duration_seconds: 4485,
            audio_device: "Device".to_string(),
            model_size: "medium".to_string(),
            total_segments: 200,
            total_characters: 15000,
        };

        // Act: JSON変換・逆変換
        let json = serde_json::to_string(&metadata).expect("serialization should succeed");
        let parsed: SessionMetadata =
            serde_json::from_str(&json).expect("deserialization should succeed");

        // Assert: ISO 8601タイムスタンプが正しく保存される
        assert_eq!(parsed.start_time, "2025-10-13T15:30:45.123Z");
        assert_eq!(parsed.end_time, "2025-10-13T16:45:30.456Z");
    }

    // ================================================================================
    // Task 6.5: セッション一覧取得と再生機能テスト (RED)
    // Related requirement: STT-REQ-005.5, STT-REQ-005.6
    // ================================================================================

    #[test]
    fn test_list_sessions() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // 3つのセッション作成
        let session1 = storage.generate_session_id();
        let session2 = storage.generate_session_id();
        let session3 = storage.generate_session_id();

        storage.create_session(&session1).unwrap();
        storage.create_session(&session2).unwrap();
        storage.create_session(&session3).unwrap();

        // メタデータ保存（異なる日時）
        let metadata1 = SessionMetadata {
            session_id: session1.clone(),
            start_time: "2025-10-13T10:00:00Z".to_string(),
            end_time: "2025-10-13T10:30:00Z".to_string(),
            duration_seconds: 1800,
            audio_device: "default".to_string(),
            model_size: "small".to_string(),
            total_segments: 10,
            total_characters: 500,
        };
        let metadata2 = SessionMetadata {
            session_id: session2.clone(),
            start_time: "2025-10-13T11:00:00Z".to_string(),
            end_time: "2025-10-13T11:15:00Z".to_string(),
            duration_seconds: 900,
            audio_device: "default".to_string(),
            model_size: "small".to_string(),
            total_segments: 5,
            total_characters: 250,
        };
        let metadata3 = SessionMetadata {
            session_id: session3.clone(),
            start_time: "2025-10-13T09:00:00Z".to_string(),
            end_time: "2025-10-13T09:45:00Z".to_string(),
            duration_seconds: 2700,
            audio_device: "default".to_string(),
            model_size: "small".to_string(),
            total_segments: 15,
            total_characters: 750,
        };

        storage.save_session_metadata(&metadata1).unwrap();
        storage.save_session_metadata(&metadata2).unwrap();
        storage.save_session_metadata(&metadata3).unwrap();

        // セッション一覧取得（日時降順）
        let sessions = storage.list_sessions().unwrap();

        // 検証: 3セッション取得
        assert_eq!(sessions.len(), 3);

        // 検証: 日時降順ソート（11:00 > 10:00 > 09:00）
        assert_eq!(sessions[0].session_id, session2);
        assert_eq!(sessions[1].session_id, session1);
        assert_eq!(sessions[2].session_id, session3);
    }

    #[test]
    fn test_load_session() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        let session_id = storage.generate_session_id();
        storage.create_session(&session_id).unwrap();

        // メタデータ保存
        let metadata = SessionMetadata {
            session_id: session_id.clone(),
            start_time: "2025-10-13T10:00:00Z".to_string(),
            end_time: "2025-10-13T10:30:00Z".to_string(),
            duration_seconds: 1800,
            audio_device: "default".to_string(),
            model_size: "small".to_string(),
            total_segments: 10,
            total_characters: 500,
        };
        storage.save_session_metadata(&metadata).unwrap();

        // 文字起こしイベント保存
        let mut transcript_writer = storage.create_transcript_writer(&session_id).unwrap();
        let event1 = TranscriptionEvent {
            timestamp_ms: 1000,
            text: "Hello".to_string(),
            is_final: false,
        };
        let event2 = TranscriptionEvent {
            timestamp_ms: 2000,
            text: "Hello world".to_string(),
            is_final: true,
        };
        transcript_writer.append_event(&event1).unwrap();
        transcript_writer.append_event(&event2).unwrap();
        transcript_writer.close().unwrap();

        // セッション読み込み
        let loaded_session = storage.load_session(&session_id).unwrap();

        // 検証: メタデータ一致
        assert_eq!(loaded_session.metadata.session_id, session_id);
        assert_eq!(loaded_session.metadata.duration_seconds, 1800);

        // 検証: 文字起こしイベント一致
        assert_eq!(loaded_session.transcripts.len(), 2);
        assert_eq!(loaded_session.transcripts[0].text, "Hello");
        assert_eq!(loaded_session.transcripts[1].text, "Hello world");

        // 検証: 音声ファイルパス存在
        assert!(loaded_session.audio_path.ends_with("audio.wav"));
    }

    #[test]
    fn test_list_sessions_empty() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // 空のrecordingsディレクトリ
        let sessions = storage.list_sessions().unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn test_load_session_not_found() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // 存在しないセッション読み込み
        let result = storage.load_session("non-existent-session");
        assert!(result.is_err());
    }

    // ================================================================================
    // Task 6.6: ディスク容量監視と警告機能テスト (RED)
    // Related requirement: STT-REQ-005.7, STT-REQ-005.8
    // ================================================================================

    #[test]
    fn test_check_disk_space_sufficient() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // ディスク容量チェック（十分な容量がある場合）
        let result = storage.check_disk_space();
        assert!(result.is_ok());

        let status = result.unwrap();
        assert_eq!(status, DiskSpaceStatus::Sufficient);
    }

    #[test]
    fn test_check_disk_space_warning() {
        use super::*;

        // 1GB未満（警告レベル）の容量をシミュレート
        // 注: 実際のテストではモック必要、ここではAPI確認のみ
        let status = DiskSpaceStatus::Warning;
        assert_eq!(status.to_string(), "ディスク容量が不足しています");
    }

    #[test]
    fn test_check_disk_space_critical() {
        use super::*;

        // 500MB未満（クリティカルレベル）の容量をシミュレート
        let status = DiskSpaceStatus::Critical;
        assert_eq!(
            status.to_string(),
            "ディスク容量が不足しているため録音できません"
        );
    }

    #[test]
    fn test_begin_session() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // セッション開始（原子的操作）
        let handle = storage.begin_session().unwrap();

        // 検証: セッションIDが生成されている
        assert!(!handle.session_id.is_empty());

        // 検証: セッションディレクトリが作成されている
        assert!(handle.session_dir.exists());

        // 検証: ディスク容量ステータスが設定されている
        // (通常の開発環境では Sufficient または Warning)
        assert!(
            handle.disk_status == DiskSpaceStatus::Sufficient
                || handle.disk_status == DiskSpaceStatus::Warning
        );

        // 検証: 音声ライターが取得できる
        let audio_writer = handle.audio_writer();
        assert!(audio_writer.is_ok());

        // 検証: 文字起こしライターが取得できる
        let transcript_writer = handle.transcript_writer();
        assert!(transcript_writer.is_ok());
    }

    #[test]
    fn test_session_handle_disk_warning() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        let handle = storage.begin_session().unwrap();

        // needs_disk_warning() のテスト
        let needs_warning = handle.needs_disk_warning();
        assert_eq!(needs_warning, handle.disk_status == DiskSpaceStatus::Warning);

        // disk_warning_message() のテスト
        if handle.disk_status == DiskSpaceStatus::Warning {
            let msg = handle.disk_warning_message();
            assert!(msg.is_some());
            assert!(msg.unwrap().contains("1GB未満"));
        } else {
            assert!(handle.disk_warning_message().is_none());
        }
    }

    #[test]
    fn test_begin_session_with_writers() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // セッション開始
        let handle = storage.begin_session().unwrap();

        // 音声書き込み
        let mut audio_writer = handle.audio_writer().unwrap();
        let samples: Vec<i16> = vec![100, 200, 300];
        audio_writer.write_samples(&samples).unwrap();
        audio_writer.close().unwrap();

        // 文字起こし書き込み
        let mut transcript_writer = handle.transcript_writer().unwrap();
        let event = TranscriptionEvent {
            timestamp_ms: 1000,
            text: "Test".to_string(),
            is_final: true,
        };
        transcript_writer.append_event(&event).unwrap();
        transcript_writer.close().unwrap();

        // メタデータ保存
        let metadata = SessionMetadata {
            session_id: handle.session_id.clone(),
            start_time: "2025-10-13T10:00:00Z".to_string(),
            end_time: "2025-10-13T10:05:00Z".to_string(),
            duration_seconds: 300,
            audio_device: "default".to_string(),
            model_size: "small".to_string(),
            total_segments: 1,
            total_characters: 4,
        };
        handle.save_metadata(&metadata).unwrap();

        // 検証: 全ファイルが作成されている
        assert!(handle.session_dir.join("audio.wav").exists());
        assert!(handle.session_dir.join("transcription.jsonl").exists());
        assert!(handle.session_dir.join("session.json").exists());
    }

    #[test]
    fn test_create_session_with_disk_check() {
        use super::*;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageService::new(temp_dir.path().to_path_buf());

        // セッション作成前のディスク容量チェック
        let disk_status = storage.check_disk_space().unwrap();

        // 十分な容量がある場合のみセッション作成
        if disk_status == DiskSpaceStatus::Sufficient {
            let session_id = storage.generate_session_id();
            let result = storage.create_session(&session_id);
            assert!(result.is_ok());
        }
    }
}
