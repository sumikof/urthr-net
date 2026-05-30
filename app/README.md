# urthr-net Web (wallet connect)

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
3. Webで「接続」→「2 SOL Airdrop」→「initialize を実行」

## 注意
- 残高不足の場合は Airdrop ボタンで付与する。
- 拡張のRPCがlocalhost以外だと残高0や送信失敗になる。
