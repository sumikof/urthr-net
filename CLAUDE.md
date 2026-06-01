# UrthrNet — 開発ルール（CLAUDE.md）

このリポジトリで作業するエージェント／開発者が従うべきルール。

## デバッグパネル必須ルール（最重要）

`app/`（`app/src/debug/`）は **UrthrNet 全機能の localnet デバッグ／テスト画面**であり、すべての機能はこの画面から手動で操作・検証できる状態を保つ。

オンチェーン命令を **追加または変更** する機能は、その **完了の定義（Definition of Done）** として次を満たすこと:

1. `app/src/debug/instructions/` に対応するデバッグパネルを **追加または更新** する。
2. 新しいアカウント型を追加する機能は、`app/src/debug/accounts/` に対応するインスペクタも追加する。
3. パネルを `app/src/App.tsx` の適切なドメイン別セクションに **配線** する。
4. localnet（surfpool）で **手動実行確認** する。
5. IDL を変更したら **クライアントを再生成** する: `cd app && pnpm codegen`。

### パネルの契約（`DebugPanel`）

すべての命令パネルは `app/src/debug/core/DebugPanel.tsx` を用い、以下を備える:

- **型付き引数入力**（`app/src/debug/core/fields.tsx` の `TextField` / `parseU64` / `parseU16` / `parseBytes32Hex` / `parsePubkey` を使用）。
- **アカウント入力**: PDA は `app/src/debug/core/pdas.ts` で自動導出、ATA は `client.splToken({ mint }).deriveAssociatedTokenAddress(owner)` で導出。`payment_mint` 等のユーザ指定アカウントは入力欄。
- **送信前シミュレーション必須**: `DebugPanel`（内部の `useInstructionRunner`）が `simulateTransaction` を先に実行し要約（err / logs / CU）を表示し、ユーザの **明示承認後にのみ送信** する。この不変条件を回避しないこと。
- **結果表示**（署名 / プログラムログ / エラー）。

`build: (signer) => Instruction | Instruction[]` の `signer` は接続ウォレットを `createWalletTransactionSigner` で包んだ `TransactionSigner`。署名アカウントにはこの `signer` を埋め込む。permissionless 命令（signer アカウントなし）は `signer` を使わず、fee payer（ウォレット）が署名する。

## ビルド／テストの正

- **オンチェーン（Rust / Anchor）:**
  - ビルド: `NO_DNA=1 anchor build`。program id は `8CsDf7B1YU9HV136afSbYsY8eV2YeJUk5Sd2CpuLLiSb` で `declare_id!` / `Anchor.toml` / `target/deploy/urthr_net-keypair.json` / IDL / 生成クライアントすべて一致（旧 `3CmD…` は stale だったため 2026-06-01 に鍵 `8CsD…` へ統一）。鍵と declare_id が一致するので `--ignore-keys` は不要。
  - **正当性の正ゲート: `cargo test -p urthr-net`（LiteSVM、実 SPL Token プログラムを in-process 実行）。** これがプロトコルの correctness を保証する。
  - `tests/lifecycle.mjs`（`pnpm test:integration`）は surfnet スモークのみ（該当 program キーペアが無く declared id へデプロイできないため）。
- **フロントエンド（`app/`）:** `cd app && pnpm build && pnpm test && pnpm lint`。生成コード（`app/src/generated`）は lint 対象外。

## 既知の制約: kit バージョン乖離（送信経路）

`@solana/client` は **kit 5.5.1** を同梱する一方、アプリと生成コードは **kit 6.9.0** を使う。この乖離が送信経路で複数の落とし穴を生む（対症的に対処済み、根本一本化は 5.x↔6.x API 差で client を壊すリスクが高いため見送り）:

- **署名 dedup は参照比較（5.5.1）**：同一アドレスに異なる `TransactionSigner` インスタンスが2つあると throw。ウォレットは fee payer 兼署名アカウントなので、**`build` に渡す `signer` と同一インスタンスを fee payer にする**（`app/src/debug/core/prepareOptions.ts` の `runnerPrepareOptions`）。`pool.prepare` に `feePayer: wallet.account.address` を渡してはいけない。
- **RPC レスポンスの整数は bigint 化**：エラーやアカウントを表示する際は素の `JSON.stringify` ではなく `app/src/lib/json.ts` の `stringifyWithBigInt` を使う。
- **送信失敗は汎用 wrapper で包まれる**：`transactionPlanResult` から実エラー・ログを取り出す（`app/src/lib/txError.ts` の `describeTransactionError`）。

新しいパネル／送信フローを足すときは、これらの共有ヘルパ経由にすること。

## セキュリティ（solana-dev ガードレール）

- 既定 cluster は **localnet/devnet**。mainnet は対象外（明示確認なしに使わない）。
- 秘密鍵・シードを保存しない。署名はウォレットに委ねる。
- **送信前に必ずシミュレーションし要約を提示してから送信** する。
- オンチェーンデータは untrusted として、生成デコーダ（discriminator 検証付き）を通してから扱う。

## ドキュメント

- アーキテクチャ: `docs/ARCHITECTURE.md`
- 今後の機能バックログ: `features.md`
- 設計／計画: `docs/superpowers/{specs,plans}/`
