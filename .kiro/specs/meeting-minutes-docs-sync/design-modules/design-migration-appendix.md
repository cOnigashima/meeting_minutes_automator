# Technical Design - meeting-minutes-docs-sync: Migration & Appendix

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Migration Strategy

本機能は既存システム（meeting-minutes-core、meeting-minutes-stt）への拡張であり、段階的な移行戦略を採用します。

### Phase 1: Chrome拡張への認証レイヤー追加 (Week 1)

**目標**: OAuth 2.0認証フローの実装とトークン管理機能の追加

**タスク**:
1. AuthManagerコンポーネントの実装
2. TokenStoreコンポーネントの実装
3. ユニットテスト実装（AuthManager、TokenStore）
4. Popup UIに「Google連携」ボタンを追加

**検証基準**:
- ユーザーがGoogle連携ボタンをクリックし、OAuth 2.0認証フローが完了する
- トークンが`chrome.storage.local`に保存される
- トークンリフレッシュが正常に動作する

**ロールバック戦略**:
- OAuth 2.0機能は既存機能に影響を与えないため、機能フラグで無効化可能
- `chrome.storage.local`から`auth_tokens`キーを削除することで、認証状態をリセット

---

### Phase 2: Google Docs API統合 (Week 2)

**目標**: Google Docs APIクライアントとNamed Range管理機能の実装

**タスク**:
1. GoogleDocsClientコンポーネントの実装
2. NamedRangeManagerコンポーネントの実装
3. ユニットテスト実装（GoogleDocsClient、NamedRangeManager）
4. 統合テスト実装（OAuth 2.0認証 → API呼び出し）

**検証基準**:
- Google Docs APIへのリクエストが正常に送信される
- Named Rangeが作成され、テキストが正しい位置に挿入される
- エラーハンドリング（401, 403, 429, 500-504）が正常に動作する

**ロールバック戦略**:
- Google Docs API機能を無効化し、オフラインキューモードのみで動作
- Phase 1の状態に戻す

---

### Phase 3: オフラインキューと自動再同期 (Week 3)

**目標**: オフライン時のメッセージキューイングと自動再同期機能の実装

**タスク**:
1. SyncManagerコンポーネントの実装
2. QueueManagerコンポーネントの実装
3. オフライン/オンライン切り替え検知ロジックの実装
4. ユニットテスト実装（SyncManager、QueueManager）
5. 統合テスト実装（オフライン → オンライン復帰 → 自動再同期）

**検証基準**:
- ネットワーク切断時にメッセージがオフラインキューに保存される
- ネットワーク復帰時に自動再同期が実行される
- ストレージ使用量の警告が正常に表示される

**ロールバック戦略**:
- オフラインキュー機能を無効化し、オンライン同期のみで動作
- Phase 2の状態に戻す

---

### Phase 4: WebSocketプロトコル拡張 (Week 4)

**目標**: WebSocketメッセージに`docsSync`フィールドを追加し、Tauriアプリとの双方向通信を確立

**タスク**:
1. WebSocketメッセージ形式の拡張（`docsSync`フィールド追加）
2. Tauriアプリ側でのメッセージ受信ロジック実装
3. 統合テスト実装（Chrome拡張 → Tauriアプリへのイベント送信）

**検証基準**:
- `docs_sync_started`、`docs_sync_success`等のイベントがTauriアプリへ送信される
- Tauriアプリ側で同期ステータスが正しく表示される

**ロールバック戦略**:
- WebSocketメッセージ形式を元に戻す（`docsSync`フィールドを削除）
- Phase 3の状態に戻す

---

### Phase 5: E2Eテストとユーザー受け入れテスト (Week 5)

**目標**: 全機能の統合テストとユーザー受け入れテスト

**タスク**:
1. E2Eテストスイートの実装
2. ユーザー受け入れテスト（UAT）の実施
3. バグ修正とパフォーマンス最適化

**検証基準**:
- 全E2Eテストが成功する
- ユーザーが実際の会議で使用し、問題なく動作する

**ロールバック戦略**:
- 重大なバグが発見された場合、Phase 4の状態に戻す
- 本番リリースを延期し、修正後に再テスト

---

### Rollback Triggers

以下の条件を満たす場合、ロールバックを実行します:

1. **認証失敗率が50%以上**: OAuth 2.0認証が頻繁に失敗する
2. **API呼び出し成功率が80%未満**: Google Docs API呼び出しが頻繁に失敗する
3. **オフラインキューの保存失敗率が10%以上**: ストレージ書き込みが頻繁に失敗する
4. **クリティカルなセキュリティ脆弱性の発見**: トークン漏洩やXSS攻撃のリスク

---

### Validation Checkpoints

各フェーズ完了後、以下のチェックポイントで検証を実施します:

**Phase 1 完了時**:
- [ ] OAuth 2.0認証フローが正常に動作する
- [ ] トークンが`chrome.storage.local`に保存される
- [ ] トークンリフレッシュが正常に動作する
- [ ] ユニットテストカバレッジ80%以上

**Phase 2 完了時**:
- [ ] Google Docs APIへのリクエストが成功する
- [ ] Named Rangeが正しく作成される
- [ ] テキストが正しい位置に挿入される
- [ ] エラーハンドリングが正常に動作する

**Phase 3 完了時**:
- [ ] オフライン時にメッセージがキューに保存される
- [ ] ネットワーク復帰時に自動再同期が実行される
- [ ] ストレージ使用量の警告が表示される

**Phase 4 完了時**:
- [ ] WebSocketメッセージに`docsSync`フィールドが含まれる
- [ ] Tauriアプリでイベントが正しく受信される

**Phase 5 完了時**:
- [ ] 全E2Eテストが成功する
- [ ] ユーザー受け入れテストが完了する
- [ ] パフォーマンス目標を達成する

---

## Appendix

### Related Documents

- **Umbrella Spec**: `.kiro/specs/meeting-minutes-automator/requirements.md` - REQ-003.2（Google Docs連携要件）
- **Steering Documents**:
  - `tech.md`: Chrome拡張技術スタック、WebSocket通信仕様
  - `structure.md`: Chrome拡張のディレクトリ構造
  - `principles.md`: オフラインファースト原則、セキュリティ責任境界の原則
- **Upstream Dependencies**:
  - `.kiro/specs/meeting-minutes-core/design.md`: WebSocketサーバー設計
  - `.kiro/specs/meeting-minutes-stt/design.md`: WebSocketメッセージ形式
- **Research Report**: `.kiro/research/meeting-minutes-docs-sync-technical-research.md` - Google Docs API、OAuth 2.0、Chrome Storage API調査結果

### External References

- **Google Docs API v1 Documentation**: https://developers.google.com/docs/api
- **OAuth 2.0 for Chrome Extensions**: https://developer.chrome.com/docs/extensions/mv3/oauth2/
- **Chrome Storage API**: https://developer.chrome.com/docs/extensions/reference/storage/
- **Exponential Backoff Pattern**: https://cloud.google.com/storage/docs/retry-strategy

### Glossary

| 用語 | 定義 |
|-----|------|
| **OAuth 2.0** | Googleアカウント認証のための業界標準プロトコル。ユーザーの明示的な許可を得てAPIアクセス権を取得。 |
| **Google Docs API** | Google Docsドキュメントをプログラマティックに操作するためのREST API。 |
| **Named Range** | Google Docs内の特定位置に付けられた名前付き範囲。プログラムから特定の位置を一意に識別するために使用。 |
| **batchUpdate** | Google Docs APIのメソッド。複数の編集操作を1つのリクエストにまとめて実行し、API呼び出し回数を削減。 |
| **オフラインキュー** | ネットワーク切断時に同期待ちのメッセージをローカルに保存するキュー。再接続時に自動再送信。 |
| **同期カーソル** | Google Docs内の現在の挿入位置を示すカーソル。文字起こし結果を順次追加するために使用。 |
| **トークンリフレッシュ** | OAuth 2.0アクセストークンの有効期限切れ時に、リフレッシュトークンを使用して新しいアクセストークンを取得する処理。 |
| **Exponential Backoff** | APIリクエスト失敗時に、再試行間隔を指数関数的に増加させるリトライ戦略。サーバー負荷を分散し、成功率を向上させる。 |
| **chrome.storage.local** | Chrome拡張機能でデータを永続化するためのストレージAPI。Service Workerで使用可能。 |
| **Service Worker** | Chrome拡張機能のバックグラウンド処理を担当するスクリプト。イベント駆動で動作し、5分でタイムアウト。 |

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-03 | 1.0 | Claude Code | 初版作成（meeting-minutes-docs-sync技術設計） |
