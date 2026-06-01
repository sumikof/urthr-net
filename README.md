# Project UrthrNet

Solana ベースの**分散型広告ネットワーク**。広告不正を「skin in the game」モデルで抑止します。パブリッシャーは広告スロットを掲載する際に担保をステークし、不正トラフィックが検出されるとそのステークがスラッシュされ、広告主に支払われます。プロトコルは小さな手数料のみで動作し、中間業者のマークアップはありません。

> **Status:** サブプロジェクト #1 — オンチェーンプロトコルコア — は実装済みであり、`app/` はブラウザからすべての命令を操作できる **localnet デバッグハーネス** です。オフチェーンサービスとダッシュボードは計画中です。[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) および [`features.md`](features.md) を参照してください。

## 仕組み

広告主はキャンペーンごとに予算をエスクローします。パブリッシャーは担保をステークし、パフォーマンス証明の**クレーム**をバッチで送信します。各クレームは**チャレンジウィンドウ**を開きます。不正の証拠があれば誰でもチャレンジできます。設定可能な**アテスター**がチャレンジを解決します — 有効なクレームは**決済**（パブリッシャーへ `amount − fee`、手数料はトレジャリへ）、不正なクレームはパブリッシャーのステークを広告主のエスクローに**スラッシュ**します。チャレンジされなかったクレームはウィンドウ経過後にパーミッションレスで決済されます。

完全な設計書: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) ·
仕様: [`docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md`](docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md)

## 技術スタック

- **Program:** Anchor 1.0.2 (Rust)、プログラム `urthr_net`
  (`8CsDf7B1YU9HV136afSbYsY8eV2YeJUk5Sd2CpuLLiSb`)
- **Payments:** SPL Token、設定可能な単一 mint（USDC ライク）
- **Tests:** LiteSVM（Rust、インプロセス、実 SPL Token プログラム）+ Surfpool インテグレーションスモークテスト
- **Frontend:** Vite + React + `@solana/client` / `@solana/react-hooks` — [`app/`](app/) の localnet **デバッグハーネス**（命令ごとにパネル、シミュレーション後に送信）

## 使い方 — localnet デバッグハーネス

プロトコル全体をエンドツーエンドで試す最速の方法は、[`app/`](app/) のブラウザデバッグハーネスです。**命令ごとに 1 パネル（全 12 件）**、4 つの**アカウントインスペクタ**、**localnet mint ヘルパー**を備えています。すべてのパネルは**トランザクションを先にシミュレーションして結果を表示し、明示的に承認した場合のみ送信**します。

```bash
# 1. localnet を起動（別シェル）
NO_DNA=1 surfpool start                     # RPC :8899 / WS :8900

# 2. （IDL が変更された場合のみ）型付きクライアントを再生成
cd app && pnpm codegen

# 3. 開発サーバを起動
cd app && pnpm dev                          # http://localhost:5173 を開く
```

ページ内で（ウォレットを localnet RPC `http://127.0.0.1:8899` に向ける）:

1. **ウォレット**: connect.
2. **localnet**: Airdrop SOL → **CreateMint**（`payment_mint` を生成）→ **MintTo**（自分に資金を付与）.
3. **プロトコル設定**: `initialize_protocol`（attestor = 自分のアドレス、作成した mint）→ **ProtocolConfig** インスペクタで確認。`initialize_protocol` はプログラムのアップグレード権限（デプロイヤー）のみ実行可能です。
4. **パブリッシャー**: `register_publisher` → `stake`.
5. **キャンペーン**: `create_campaign`（price > 0）→ `fund_campaign`.
6. **Claim**: `submit_claim` → チャレンジウィンドウ経過後に `settle_claim`、または `challenge_claim` → `resolve_claim(fraud=true)` でスラッシュを確認。

完全なウォークスルー: [`app/README.md`](app/README.md). このハーネスは**開発上の契約**でもあります。オンチェーン命令に触れる機能は、対応するデバッグパネルとともに出荷する必要があります — [`CLAUDE.md`](CLAUDE.md) を参照してください。

## ビルド & テスト

```bash
# プログラムをビルド（target/deploy/urthr_net.so + IDL を生成）
NO_DNA=1 anchor build

# 正式なテストスイートを実行（31 件の LiteSVM テスト、スラッシュ・set_paused・アップグレード権限制限済み init を含む完全ライフサイクル）
cargo test -p urthr-net

# オプション: surfnet インテグレーションスモークテスト（事前に surfnet を起動）
NO_DNA=1 surfpool start      # 別シェル
pnpm test:integration
```

## レイアウト

| パス | 内容 |
|---|---|
| `programs/urthr-net/` | `urthr_net` Anchor プログラム（状態、12 命令）+ LiteSVM テスト |
| `tests/lifecycle.mjs` | surfnet インテグレーションスモークテスト |
| `app/` | localnet **デバッグハーネス** フロントエンド（命令ごとにパネル）— [`app/README.md`](app/README.md) 参照 |
| `CLAUDE.md` | 開発ルール（必須デバッグパネルルールを含む） |
| `features.md` | 今後構築する機能のバックログ |
| `runbooks/` | Surfpool / txtx デプロイ Runbook |
| `docs/ARCHITECTURE.md` | システムアーキテクチャ & ロードマップ |

## ロードマップ

1. ✅ プロトコルコア（オンチェーン）— このマイルストーン
2. ⬜ オフチェーンサービス（reporter / challenger / attestor / indexer）
3. ⬜ 広告主 & パブリッシャーダッシュボード（`app/` の拡張）
