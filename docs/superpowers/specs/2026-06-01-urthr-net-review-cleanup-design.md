# UrthrNet レビュー＆クリーンアップ 設計書

- 日付: 2026-06-01
- 対象: 既存のオンチェーンプログラム（`programs/urthr-net`）、フロントエンド（`app/`）、現役ドキュメント
- 目的: (A) 現役ドキュメントを日本語へ統一し、(B) プログラムを solana-dev のベストプラクティスに適合させる

## 背景

これまでに次が実装済み:

- オンチェーンプログラム `urthr_net`（Anchor、11 命令、LiteSVM テスト一式）
- React 製の localnet デバッグハーネス（`app/src/debug/`）
- ドキュメント群（`README` / `ARCHITECTURE` / runbook / `features` / specs / plans）

ドキュメントの言語が混在しており（純英語: `ARCHITECTURE.md`・runbooks・root `README`／日本語主体: `features.md`・specs／混在: debug-harness runbook・app `README`）、また solana-dev 基準でのプログラム監査が未実施。本作業でこの2点を解消する。

## スコープ

- **Track A — ドキュメント日本語化**: 現役ドキュメントのみ。`docs/superpowers/specs` および `plans` は履歴成果物として英語のまま据え置く。
- **Track B — プログラムベストプラクティス対応**: Rust（オンチェーン）と `app/`（クライアント）の両レイヤーを solana-dev 基準で監査し、承認の上で修正する。

### スコープ外

- `docs/superpowers/specs` / `plans` の翻訳
- 生成コード（`app/src/generated`）の翻訳・改変
- `runbooks/*.tx`（txtx 設定）の翻訳
- mainnet 対応（既定 cluster は localnet/devnet のまま）
- 関連性のないリファクタリング（現作業の目的に資するもののみ）

## 進め方（順序）

プログラム修正は `ARCHITECTURE.md` / runbook / `features.md` に波及するため、**監査・修正を先に確定し、その最終状態を一度だけ日本語化する**（ドキュメントの二度手間を避ける）。

1. Track B 監査レポート作成・コミット → **承認ゲート**
2. 承認された修正を適用（テスト緑・パネル更新・必要なら codegen）
3. 確定した現役ドキュメントを日本語化＋陳腐化箇所を訂正
4. 最終検証

## Track B — プログラム監査

### 監査観点（solana-dev `security.md` 準拠）

**オンチェーン（Rust / Anchor）:**

- 署名者チェック（各命令で誰が署名すべきか）
- 所有者・アカウント型検証
- PDA の seeds/bump（canonical bump）検証、`has_one` / address 制約
- 検算（checked 128-bit math）の網羅、オーバーフロー時のエラー処理
- CPI 安全性（token transfer の signer seeds、`token::mint` 検証）
- アカウントクローズ（`close_campaign` の rent 返却・復活防止）／再初期化の誤用
- SPL Token vs Token-2022 の前提、`paused` 緊急停止の網羅範囲
- エラー型の網羅・重複可変アカウント混同の有無

**app（kit / フロントエンド）:**

- kit バージョン乖離（`@solana/client` kit 5.5.1 vs app 6.9.0）の共有ヘルパ経由徹底
  （`runnerPrepareOptions` / `stringifyWithBigInt` / `describeTransactionError`）
- 送信前シミュレーション不変条件（`DebugPanel` / `useInstructionRunner`）の全パネル遵守
- 生成クライアントと IDL の一致
- 秘密鍵非保持、既定 cluster = localnet/devnet

### 成果物: 監査レポート

`docs/` 配下に Markdown で作成。指摘を表形式で記載する:

| ID | レイヤー | file:line | 重要度 | 内容 | 推奨修正 |
|---|---|---|---|---|---|

- 重要度: 高（correctness/security に直結） / 中 / 低（スタイル・可読性）
- このレポートを提示して**承認ゲート**。ユーザが修正対象を選定する。

### 修正フロー（承認後）

- 各オンチェーン修正は `cargo test -p urthr-net`（LiteSVM）を緑に保つ。
- CLAUDE.md の Definition of Done に従い、命令を変更した場合は該当デバッグパネル
  （`app/src/debug/instructions/`）と必要に応じてアカウントインスペクタを更新。
- IDL を変更した場合は `cd app && pnpm codegen` でクライアント再生成。

## Track A — ドキュメント日本語化

### 対象（現役ドキュメントのみ）

- `README.md`
- `app/README.md`
- `docs/ARCHITECTURE.md`
- `docs/debug-harness-runbook.md`
- `features.md`
- `runbooks/README.md`

`CLAUDE.md` は既に日本語、`features.md` もほぼ日本語のため、残りの英語・混在箇所を仕上げる。実質的な翻訳量は `ARCHITECTURE.md`・`runbooks/README.md`・root `README.md` が中心。

### 翻訳規約

- 散文は日本語化。**コード識別子・ファイルパス・コマンド・型名・命令名・エラー名は英語/monospace のまま**保持する。
- 用語集を統一する（暫定）:
  - publisher = パブリッシャー
  - advertiser = 広告主
  - stake = ステーク（担保）
  - slash = スラッシュ
  - claim = クレーム
  - attestor = アテスター
  - challenge = チャレンジ
  - settle = 決済
  - escrow = エスクロー
  - treasury = トレジャリ
  - vault = vault
- 翻訳のついでに**陳腐化した記述を訂正**する。既知: `ARCHITECTURE.md:138` 付近の
  `--ignore-keys` 推奨は、CLAUDE.md の鍵統一（`8CsD…` へ統一済み）により不要 → 削除/修正。
  他にも翻訳中に見つけた陳腐化箇所は同様に訂正。

## 最終検証

- オンチェーン: `NO_DNA=1 anchor build` → `cargo test -p urthr-net`
- フロントエンド: `cd app && pnpm build && pnpm test && pnpm lint`
- ドキュメント: 対象ファイルがすべて日本語に統一され、コード識別子・コマンドが破壊
  されていないこと、陳腐化箇所が訂正されていることを目視確認。

## 成功基準

- 現役ドキュメントが日本語で統一され、用語が一貫している。
- 監査レポートが提示・承認され、承認された指摘がすべて修正済み。
- 全テスト（`cargo test -p urthr-net`、app の test/lint）が緑、ビルド成功。
- オンチェーン命令を変更した場合、対応するデバッグパネルが更新済み。
