# Project UrthrNet — サブプロジェクト #1: Protocol Core (MVP) 設計書

**作成日:** 2026-05-31
**対象:** オンチェーン Protocol Core の初期実装＋リポジトリ/アーキテクチャ整備
**ブロックチェーン:** Solana / **フレームワーク:** Anchor 1.0.2
**Program ID:** `3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR`（既存を維持）

---

## 1. 背景とゴール

Project UrthrNet は、巨大プラットフォームに代わる Solana ベースの分散型広告ネットワーク。
中心テーマは Web3 広告のアドフラウド（Bot 等によるクリック詐欺）の解決で、掲載側（パブリッシャー）に
担保リスクを負わせる **"Skin in the game"** で解決する。

本サブプロジェクト #1 のゴールは、その**オンチェーン中核（escrow / stake / challenge / slash / settle の
状態機械）の初期実装**と、**リポジトリ構成・アーキテクチャドキュメントの整備**。
オフチェーンサービス（#2）とフロントエンド（#3）は後続サブプロジェクトとして本書のロードマップに記載する。

### 確定した設計判断

| 論点 | 決定 |
|---|---|
| 裁定モデル | **単一の信頼 Attestor**（Protocol Config 内の差し替え可能ロール）。将来、投票/ディスピュートゲームへ拡張可能な抽象化 |
| トークン設計 | **単一の許可 mint**（予算・担保とも同一、MVP は USDC 想定）。スラッシュ担保を同一通貨で広告主に補填 |
| イベント記録粒度 | **バッチ集計 Claim**（`(campaign × publisher)` 件数を1 Claim に集約）。`merkle_root` は保持のみ（検証は将来） |
| プログラム分割 | **単一 Anchor プログラム** `urthr_net`。ドメインは instruction モジュール＋PDA seed で分離 |
| 決済トークン規格 | SPL Token（Token-2022 は対象外） |

---

## 2. アーキテクチャ全体像

```
┌─────────────────────────────────────────────────────────────┐
│ オンチェーン (Anchor program: urthr_net)  ← 本サブプロジェクト#1 │
│  ① Protocol Config / Treasury                                │
│  ② Publisher Registry + Stake                                │
│  ③ Campaign Escrow                                           │
│  ④ Attribution & Settlement (Claim 状態機械)                  │
└─────────────────────────────────────────────────────────────┘
            ▲ IDL / codegen client          ▲ 署名トランザクション
┌───────────────────────────┐   ┌───────────────────────────────┐
│ オフチェーン (#2 後続)       │   │ フロントエンド (#3 後続)         │
│  ・Reporter(集約・投稿)     │   │  ・広告主ダッシュボード          │
│  ・Fraud検知/Challenger     │   │  ・パブリッシャーダッシュボード    │
│  ・Attestor(裁定権限)       │   │  (既存 app/ を拡張)            │
│  ・Indexer / 読み取りAPI    │   │                                │
└───────────────────────────┘   └───────────────────────────────┘
```

---

## 3. リポジトリ構成

```
urthr-net/
├── programs/urthr-net/src/
│   ├── lib.rs                 # #[program] エントリ・declare_id
│   ├── constants.rs           # seed・手数料分母(10_000)など
│   ├── error.rs               # UrthrError（ドメイン別エラー）
│   ├── state/                 # 現状の単一 state.rs を分割
│   │   ├── mod.rs
│   │   ├── protocol_config.rs
│   │   ├── publisher.rs
│   │   ├── campaign.rs
│   │   └── claim.rs
│   ├── instructions/
│   │   ├── mod.rs
│   │   ├── initialize_protocol.rs
│   │   ├── register_publisher.rs
│   │   ├── stake.rs
│   │   ├── unstake.rs
│   │   ├── create_campaign.rs
│   │   ├── fund_campaign.rs
│   │   ├── close_campaign.rs
│   │   ├── submit_claim.rs
│   │   ├── challenge_claim.rs
│   │   ├── resolve_claim.rs   # Attestor が settle / slash を裁定
│   │   └── settle_claim.rs    # 期限超過後は誰でも決済可
│   └── util.rs                # token転送ヘルパ（CPI）
├── programs/urthr-net/tests/  # LiteSVM ユニットテスト（Rust）
├── tests/                     # Surfpool 統合テスト（TS / anchor test）
├── app/                       # 既存フロント（#3 で拡張）
├── runbooks/                  # 既存 txtx デプロイ
├── docs/
│   ├── ARCHITECTURE.md        # 全体像・ロードマップ（本サブプロジェクトで作成）
│   └── superpowers/{specs,plans}/
└── README.md                  # 概要・ローカル起動手順（更新）
```

- `services/`（#2）, `clients/`（#3 codegen）は**今回は作らない**（YAGNI）。`ARCHITECTURE.md` に予定地として記載。
- 既存 `state.rs`（プレースホルダ）→ `state/` へ分割。既存 `initialize` → `initialize_protocol` へ発展。

---

## 4. データモデル（PDA アカウント）

### ProtocolConfig — seed `["config"]`
| field | type | 説明 |
|---|---|---|
| `admin` | Pubkey | config 更新権限 |
| `attestor` | Pubkey | challenge 裁定権限 |
| `payment_mint` | Pubkey | 許可する SPL mint（USDC 想定） |
| `protocol_fee_bps` | u16 | 手数料（bps、分母 10_000）。`<= 10_000` を検証 |
| `min_publisher_stake` | u64 | publisher 参加に必要な最小担保 |
| `challenge_window` | i64 | チャレンジ可能秒数 |
| `treasury` | Pubkey | 手数料受取トークンアカウント（ATA） |
| `paused` | bool | 緊急停止 |
| `bump` | u8 | |

### Publisher — seed `["publisher", authority]`
| field | type | 説明 |
|---|---|---|
| `authority` | Pubkey | 所有ウォレット |
| `staked_amount` | u64 | ステーク済み担保総額 |
| `locked_amount` | u64 | 係争中 Claim に拘束中の担保 |
| `metadata` | `[u8; 32]` | 媒体識別ハッシュ（固定長） |
| `stake_vault` | Pubkey | 担保保管 ATA（PDA 所有） |
| `bump` | u8 | |

不変条件: `locked_amount <= staked_amount`。新規 submit_claim は `staked_amount - locked_amount >= amount` を要求。

### Campaign — seed `["campaign", advertiser, campaign_id]`
| field | type | 説明 |
|---|---|---|
| `advertiser` | Pubkey | 広告主ウォレット |
| `campaign_id` | u64 | 広告主ごとの nonce |
| `price_per_event` | u64 | 有効イベント1件あたり支払額 |
| `budget_remaining` | u64 | 未拘束の残予算 |
| `locked_budget` | u64 | 係争中 Claim に拘束中の予算 |
| `claims_count` | u64 | 発行済み Claim 数。次の `claim_nonce` として使用し submit ごとに +1 |
| `status` | enum | Active / Paused / Closed |
| `escrow_vault` | Pubkey | 予算保管 ATA（PDA 所有） |
| `bump` | u8 | |

### Claim — seed `["claim", campaign, publisher, claim_nonce]`
| field | type | 説明 |
|---|---|---|
| `campaign` | Pubkey | 対象キャンペーン |
| `publisher` | Pubkey | 申告 publisher |
| `event_count` | u64 | 集計イベント件数（> 0） |
| `amount` | u64 | `event_count * price_per_event`（checked） |
| `merkle_root` | `[u8; 32]` | 将来の精密証拠用（MVP は保持のみ・未検証） |
| `status` | enum | Pending / Challenged / Settled / Slashed |
| `challenge_deadline` | i64 | `created_at + challenge_window` |
| `challenger` | `Option<Pubkey>` | チャレンジ者 |
| `evidence_hash` | `[u8; 32]` | チャレンジ証拠ハッシュ |
| `bump` | u8 | |

`claim_nonce` は Campaign の `claims_count`（単調増加カウンタ）を採用する。`submit_claim` で
現在値を seed に使って Claim PDA を導出し、その後 `claims_count += 1`。これにより
`(campaign, claims_count)` で一意性が保証される。

---

## 5. 状態機械（Claim ライフサイクル）

```
submit_claim (publisher 署名: 自分の成果を申告 = skin in the game)
  ├─ campaign.status == Active を確認
  ├─ campaign.budget_remaining >= amount を確認 → locked_budget へ移動
  ├─ publisher.staked_amount - locked_amount >= amount を確認 → locked_amount へ移動
  └─ challenge_deadline = now + challenge_window,  status = Pending
       │
   ┌───┴───────────────────────────┬──────────────────────────────┐
   │ challenge_claim                │ 期限超過・無チャレンジ           │
   │ (誰でも: 広告主/監視者)         │ settle_claim (permissionless)  │
   │ evidence_hash 記録             │   publisher へ amount - fee     │
   ▼ status = Challenged            │   fee → treasury, lock 解除     │
   resolve_claim (attestor のみ)    ▼   status = Settled
   ├─ 正当 → settle:
   │    publisher へ amount - fee, fee → treasury, lock 解除, Settled
   └─ 不正 → slash:
        campaign.locked_budget → budget_remaining へ返却（広告主は支払わない）
        publisher の locked 担保 amount → 広告主 escrow へ補填
        status = Slashed
```

### 経済ロジック
- `fee = amount * protocol_fee_bps / 10_000`（全て checked 演算）。
- **settle**: publisher が `amount - fee` 受取、`fee` は treasury、campaign の `locked_budget` を消費、publisher の `locked_amount` を解放。
- **slash**: 広告主は予算を消費せず（`locked_budget → budget_remaining`）、さらに publisher の `locked` 担保 `amount` が広告主 escrow に補填され、`staked_amount` から減算。
- `close_campaign` は Pending Claim が無いときのみ残予算を広告主へ返金。

---

## 6. Instruction 一覧

| # | instruction | 署名者 | 役割 |
|---|---|---|---|
| 1 | `initialize_protocol` | admin | Config＋treasury 作成（既存 `initialize` を発展） |
| 2 | `register_publisher` | publisher | Publisher＋stake_vault 作成 |
| 3 | `stake` | publisher | 担保入金（mint→stake_vault、staked_amount 加算） |
| 4 | `unstake` | publisher | 担保出金（`staked - locked >= 引出額` かつ min_stake 維持） |
| 5 | `create_campaign` | advertiser | Campaign＋escrow_vault 作成・単価設定 |
| 6 | `fund_campaign` | advertiser | 予算デポジット（mint→escrow_vault） |
| 7 | `close_campaign` | advertiser | 残予算返金（Pending Claim 無いとき） |
| 8 | `submit_claim` | publisher | 成果申告・budget/stake ロック・challenge 期間開始 |
| 9 | `challenge_claim` | 誰でも | evidence_hash 付きでチャレンジ |
| 10 | `resolve_claim` | **attestor** | settle / slash を裁定 |
| 11 | `settle_claim` | 誰でも | 期限超過の無チャレンジ Claim を決済 |

---

## 7. エラー設計（`UrthrError`）

`Unauthorized`（admin/attestor 署名違反）, `ProtocolPaused`, `InvalidMint`, `InvalidFeeBps`,
`InsufficientStake`（min 未満）, `StakeLocked`（unstake で locked 割れ）, `InsufficientBudget`,
`CampaignNotActive`, `InvalidEventCount`（0 件）, `ClaimNotPending`, `ClaimNotChallenged`,
`ChallengeWindowOpen`（期限前の settle）, `ChallengeWindowClosed`（期限後の challenge）,
`HasPendingClaims`（close 時）, `MathOverflow`（checked 演算）。

---

## 8. テスト戦略

- **ユニット（主）: LiteSVM（Rust, `programs/urthr-net/tests/`）** — 各 instruction の正常系＋主要失敗系。
  SPL Token 転送も含めインプロセスで高速検証。
- **統合（1本）: Surfpool（TS, `tests/`）** — USDC を模した mint をチートコードで用意し、フルライフサイクル：
  - 正常系: `fund → stake → submit_claim → 期限超過 → settle_claim`（publisher 入金・手数料・残予算を確認）
  - 不正系: `submit_claim → challenge_claim → resolve_claim(slash)`（担保没収・広告主補填を確認）
- 実行: `NO_DNA=1 anchor build` / `NO_DNA=1 anchor test`。

---

## 9. MVP 範囲（サブプロジェクト#1 の DONE 定義）

### IN（作る）
- `state.rs` → `state/` 分割、上記 11 instruction＋状態機械の実装（単一 mint・SPL Token CPI）
- Attestor 裁定・バッチ Claim・`merkle_root` 保持のみ（検証なし）
- LiteSVM ユニットテスト＋Surfpool 統合テスト1本
- `docs/ARCHITECTURE.md`（全体ロードマップ含む）・`README.md` 更新

### OUT（先送り＝ARCHITECTURE にロードマップ記載）
- オフチェーン Reporter/Attestor/Indexer（#2）、フロント ダッシュボード（#3）
- merkle 証明の精密検証、per-event チャレンジ
- 投票/ディスピュートゲーム裁定、複数 mint・価格オラクル
- Token-2022／秘匿送金
- campaign の pause/resume 専用 instruction（status enum は将来用に保持）

---

## 10. 後続サブプロジェクト（ロードマップ）

| # | サブプロジェクト | 概要 | 依存 |
|---|---|---|---|
| 2 | オフチェーンサービス | Reporter（イベント集約・claim 投稿）/ Attestor（裁定）/ Challenger（Bot 検知）/ Indexer | #1 IDL |
| 3 | フロントエンド ダッシュボード | 広告主（campaign 作成・入金・成果監視）/ パブリッシャー（登録・stake・収益）。既存 `app/` を拡張 | #1, #2 |
