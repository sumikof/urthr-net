# UrthrNet デバッグハーネス 操作ランブック（全機能テスト手順）

`app/`（localnet デバッグ画面）を **画面の上から順に** 操作して、オンチェーン全命令を手動で網羅検証するための手順書。
各パネルは「**シミュレート → 内容確認 → 承認して送信**」の契約（[`CLAUDE.md`](../CLAUDE.md)）に従う。

> 対象範囲: 現状デプロイ済みの **14 実行**（12 命令 + `resolve_claim` の fraud 2 経路）。`set_paused` は実装済みでパネルあり（手順 3 参照）。
> 残りの管理命令（`update_attestor` / `update_fee` / `update_min_stake`, features.md A1）は未実装のため本書では未カバー。

---

## 0. 前提

1. surfnet 起動 + program デプロイ済み（[`runbooks/`](../runbooks/) / `surfpool start --watch`）。
2. アプリ起動: `cd app && pnpm dev`。
3. wallet-standard 対応ウォレット（Phantom 等）を localnet RPC に向けて用意。

---

## テスト方針（最初に読む）

- **単一ウォレットで大半のロールを兼ねる**：同じ接続ウォレットが `admin` / `attestor` / `publisher authority` / `advertiser` を兼任できる。**ただし `challenge_claim` の `challenger` は publisher authority と別のウォレットである必要がある**（自己チャレンジは `SelfChallengeNotAllowed` で弾かれる）。チャレンジ系（経路 A/B）を試すときだけ、別のウォレットを接続して challenge する。
- **payment_mint は全命令で同一**：手順 1 で作る mint を、initialize の `payment_mint` にも、register / stake / create / fund / resolve / settle の `payment_mint` 入力にも **すべて同じ値** で使う（不一致は `InvalidMint`）。
- **PDA は自動導出**：`config` / `publisher` / `stake_vault` / `campaign` / `escrow_vault` / `claim` / `treasury` はパネルが自動導出。ATA は `payment_mint` から自動導出。**ユーザが手入力するのは値と「ユーザ指定アカウント」だけ**（`payment_mint`、`advertiser`、`publisher_authority`、ClaimInspector の `campaign` PDA）。
- **challenge_window の設計が重要**（後述）：`settle_claim` は「期限切れ後」、`challenge_claim` / `resolve_claim` は「期限内」。両方を試すため **短め（例 15 秒）** を推奨。

### claim ライフサイクル（状態遷移）

```
submit_claim ─► Pending ─┬─(期限内に challenge)─► Challenged ─┬─ resolve_claim(fraud=false) ─► Settled
                         │                                    └─ resolve_claim(fraud=true)  ─► Slashed
                         └─(誰も challenge せず期限経過)──────── settle_claim ─────────────► Settled
```

- `challenge_deadline = 送信時刻 + config.challenge_window`
- `challenge_claim`: `Pending` かつ `now <= deadline`
- `settle_claim`: `Pending` かつ `now > deadline`（期限前に送ると `ChallengeWindowOpen`）
- `resolve_claim`: `Challenged` のみ。署名は `config.attestor` のウォレット必須

---

## 1. ウォレット（画面: ウォレット）

1. **接続**：`ConnectButton` で「〇〇 で接続」。
2. **Airdrop**：`2 SOL Airdrop (localnet)` を押し、`AccountInfo` の残高が増えることを確認（rent / 手数料用の SOL）。

---

## 2. localnet — 支払い mint の用意（画面: localnet）

このセクションの mint が、以降すべての `payment_mint` になる。

1. **mint作成**
   - `decimals (0〜9)` = `6`
   - 「mint鍵を生成」を押す → 「シミュレート」→「承認して送信」
   - 表示された **`mint:` のアドレスを控える**（＝ `<MINT>`）。
2. **mintTo → 自分のATA**
   - `mint` = `<MINT>`
   - `amount (base units)` = 例 `1000000000`（後段の stake + fund を十分賄う額）
   - 送信後、「残高を更新」で残高が反映されることを確認。

> 補足: mintTo は接続ウォレットの ATA を冪等作成して mint する。この ATA が後の `authority/advertiser/publisher token account` として使われる。

---

## 3. プロトコル設定 — `initialize_protocol`（画面: プロトコル設定）

1. **initialize_protocol**
   - `attestor (pubkey)` = **自分のアドレス**（後で `resolve_claim` を自分で実行するため）
   - `protocol_fee_bps (u16)` = `50`（0.5%。`<= 10000`）
   - `min_publisher_stake (u64)` = `1000000`
   - `challenge_window (u64 秒)` = **`15`**（settle を短時間で試すため）
   - `payment_mint (pubkey)` = `<MINT>`
   - 送信。
2. **ProtocolConfigInspector** の「取得」で確認：`paused=false` / `attestor` = 自分 / `payment_mint` = `<MINT>` / `treasury` 生成済み。
3. （任意）**set_paused**：`paused` チェックボックスで緊急停止を ON/OFF（admin 署名）。ON の間は資金移動命令（register / stake / create / fund / submit / settle / resolve）が `ProtocolPaused` で弾かれ、`unstake` / `close_campaign` は引き続き可能。ProtocolConfigInspector で `paused` を確認。検証後は OFF に戻して以降の手順を進める。

> `initialize_protocol` は `treasury`（config 権限の SPL アカウント）も生成する。mint 作成（手順 2）より後に実行すること。
> `initialize_protocol` はデプロイ／upgrade authority のウォレットでのみ実行できる（パネルが `program` / `program_data` を自動導出するため手入力は不要）。

---

## 4. パブリッシャー（画面: パブリッシャー）

1. **register_publisher**
   - `metadata (bytes32 hex)` = 64 桁 hex（例: `0`×64、または任意の 32 バイト）
   - `payment_mint (pubkey)` = `<MINT>`
   - 送信（`publisher` / `stake_vault` PDA を生成）。
2. **PublisherInspector**：`authority` = 自分 → 取得（`staked_amount=0` / `locked_amount=0`）。
3. **stake**
   - `amount (u64)` = `5000000`（後段 claim の `amount` 以上にしておく）
   - `payment_mint` = `<MINT>` → 送信。Inspector で `staked_amount` 増加を確認。
4. **unstake**（少額で動作確認）
   - `amount (u64)` = `1000000`、`payment_mint` = `<MINT>` → 送信。
   - 検証ポイント: 残額は `0` か `min_publisher_stake` 以上、かつ `locked_amount` 未満には下げられない（claim ロック中は弾かれる＝後段で再確認可）。

---

## 5. キャンペーン（画面: キャンペーン）

1. **create_campaign**
   - `campaign_id (u64)` = `0`
   - `price_per_event (u64, must be > 0)` = `1000`
   - `payment_mint (pubkey)` = `<MINT>` → 送信。
2. **CampaignInspector**：`advertiser` = 自分 / `campaign_id` = `0` → 取得。
   - `status=Active` / `budget_remaining=0` / `claims_count=0` を確認。
   - **表示されたアドレス（= campaign PDA `<CAMPAIGN_PDA>`）を控える**（ClaimInspector で使用）。
3. **fund_campaign**
   - `campaign_id` = `0`、`amount (u64)` = `10000000`、`payment_mint` = `<MINT>` → 送信。
   - Inspector で `budget_remaining` 増加を確認。
4. **close_campaign は手順 6（claim 全解決後）に実行**（`locked_budget=0` が条件）。

---

## 6. Claim ライフサイクル（画面: Claim ライフサイクル）

> **claim_nonce のルール**：claim PDA の nonce は **submit 時の `campaign.claims_count`**。最初の claim = `0`、次 = `1`、その次 = `2`…。
> submit 前に CampaignInspector の `claims_count` を見れば、これから作る claim の nonce が分かる。

`amount = event_count × price_per_event`。`budget_remaining` と「利用可能 stake（`staked_amount − locked_amount`）」が `amount` 以上必要。
本手順では `event_count=1`, `price_per_event=1000` なので `amount=1000`（stake 5,000,000 / budget 10,000,000 に対し十分）。

3 つの経路を **別々の claim_nonce** で順にテストする。

### 経路 A: チャレンジ → 裁定（支払い / Settled）

1. **submit_claim**（nonce=0 になる）
   - `advertiser (pubkey)` = 自分、`campaign_id` = `0`、`event_count (u64)` = `1`、`merkle_root (bytes32 hex)` = 64 桁 hex（例 `1`×64）→ 送信。
   - CampaignInspector で `locked_budget`↑ `claims_count`→`1`、PublisherInspector で `locked_amount`↑ を確認。
2. **challenge_claim**（**期限内に**、**publisher authority とは別のウォレットで接続して実行**）
   - `advertiser` = 自分、`campaign_id` = `0`、`claim_nonce (u64)` = `0`、`publisher_authority (pubkey)` = **元の publisher のアドレス**、`evidence_hash (bytes32 hex)` = 64 桁 hex（例 `2`×64）→ 送信。
   - ※ publisher authority 自身で送ると `SelfChallengeNotAllowed` で弾かれる。
   - ClaimInspector（`campaign` = `<CAMPAIGN_PDA>`, `claim_nonce` = `0`）で `status=Challenged` を確認。
3. **resolve_claim**（`fraud` = **OFF**）
   - `advertiser` = 自分、`campaign_id` = `0`、`claim_nonce` = `0`、`publisher_authority (pubkey)` = 自分、`payment_mint` = `<MINT>`、`fraud (bool)` = **チェックしない** → 送信。
   - 結果: `status=Settled`。`payout = amount − fee` が publisher ATA、`fee` が treasury。CampaignInspector `locked_budget`↓、PublisherInspector `locked_amount`↓。

### 経路 B: チャレンジ → 裁定（スラッシュ / Slashed）

1. **submit_claim**（nonce=1）：経路 A.1 と同様、新しい claim を作成。
2. **challenge_claim**（期限内、publisher と別ウォレットで）：`claim_nonce` = `1`、`publisher_authority` = 元の publisher のアドレス → 送信（`Challenged`）。
3. **resolve_claim**（`fraud` = **ON**）
   - `claim_nonce` = `1`、他は経路 A.3 と同様、**`fraud (bool)` をチェック** → 送信。
   - 結果: `status=Slashed`。stake_vault → escrow_vault へ `amount` 移動、PublisherInspector `staked_amount`↓、CampaignInspector `budget_remaining` が **`amount × 2`**（ロック解除分＋スラッシュ補償）増加。

### 経路 C: 無チャレンジ → 期限後 settle（Settled）

1. **submit_claim**（nonce=2）：新しい claim を作成。
2. **`challenge_window` 秒（=15 秒）以上待つ**（`challenge_deadline` を経過させる）。
3. **settle_claim**（**署名アカウントなし＝permissionless**, fee payer はウォレット）
   - `advertiser` = 自分、`campaign_id` = `0`、`claim_nonce` = `2`、`publisher_authority` = 自分、`payment_mint` = `<MINT>` → 送信。
   - 結果: `status=Settled`（payout/fee の分配は resolve(settle) と同じ）。
   - ※期限前に送ると `ChallengeWindowOpen`、誰かが challenge 済みだと `ClaimNotPending`。

各 claim は **ClaimInspector**（`campaign (PDA)` = `<CAMPAIGN_PDA>`, `claim_nonce`）で `status` / `challenger` / `challenge_deadline` を確認。

---

## 7. 後始末（残りの命令）

1. **close_campaign**：全 claim 解決後（`locked_budget=0`）に `campaign_id` = `0`、`payment_mint` = `<MINT>` で送信。
   - `budget_remaining` が advertiser ATA に返却され、`escrow_vault` が閉じられて rent が返却され、`status=Closed`。
2. （任意）**unstake** で残り stake を引き出し（`locked_amount=0` のはず）。`amount` = 残額 → 送信。

---

## 全命令網羅チェックリスト

| # | 命令 | 操作する手順 | 署名ロール | 主な前提 |
|---|------|------|------|------|
| 1 | `initialize_protocol` | 3 | admin（自分） | mint 作成済み, upgrade authority |
| 1b | `set_paused` | 3.3 | admin | config.admin |
| 2 | `register_publisher` | 4.1 | authority（自分） | !paused, mint 一致 |
| 3 | `stake` | 4.3 | authority | !paused, mint 一致, ATA に残高 |
| 4 | `unstake` | 4.4 / 7.2 | authority | 残額 0 or ≥ min_stake, locked 以上 |
| 5 | `create_campaign` | 5.1 | advertiser | !paused, price>0 |
| 6 | `fund_campaign` | 5.3 | advertiser | !paused, Active, ATA に残高 |
| 7 | `submit_claim` | 6.A.1 / 6.B.1 / 6.C.1 | publisher authority | !paused, Active, budget/stake ≥ amount |
| 8 | `challenge_claim` | 6.A.2 / 6.B.2 | challenger（publisher authority 以外の誰でも） | Pending, 期限内, challenger ≠ publisher authority |
| 9 | `resolve_claim`（fraud=false） | 6.A.3 | attestor（=config.attestor） | !paused, Challenged |
| 10 | `resolve_claim`（fraud=true） | 6.B.3 | attestor | !paused, Challenged |
| 11 | `settle_claim` | 6.C.3 | なし（permissionless） | Pending, 期限切れ |
| 12 | `close_campaign` | 7.1 | advertiser | Active, locked_budget=0 |

> 未カバー（実装後に追記）: `update_attestor` / `update_fee` / `update_min_stake`（features.md **A1**）。

---

## アカウントインスペクタ早見表

| インスペクタ | 入力 | 導出/表示 |
|------|------|------|
| ProtocolConfigInspector | なし | `config` PDA。admin/attestor/payment_mint/treasury/fee_bps/min_stake/challenge_window/paused |
| PublisherInspector | `authority` | `publisher` PDA。staked_amount/locked_amount/stake_vault/metadata |
| CampaignInspector | `advertiser`, `campaign_id` | `campaign` PDA。status/budget_remaining/locked_budget/claims_count（**表示アドレスが `<CAMPAIGN_PDA>`**） |
| ClaimInspector | `campaign (PDA)`, `claim_nonce` | `claim` PDA。status/amount/challenger/challenge_deadline/merkle_root |

各インスペクタは「取得」で再フェッチ。未作成なら「未作成」、デコード不可ならエラーを表示。

---

## トラブルシュート

- **`InvalidMint`**：`payment_mint` が config と不一致。全命令で手順 2 の `<MINT>` を使う。
- **`Unauthorized`（resolve）**：`config.attestor` 以外のウォレットで署名している。initialize で attestor=自分にしておく。
- **`ChallengeWindowOpen`（settle）/ `ChallengeWindowClosed`（challenge）**：時間条件違反。settle は期限後、challenge は期限内。
- **`ClaimNotPending` / `ClaimNotChallenged`**：claim の状態と命令が不一致。ClaimInspector で `status` を確認。
- **claim PDA が見つからない**：`claim_nonce` の取り違え。submit 時の `claims_count` 値が nonce。
- **シミュレーション失敗**：要約・プログラムログに実エラーが出る（BigInt / transaction-plan の詳細表示は対応済み）。送信前に必ず内容を確認すること。
