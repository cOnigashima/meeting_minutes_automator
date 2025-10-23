# Phase 5: E2E Testing & User Acceptance (Week 5)

> **親ドキュメント**: [tasks.md](../tasks.md) | [task-details/README.md](README.md)
> **関連設計**: [design-testing-security.md](../design-modules/design-testing-security.md)
> **Requirements**: 全要件の検証、DOCS-REQ-008.1-5

## Goal

E2Eテストとユーザー受け入れテスト。6シナリオのE2Eテスト、パフォーマンステスト、セキュリティテスト、ユーザー設定機能、ドキュメント作成、UAT実施。

---

### 12. E2Eテストスイートの実装

_Requirements: 全要件の検証_

#### 12.1 E2Eテストシナリオの作成

全機能を統合したE2Eテストシナリオを作成する。

**受け入れ基準**:
- [ ] シナリオ1: OAuth 2.0認証フロー → ドキュメント選択 → 同期開始
- [ ] シナリオ2: リアルタイム同期（文字起こし → Google Docs反映）
- [ ] シナリオ3: オフライン → オンライン復帰 → 自動再同期
- [ ] シナリオ4: Named Range消失 → 自動復旧
- [ ] シナリオ5: レート制限エラー → Exponential Backoff
- [ ] シナリオ6: トークンリフレッシュ → API呼び出し継続
- [ ] テストカバレッジ: 全要件の90%以上

**技術詳細**:
- ツール: Playwright（Chrome拡張E2Eテスト）
- ファイル: `extension/tests/e2e/*.spec.ts`

#### 12.2 パフォーマンステストの実装

_Requirements: DOCS-NFR-001.1-4_

パフォーマンス目標の達成を検証するテストを実装する。

**受け入れ基準**:
- [ ] 文字起こし受信 → Google Docs挿入完了: 2秒以内（DOCS-NFR-001.1）
- [ ] Google Docs API応答時間: 95パーセンタイルで3秒以内（DOCS-NFR-001.2）
- [ ] オフラインキュー再送信: 100メッセージあたり最大120秒（DOCS-NFR-001.3）
- [ ] ローカルストレージ書き込み: 10ms以内（DOCS-NFR-001.4）
- [ ] 測定ツール: `performance.now()`、Chrome DevTools Performance API

**技術詳細**:
- ファイル: `extension/tests/performance/*.spec.ts`
- 詳細: [design-testing-security.md#Target Metrics](design-modules/design-testing-security.md) L159-165参照

#### 12.3 セキュリティテストの実装

_Requirements: DOCS-NFR-003.1-4_

セキュリティ要件の検証テストを実装する。

**受け入れ基準**:
- [ ] トークンストレージの検証（暗号化なし警告表示確認）
- [ ] HTTPS通信の強制検証
- [ ] Authorization Headerの検証
- [ ] トークン無効化の検証（Google連携解除時）
- [ ] CSP（Content Security Policy）の検証

**技術詳細**:
- ツール: OWASP ZAP、Chrome DevTools Security
- ファイル: `extension/tests/security/*.spec.ts`

### 13. ユーザー設定機能の実装

_Requirements: DOCS-REQ-008.1-5_

#### 13.1 設定画面UIの実装

ユーザーがGoogle Docs同期の動作をカスタマイズできる設定画面を実装する。

**受け入れ基準**:
- [ ] Google Docs同期の有効/無効切り替え
- [ ] タイムスタンプ表示のオン/オフ切り替え
- [ ] 話者名表示のオン/オフ切り替え
- [ ] バッファリング時間の調整（1-5秒）
- [ ] 設定の`chrome.storage.local`への永続化

**技術詳細**:
- ファイル: `extension/popup/settings.html`, `extension/popup/settings.ts`
- ストレージキー: `docs_sync_settings`

#### 13.2 デフォルト設定の適用

**受け入れ基準**:
- [ ] Google Docs同期: 有効（デフォルト）
- [ ] タイムスタンプ表示: 有効（デフォルト）
- [ ] 話者名表示: 無効（デフォルト）
- [ ] バッファリング時間: 3秒（デフォルト）

### 14. ドキュメント作成とリリース準備

#### 14.1 ユーザーマニュアルの作成

エンドユーザー向けのユーザーマニュアルを作成する。

**受け入れ基準**:
- [ ] Google連携手順（スクリーンショット付き）
- [ ] ドキュメントID取得方法
- [ ] トラブルシューティング（認証失敗、同期エラー）
- [ ] FAQ（よくある質問）

**技術詳細**:
- ファイル: `docs/user/google-docs-sync-guide.md`

#### 14.2 開発者ドキュメントの更新

開発者向けドキュメントを更新する。

**受け入れ基準**:
- [ ] API仕様書（AuthManager、SyncManager、GoogleDocsClient）
- [ ] アーキテクチャ図の更新
- [ ] セットアップ手順（Google Cloud Project作成、OAuth 2.0設定）
- [ ] コントリビューションガイド

**技術詳細**:
- ファイル: `docs/dev/google-docs-api-integration.md`

### 15. ユーザー受け入れテスト（UAT）の実施

#### 15.1 UAT実施計画の作成

**受け入れ基準**:
- [ ] テスト参加者: 3名以上
- [ ] テスト期間: 1週間
- [ ] テストシナリオ: 実際の会議での使用
- [ ] フィードバック収集方法（アンケート、インタビュー）

#### 15.2 UAT実施とフィードバック収集

**受け入れ基準**:
- [ ] 全テスト参加者が全シナリオを完了する
- [ ] クリティカルバグ: 0件
- [ ] ユーザー満足度: 80%以上

#### 15.3 バグ修正とパフォーマンス最適化

UAT中に発見されたバグを修正し、パフォーマンスを最適化する。

**受け入れ基準**:
- [ ] 全バグの修正完了
- [ ] パフォーマンス目標の達成確認
- [ ] セキュリティ脆弱性スキャンの実施

### 16. Phase 5検証とリリース準備

#### 16.1 Phase 5検証チェックリストの実行

**受け入れ基準**:
- [ ] 全E2Eテストが成功する
- [ ] ユーザー受け入れテストが完了する
- [ ] パフォーマンス目標を達成する
- [ ] セキュリティテストが成功する
- [ ] ドキュメントが完成する

#### 16.2 本番リリース準備

**受け入れ基準**:
- [ ] リリースノートの作成
- [ ] バージョン番号の決定（Semantic Versioning）
- [ ] Chrome Web Storeへの申請準備
- [ ] ロールバック手順書の最終確認

---

## Success Criteria

本MVP2実装は、以下の条件を全て満たした場合に成功とみなされます（[requirements.md#Success Criteria](requirements.md#L351-361)参照）:

1. ✅ **OAuth 2.0認証**: Chrome拡張からGoogleアカウントにログインし、OAuth 2.0トークンを取得できる
2. ✅ **リアルタイム同期**: 文字起こし結果がリアルタイム（2秒以内）でGoogle Docsに反映される
3. ✅ **Named Range管理**: 文字起こし結果が構造化されたフォーマットでドキュメントに挿入される
4. ✅ **オフライン対応**: ネットワーク切断時もローカルキューに保存され、再接続時に自動同期される
5. ✅ **エラーハンドリング**: トークンリフレッシュ、APIエラー、ネットワークエラーに対して適切に対処する
6. ✅ **ユーザー設定**: Google Docs同期の有効/無効、タイムスタンプ表示等の設定が可能

---

## Rollback Triggers

以下の条件を満たす場合、ロールバックを実行します（[design-migration-appendix.md#Rollback Triggers](design-modules/design-migration-appendix.md#L113-121)参照）:

1. **認証失敗率が50%以上**: OAuth 2.0認証が頻繁に失敗する
2. **API呼び出し成功率が80%未満**: Google Docs API呼び出しが頻繁に失敗する
3. **オフラインキューの保存失敗率が10%以上**: ストレージ書き込みが頻繁に失敗する
4. **クリティカルなセキュリティ脆弱性の発見**: トークン漏洩やXSS攻撃のリスク

---

## Dependencies

### Upstream Dependencies (Blocking)

本specの実装開始前に、以下の成果物が完了している必要があります:

- **meeting-minutes-core** (phase: design-validated以降):
  - **CORE-REQ-006**: WebSocketサーバー (ポート9001-9100)
  - **CORE-REQ-007**: Chrome拡張スケルトン (WebSocket接続機能)
- **meeting-minutes-stt** (phase: implementation-completed):
  - **STT-REQ-008**: WebSocketメッセージ拡張 (confidence, language, isPartial フィールド)

### External Dependencies

- **Google Docs API**: v1
- **Google OAuth 2.0**: Google Identity Services
- **Chrome Extensions API**: Manifest V3
- **Chrome Storage API**: chrome.storage.local

---

## Task Status Tracking

- **Phase 1**: 未着手
- **Phase 2**: 未着手
- **Phase 3**: 未着手
- **Phase 4**: 未着手
- **Phase 5**: 未着手

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-24 | 1.0 | Claude Code | 初版作成（タスク生成） |
