# urthr-net ウォレット接続 初期構築 設計書

- **日付**: 2026-05-30
- **対象**: Anchor を使った Solana Web システムの初期構築。ウォレット接続と、デプロイ済みプログラムの `initialize` 命令呼び出しまでを実現する。
- **実装方針**: 実装フェーズでは `solana-dev` スキルを活用する。

## 1. 目的とスコープ

### ゴール
1. 既存 Anchor プログラム `urthr_net` を **localnet（Surfpool/surfnet）** にデプロイする。
2. Web から **wallet-standard 対応の拡張ウォレット**（Phantom 等）を接続できる。
3. 接続アドレスと SOL 残高を表示する。
4. localnet 向けの **Airdrop ボタン**で接続アドレスに資金を付与できる。
5. Web から `urthr_net` の **`initialize` 命令を実行**し、トランザクション署名を得られる。

### スコープ外（YAGNI）
- 本番（devnet/mainnet）デプロイ。設定上は将来可能だが本タスクでは扱わない。
- `initialize` 以外の命令、追加の state/account 設計。
- 認証・セッション・サーバーサイド機能（SPA のみ）。
- ブラウザ内バーナーウォレット（実機拡張ウォレットを採用）。

## 2. 確定スタック

| レイヤ | 採用技術 |
|---|---|
| オンチェーン | 既存 `urthr_net` Anchor プログラム（`initialize` 命令はそのまま） |
| localnet | Surfpool（surfnet @ `127.0.0.1:8899`）+ `runbooks/deployment` |
| クライアント生成 | Codama で IDL → TypeScript クライアント生成 |
| RPC クライアント | `@solana/client`（@solana/kit 系） |
| ウォレット接続 | wallet-standard + `@solana/react-hooks` |
| フロントエンド | Vite + React + TypeScript（`app/` 配下、独立 pnpm プロジェクト） |

決定理由:
- フロントは軽量・高速な SPA で十分なため Vite + React を採用。
- localnet はリポに既存の surfpool/txtx 資産を活用。
- ウォレットは solana-dev スキル推奨の wallet-standard ファーストを採用し、アダプタ依存を軽くする。

## 3. コンポーネント構成（`app/`）

```
app/
  src/
    rpc.ts                # @solana/client で surfnet RPC クライアントを生成（責務: 接続生成のみ）
    generated/            # Codama 生成クライアント（getInitializeInstruction 等。責務: 命令ビルダのみ）
    wallet/
      WalletProvider.tsx  # wallet-standard プロバイダ
    components/
      ConnectButton.tsx   # 接続 / 切断
      AccountInfo.tsx      # 接続アドレス + SOL 残高表示（ポーリング）
      AirdropButton.tsx    # localnet 向け: 接続アドレスへ airdrop
      InitializeButton.tsx # initialize 命令を構築 → wallet 署名 → 送信 → 署名表示
    App.tsx               # 上記コンポーネントを配置
  .env                    # VITE_RPC_URL, VITE_PROGRAM_ID
  vite.config.ts
  index.html
  package.json
  tsconfig.json
```

各ユニットは単一責務を持ち、独立して理解・テスト可能とする。

## 4. データフロー

```
拡張ウォレット → wallet-standard → react-hooks(account, signer)
  → getInitializeInstruction()（generated）
  → wallet で署名 → @solana/client RPC で送信 → surfnet
  → 確認 → UI に署名(signature)を表示
```

- 残高表示: `rpc.getBalance(address)` をポーリングして更新。
- Airdrop: localnet RPC の `requestAirdrop` を呼び出し、接続アドレスへ付与。
- `initialize` 命令は `Initialize {}`（アカウント不要）であり、ウォレットが fee payer 兼署名者となる単純なトランザクション。

## 5. ビルド & デプロイ手順（README 化する）

1. `anchor build` で IDL（`target/idl/urthr_net.json`）を生成。`anchor keys sync` でプログラム ID を整合させる。
2. `surfpool start` で surfnet を起動し、別シェルで `surfpool run deployment` を実行してプログラムをデプロイ（`runbooks/deployment` を使用）。
3. IDL から Codama でクライアントを生成し `app/src/generated/` に出力。
4. `pnpm --dir app dev` で Vite を起動。
5. ブラウザで Phantom 等の拡張を localhost RPC（`http://127.0.0.1:8899`）に切替（手順を README に記載）→ 接続 → Airdrop → `initialize` 実行。

## 6. エラーハンドリング

| 状況 | 対応 |
|---|---|
| ウォレット未インストール / 未接続 | 接続を促す表示 |
| RPC 不一致（拡張が localhost 以外） | 注意喚起と手順リンクを表示 |
| 残高不足 | Airdrop ボタンへ誘導 |
| トランザクション失敗 | エラー内容を UI に表示 |

## 7. 検証

- **プログラム**: 既存 `programs/urthr-net/tests/test_initialize.rs`（`cargo test`）を維持し、グリーンを確認。
- **E2E（手動）**: surfnet 起動 → デプロイ → Web 接続 → Airdrop → `initialize` 実行で、トランザクション署名が返ることを確認。

## 8. 実装上の留意点

- **Program ID 整合**: `programs/urthr-net/src/lib.rs` の `declare_id!`、`Anchor.toml`、`target/deploy/*-keypair.json` を `anchor keys sync` で一致させる。
- **パッケージマネージャ**: ワークスペースは yarn 設定だが、フロントは `app/` に独立した pnpm プロジェクトとして分離し、依存の混在を避ける。
- **solana-dev スキル活用**: 正確なパッケージ名、wallet-standard / `@solana/react-hooks` / Codama の手順は実装時に solana-dev スキルで確認する。
- **既存資産の尊重**: surfpool/txtx の runbook 構成・命名に従う。不要なリファクタリングは行わない。
