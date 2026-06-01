# urthr-net Web — デバッグハーネス

## 前提
- Surfpool 1.2.1 / Node 24 / pnpm
- ルートで `anchor build` 済み（`target/idl/urthr_net.json` が存在）

## 起動手順
1. localnet（surfnet）を起動（リポジトリルートで）:
   ```bash
   NO_DNA=1 surfpool start
   ```
   RPC: http://127.0.0.1:8899 / WS: ws://127.0.0.1:8900
2. クライアント生成（IDL変更時のみ）:
   ```bash
   cd app && pnpm codegen
   ```
3. 開発サーバ起動:
   ```bash
   cd app && pnpm dev
   ```
   http://localhost:5173 を開く。

## Phantom を localnet に向ける
1. Phantom → 設定 → Developer Settings → Change Network → Custom RPC
2. RPC URL に `http://127.0.0.1:8899` を設定（Cluster: Custom/Localnet）

## デバッグハーネスの使い方（localnet 全機能フロー）

この画面は UrthrNet の全11命令を叩け、4アカウントを覗ける localnet デバッグ画面です。
各命令パネルは **「シミュレート」→ 要約確認 → 「承認して送信」** の順で実行します（送信前シミュレーション必須）。

代表的な一連フロー:
1. **ウォレット**: 「接続」。
2. **localnet**: 「2 SOL Airdrop」→「mint鍵を生成」して `payment_mint` を作成（CreateMint）→「MintTo」で自分にトークンを付与。
3. **プロトコル設定**: `initialize_protocol`（attestor=自分のアドレス、payment_mint=作成した mint）。`ProtocolConfig` インスペクタで確認。
4. **パブリッシャー**: `register_publisher` → `stake`（payment_mint=同じ mint）。`Publisher` インスペクタで確認。
5. **キャンペーン**: `create_campaign`（price_per_event > 0）→ `fund_campaign`。`Campaign` インスペクタで確認。
6. **Claim**: `submit_claim`（advertiser=自分、campaign_id 指定）→ `challenge_window` 経過後に `settle_claim`。
   不正系は `challenge_claim` → `resolve_claim(fraud=true)` でスラッシュを確認。`Claim` インスペクタで状態遷移を確認。

新しいオンチェーン命令を追加する際は、対応するデバッグパネルをここに追加すること（[`../CLAUDE.md`](../CLAUDE.md) のデバッグパネル必須ルール）。

## 注意
- 残高不足の場合は Airdrop ボタンで付与する。
- 拡張のRPCがlocalhost以外だと残高0や送信失敗になる。
- `payment_mint` は CreateMint で作った mint アドレスを各パネルに貼り付けて使う。
- 命令の正当性の最終ゲートは Rust LiteSVM スイート（`cargo test -p urthr-net`）。この画面は手動結合確認用。
