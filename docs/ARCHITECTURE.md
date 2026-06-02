# UrthrNet アーキテクチャ

## ビジョン

UrthrNetは、Solanaベースの分散型広告ネットワークである。既存の広告プラットフォームに代わる仕組みであり、わずかなプロトコル手数料で自律的に動作する。中心的な課題は**広告不正**（ボットクリック／偽コンバージョン）であり、**「skin in the game」モデル**で解決する。パブリッシャーは広告スロットを登録するためにステーク（担保）を積む必要があり、不正なトラフィックが検出されると、そのステーク（担保）が**スラッシュ**されて広告主への補償に充てられる。

## システムマップ

```
┌──────────────────────────────────────────────────────────────┐
│ オンチェーンプログラム `urthr_net`  (このリポジトリ — 実装済み)    │
│   ① Protocol Config / Treasury                                │
│   ② Publisher Registry + Stake vault                          │
│   ③ Campaign Escrow                                           │
│   ④ Attribution & Settlement (Claim チャレンジ/スラッシュ/決済)  │
└──────────────────────────────────────────────────────────────┘
        ▲ IDL / codegen クライアント      ▲ 署名済みトランザクション
┌────────────────────────────┐   ┌────────────────────────────────┐
│ オフチェーンサービス (#2)     │   │ フロントエンドダッシュボード (#3)   │
│  • Reporter (集計 +          │   │  • 広告主ダッシュボード            │
│    クレーム送信)              │   │  • パブリッシャーダッシュボード    │
│  • Challenger (ボット検知)   │   │  (既存 `app/` を拡張)            │
│  • アテスター (解決)          │   │                                  │
│  • Indexer (状態読み取り)    │   │                                  │
└────────────────────────────┘   └────────────────────────────────┘
```

このマイルストーンで構築するのはオンチェーンプログラムのみ（サブプロジェクト #1）。オフチェーンサービス（#2）とダッシュボード（#3）は計画中 — ロードマップを参照。

## 信頼モデル

オフチェーンの不正判定は、`ProtocolConfig` 内の設定可能な単一の**`attestor`**公開鍵として抽象化されている。オンチェーンのステートマシンは評決の*到達方法*に依存しないため、MVPより後に、アテスター役を投票委員会やディスピュートゲームに**プロトコルコアを変更せずに**置き換えることができる。MVPでは、単一の信頼されたアテスターがチャレンジを解決する。

## オンチェーンアカウント（PDA）

| アカウント | シード | 主要フィールド |
|---|---|---|
| **ProtocolConfig** | `["config"]` | `admin`, `attestor`, `payment_mint`, `treasury`, `protocol_fee_bps:u16`, `min_publisher_stake:u64`, `challenge_window:u64`, `paused:bool`, `bump` |
| **Publisher** | `["publisher", authority]` | `authority`, `stake_vault`, `staked_amount:u64`, `locked_amount:u64`, `metadata:[u8;32]`, `bump` |
| **Campaign** | `["campaign", advertiser, campaign_id]` | `advertiser`, `escrow_vault`, `campaign_id:u64`, `price_per_event:u64`, `budget_remaining:u64`, `locked_budget:u64`, `claims_count:u64`, `status`, `bump` |
| **Claim** | `["claim", campaign, claim_nonce]` | `campaign`, `publisher`, `claim_nonce:u64`, `event_count:u64`, `amount:u64`, `merkle_root:[u8;32]`, `evidence_hash:[u8;32]`, `challenger:Option<Pubkey>`, `challenge_deadline:i64`, `status`, `bump` |

資金は**PDA所有のSPLトークンvault**に保管される: `treasury`（authority = config PDA）、`stake_vault`（authority = publisher PDA）、`escrow_vault`（authority = campaign PDA）。所有するステートPDAがシードを用いてCPI転送に署名する。

**単一 payment mint。** 広告主の予算とパブリッシャーの担保は、どちらも1つの設定可能なSPL mint（本番ではUSDC相当）を使用する。スラッシュされたステーク（担保）が同一通貨で広告主を補償するため、価格オラクルが不要。`claim_nonce`（キャンペーン単位の単調増加カウンタ、`Campaign.claims_count`）が各クレームPDAの鍵となり、クレームが自己識別できる形で保存される。

## クレームライフサイクル

```
submit_claim (パブリッシャーが署名 — skin in the game)
  ├─ require campaign Active, event_count > 0
  ├─ amount = event_count * price_per_event
  ├─ require budget_remaining >= amount  → locked_budget へ移動
  ├─ require staked - locked >= amount    → publisher.locked_amount へ移動
  └─ challenge_deadline = now + challenge_window;  status = Pending
       │
   ┌───┴───────────────────────────┬──────────────────────────────────┐
   │ challenge_claim (誰でも可、    │ ウィンドウ経過、チャレンジなし       │
   │ ウィンドウ内): evidence_hash   │ settle_claim (permissionless)      │
   ▼  を記録; status = Challenged   ▼   パブリッシャーへ amount−fee を支払  │
   resolve_claim (ATTESTOR のみ):       fee → treasury、ロック解除、       │
   ├─ valid → settle: パブリッシャー     status = Settled                  │
   │   へ amount−fee を支払、
   │   fee → treasury、ロック解除、status = Settled
   └─ fraud → slash: stake_vault の `amount` を escrow_vault へ移動
       （パブリッシャーから広告主への補償）; budget_remaining += amount
       （アンロック）+ amount（補償）; staked_amount −= amount;
       ロック解除; status = Slashed
```

**ロックは純粋な会計処理。** submit 時点でトークンは移動しない — `amount` が `budget_remaining` から `locked_budget` に振り替えられ、対応するステーク（担保）が `locked_amount` にマークされる。`amount` トークンは物理的に `escrow_vault` に留まる。トークが実際に移動するのは **settle**（エスクロー → パブリッシャー/トレジャリ）または **slash**（ステーク → エスクロー）のときのみ。これにより不変条件 `escrow_vault残高 == budget_remaining + locked_budget` が休止状態で保たれる。

## トークン＆経済モデル

- **プロトコル手数料** = `amount * protocol_fee_bps / 10_000`（128ビット検証済み演算; `fee ≤ amount`）。
- **決済**: パブリッシャーが `amount − fee` を受け取り、`fee` がトレジャリに送られる。広告主の予算が消費され（トークンがエスクローから出る）、パブリッシャーのステーク（担保）ロックが解除される。
- **スラッシュ**: 広告主は**支払わない** — ロックされた予算は `budget_remaining` に戻り、**加えて**パブリッシャーのスラッシュされたステーク（担保）（`amount`）が補償として上乗せされる。パブリッシャーの `staked_amount` は `amount` 分減少する。
- すべての残高演算は検証済みであり、オーバーフロー時は命令がエラーを返す（`MathOverflow`）。

## Instructions (12)

| # | Instruction | Signer | Role |
|---|---|---|---|
| 1 | `initialize_protocol` | admin（アップグレード権限者） | Config + treasury vaultを作成; fee bpsを検証。`program` + `program_data` アカウントを受け取り、アップグレード権限者のみが実行可能（permissionless initのフロントラン攻撃を防止） |
| 2 | `set_paused` | admin | 緊急停止フラグの切替（`config.paused` をセット）。admin のみ実行可能 |
| 3 | `register_publisher` | publisher | Publisher + stake vaultを作成 |
| 4 | `stake` | publisher | 担保を預け入れる |
| 5 | `unstake` | publisher | 担保を引き出す（≥ locked; フル退出または ≥ 最小値） |
| 6 | `create_campaign` | advertiser | Campaign + escrow vaultを作成; `price_per_event > 0` |
| 7 | `fund_campaign` | advertiser | 予算を入金する |
| 8 | `submit_claim` | publisher | バッチクレーム; 予算とステーク（担保）をロック; チャレンジウィンドウを開く |
| 9 | `challenge_claim` | 誰でも | 証拠ハッシュで不正にフラグを立てる（ウィンドウ内） |
| 10 | `resolve_claim` | **attestor** | チャレンジされたクレームを判定: 決済またはスラッシュ |
| 11 | `settle_claim` | 誰でも | ウィンドウ経過後にチャレンジのないクレームを支払う |
| 12 | `close_campaign` | advertiser | 残予算を advertiser に返却し、`escrow_vault` を閉じて rent を返却、`status = Closed` に（campaign アカウントは claim 参照のため Closed tombstone として保持） |

### セキュリティ制約（多層防御）

- vault アカウントは `has_one`（config↔treasury、campaign↔escrow_vault、publisher↔stake_vault）でバインドされており、トークン移動を伴う命令では明示的なPDA `seeds`/`bump` と `token::mint` 制約による検証が行われる。さらに `escrow_vault` と `stake_vault` には `token::mint` 制約が付加されており、多層防御として正しい mint との紐付けを強制する。
- 単一mint規則は、資金移動命令に対する `has_one = payment_mint` で適用される。
- 資金移動命令は `!config.paused`（緊急停止）を確認する。`unstake` と `close_campaign` はポーズ中も意図的に利用可能であり、ユーザー自身の資金を返す（退出／引き出し）。緊急停止フラグは `set_paused` 命令（admin のみ）で切り替えることができる。
- `merkle_root` はMVPでは保存されるが検証されない — 将来のイベントごとの証明のためのフック。

## テスト

- **ユニット／統合テスト（権威あるゲート）: LiteSVM、Rust** — `programs/urthr-net/tests/`。
  33テストが*実際の* SPL Tokenプログラムをインプロセスで実行し、すべての命令と完全なライフサイクルをカバーする: fund→stake→submit→settle、submit→challenge→resolve（スラッシュ）、submit→settle→close。`set_paused`・アップグレード権限者限定初期化・自己チャレンジ拒否・unstake 残高超過のテストも含まれる。共有ハーネス（`tests/common/mod.rs`）がmint／トークンアカウントをバイトパックし、PDAを導出し、マルチバージョンの `solana-pubkey` クレートグラフをブリッジする。
- **Surfnetスモークテスト:** `tests/lifecycle.mjs`（`pnpm test:integration`）は実行中のsurfnet＋プログラムデプロイが存在する場合にチェックし、存在しない場合はクリーンにスキップする。

実行: `NO_DNA=1 anchor build` の後に `cargo test -p urthr-net` を実行する。

## リポジトリレイアウト

```
programs/urthr-net/src/
  lib.rs                 # #[program] エントリポイント、declare_id
  constants.rs           # シード、FEE_DENOMINATOR
  error.rs               # UrthrError
  util.rs                # fee_amount
  state/                 # ProtocolConfig, Publisher, Campaign, Claim
  instructions/          # 命令ごとに1ファイル (12)
programs/urthr-net/tests/
  common/mod.rs          # LiteSVM ハーネス
  *.rs                   # ドメインごとのテストスイート
tests/lifecycle.mjs      # surfnet 統合スモークテスト
app/                     # ウォレット接続フロントエンド (#3 で拡張予定)
runbooks/                # txtx / Surfpool デプロイメント
docs/
  ARCHITECTURE.md        # このファイル
  superpowers/{specs,plans}/
```

`services/`（オフチェーン #2）と完全なcodegenの `clients/`（#3）はまだ作成されていない — 対応するサブプロジェクト開始時に追加される。

## ロードマップ

1. ✅ **プロトコルコア**（オンチェーン `urthr_net`）— このマイルストーン。
2. ⬜ **オフチェーンサービス** — Reporter（集計＋クレーム送信）、Challenger（ボット検知＋チャレンジ送信）、アテスター（解決）、Indexer（ダッシュボード用の状態読み取り）。
3. ⬜ **フロントエンドダッシュボード** — 広告主向け（キャンペーン作成/入金/モニタリング）とパブリッシャー向け（登録/ステーク/収益）、`app/` を拡張。

将来のプロトコル強化（ポストMVP）: イベントごとのmerkle証明チャレンジ、単一アテスターを置き換える分散判定（投票／ディスピュートゲーム）、マルチmintサポート、Token-2022／confidential transfers。
