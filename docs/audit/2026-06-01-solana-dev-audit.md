# UrthrNet solana-dev 監査レポート (2026-06-01)

対象: `programs/urthr-net/`（オンチェーン Anchor プログラム、命令 11 個）。
監査基準: `/workspace/.claude/skills/solana-dev/references/security.md` の項目、および `docs/ARCHITECTURE.md` の設計意図（locking-is-pure-accounting 不変条件 `escrow_vault balance == budget_remaining + locked_budget`、単一 payment mint、attestor ロール、paused 緊急停止）。

本レポートは所見の提示のみで、コードは変更していない。修正は後続タスクで実施する。

## 対応状況

**全所見を対応済み**（第1次: `feat/review-cleanup`、第2次: `feat/audit-followups`）。

第1次（O-01/O-02/O-04）:

- **O-01 — 対応済み**: admin 署名の `set_paused(bool)` 命令を新設し、緊急停止を実効化（パネル `SetPausedPanel` 追加・LiteSVM テスト3件追加）。
- **O-02 — 対応済み**: `initialize_protocol` を program の upgrade authority に制約（`program`/`program_data` アカウント追加、front-run 乗っ取り防止、否定テスト追加）。
- **O-04 — 対応済み**: `escrow_vault`/`stake_vault` に `token::mint = payment_mint` 制約を追加（defense-in-depth）。

第2次（残り全件）:

- **O-03 — 対応済み**: `close_campaign` で残予算返却後に `escrow_vault` を `CloseAccount` で閉じ、rent を advertiser に返却。`campaign` は claim PDA 参照と `campaign_id` 再利用防止のため Closed tombstone として保持。テストで vault クローズを検証。
- **O-05 — 対応済み**: unstake の残高超過に専用エラー `UnstakeExceedsBalance` を新設し `InsufficientStake` との混同を解消。
- **O-06 — 対応済み**: `challenge_claim` に `publisher` アカウントを追加し `challenger != publisher.authority` を強制（自己チャレンジ禁止、`SelfChallengeNotAllowed`）。パネル更新・否定テスト追加。
- **A-01 — 対応済み**: `DebugPanel` に `resetKey` を追加し、入力変更時に stale なシミュレーションを無効化（全 14 パネルに配線）。
- **A-02 — 対応済み**: 送信完了/エラー後に「リセット」導線を追加。
- **A-03 — 対応済み**: デッドコード `RENT_SYSVAR_ADDRESS` を削除。

最終検証: `cargo test -p urthr-net` 33 passed、app `pnpm build`/`test`(25)/`lint`(0 errors) 緑。

## オンチェーン (Rust / Anchor)

### 所見

| ID | レイヤー | file:line | 重要度 | 内容 | 推奨修正 |
|----|---------|-----------|--------|------|----------|
| O-01 | onchain | lib.rs:16-68 / 全 instructions | 中 | `paused` 緊急停止が **到達不能**。`config.paused` を `true` にする命令も、admin が `attestor`/`protocol_fee_bps`/`min_publisher_stake`/`challenge_window` を更新する命令も存在しない（`initialize_protocol` で `paused=false` 固定、以降変更手段なし）。`!config.paused` 制約（register_publisher/stake/create_campaign/fund_campaign/submit_claim/settle_claim/resolve_claim）は事実上常に真で、設計上の防御層が機能しない。 | admin 署名（`has_one = admin`）を要求する `set_paused(bool)`（および任意で `update_config`）命令を追加し、緊急停止を実際に発火可能にする。ARCHITECTURE.md の「emergency stop」記述と整合させる。 |
| O-02 | onchain | initialize_protocol.rs:7-36,38-58 | 中 | `initialize_protocol` は permissionless。`config` PDA は固定 seed `[CONFIG_SEED]` の singleton で、最初に呼んだ者が `admin` になり `attestor`/`fee`/`mint`/`treasury` を任意設定できる。デプロイ直後に第三者が先行初期化すると、プロトコルを乗っ取る（または不正 mint で固定する）グリーフィングが可能。 | デプロイ手順で初期化を即時に同一トランザクション/原子的に行う運用を明文化するか、`declare_id` 所有者（upgrade authority 由来の既知鍵）を `admin` として要求する制約を入れる。最低限、運用上のリスクを docs に明記。 |
| O-03 | onchain | close_campaign.rs:7-33,38-69 | 中 | 命令名は `close_campaign` だが **アカウントを close していない**。`campaign` に `close =` 制約はなく、`escrow_vault`（PDA TokenAccount）も閉じない。残予算は払い戻すが、`campaign`/`escrow_vault` の rent lamports は回収されず常駐し、status=Closed のまま残る（再 close は `status == Active` 制約で防止されるため revival/double-close は無い）。チェックリスト#8（close 時の rent 返還先）の意図を満たさない。 | 意図が「払い戻しのみ・アカウント保持」なら docs/命令名を実態へ寄せる。意図が真の close なら `escrow_vault` を SPL `CloseAccount` で閉じて lamports を advertiser へ返し、`campaign` に `close = advertiser` を付与する（その場合 revival 対策として同一 tx 内 init 再生成の検討も）。 |
| O-04 | onchain | resolve_claim.rs:39-40 / settle_claim.rs:35-36 | 低 | `escrow_vault` が `#[account(mut)]` のみで `token::mint = payment_mint` 明示制約が無い（`has_one = escrow_vault` による campaign 紐付け＋`transfer_checked` の mint 検証で実害は防がれるが、防御的検証として一段弱い）。stake.rs/unstake.rs の vault も同様に `token::mint` 明示なし（こちらも `has_one` で紐付け）。 | `escrow_vault`/`stake_vault` に `token::mint = payment_mint` を付与し、紐付けと mint 検証を制約レイヤーで二重化する（defense-in-depth）。 |
| O-05 | onchain | unstake.rs:40 | 低 | `staked_amount.checked_sub(amount).ok_or(UrthrError::InsufficientStake)` は「要求額 > stake 残高」の桁不足を `InsufficientStake`（「最小ステーク未満」用）で表現しており、エラー意味が曖昧。 | 残高不足専用の理由（例: `MathOverflow` ではなく新設の `InsufficientBalance` 等）を割り当てるか、コメントで意図を明記。機能影響なし。 |
| O-06 | onchain | challenge_claim.rs:19-28 | 低 | `challenger == claim.publisher`（自己チャレンジ）を禁止していない。資金移動は伴わず resolve は attestor 限定なので直接の悪用経路は無いが、状態機械上で publisher が自分の claim を Challenged に遷移させられる。 | 必要なら `require!(ctx.accounts.challenger.key() != claim_publisher_authority, ...)` を追加。設計上許容なら docs に明記。 |

### 網羅メモ (所見なしの観点)

- OK: 署名者チェック（#1）— 全命令が意図した署名者を要求。`stake`/`unstake`/`fund_campaign`/`close_campaign` は `has_one = authority|advertiser`（stake.rs:23, unstake.rs:22, fund_campaign.rs:23, close_campaign.rs:18）、`submit_claim` は `publisher.authority == publisher_authority.key()`（submit_claim.rs:22）。
- OK: attestor 限定（#1）— `resolve_claim` は `config.attestor == attestor.key()`（resolve_claim.rs:19）かつ `attestor: Signer`（resolve_claim.rs:11）で permissioned。`settle_claim` は意図通り permissionless（attestor/admin 制約なし、ARCHITECTURE.md 表#10 と一致）。
- OK: オーナー/型検証（#2）— 全状態アカウントは `Account<'info, T>`（8 byte discriminator + owner = program）。vault は `Account<'info, TokenAccount>`＋`token::mint`/`token::authority` 制約（initialize_protocol.rs:26-27, register_publisher.rs:34-35, create_campaign.rs:35-36）。`token_program: Program<'info, Token>` で arbitrary CPI を防止（security.md #3）。
- OK: PDA seeds/bump（#3）— 全 PDA は init 時に canonical bump（`ctx.bumps.*`）を保存（initialize_protocol.rs:56, register_publisher.rs:52, create_campaign.rs:58, submit_claim.rs:84）、再検証時は保存 bump を `bump = config.bump` 等で使用。`Publisher`/`Campaign` の seed にユーザ固有鍵（authority / advertiser+campaign_id）を含み PDA 共有脆弱性（security.md #5）なし。`Claim` の seed に `campaign.key()` を含むため resolve/settle で別 campaign の claim を渡せない（resolve_claim.rs:33, settle_claim.rs:29）。
- OK: 算術（#4）— 残高/金額の加減乗は全て `checked_*`＋`MathOverflow`（stake.rs:54, fund_campaign.rs:55, submit_claim.rs:52-87, resolve_claim.rs:94-127, settle_claim.rs:79-83, close_campaign.rs 払戻は転送のみ）。手数料は 128-bit 経由（util.rs:8-15）。`challenge_deadline` は `i64::try_from`＋`checked_add`（submit_claim.rs:79-82）。素の `+ - *` は検出されず。
- OK: CPI 安全性（#5）— vault からの払い出しは全て PDA seeds 付き `CpiContext::new_with_signer`：unstake は Publisher PDA（unstake.rs:48-67）、settle/resolve(settle) は Campaign PDA（settle_claim.rs:103-141）、resolve(slash) は stake_vault→escrow を Publisher PDA（resolve_claim.rs:71-90）、close_campaign は Campaign PDA（close_campaign.rs:44-63）。anchor-lang 1.0.2 では `CpiContext::new(program_id: Pubkey, ...)` 仕様のため `token_program.key()` 受け渡しは正しい（context.rs:188）。全転送が `transfer_checked`（mint+decimals 検証、security.md #15）。
- OK: 単一 mint ルール（#9）— 資金移動命令の `config` に `has_one = payment_mint @ InvalidMint`（register_publisher.rs:16, stake.rs:15, unstake.rs:14, fund_campaign.rs:13, resolve_claim.rs:17, settle_claim.rs:14, close_campaign.rs:11）。ユーザ token account は `token::mint = payment_mint`＋`token::authority` で縛り（stake.rs:31, fund_campaign.rs:32, resolve_claim.rs:56, settle_claim.rs:48, close_campaign.rs:28）。
- OK: 再初期化（#7）— 全新規アカウントは `init`（`init_if_needed` 不使用 = security.md #4 推奨）。singleton/per-user PDA のため二重初期化は Anchor が拒否。
- OK: paused 適用範囲（#8、コード実装としての配置）— 資金移動系（register/stake/create/fund/submit/settle/resolve）に `!config.paused` 制約が存在し、`unstake`（unstake.rs に制約なし）と `close_campaign`（close_campaign.rs:35-37 にコメント付き明示的除外）は exit/withdrawal として意図的に非ゲート。配置自体は設計意図と一致（ただし O-01 の通り paused を立てる手段が無く実効しない）。
- OK: 重複可変アカウント（#10, security.md #7）— resolve_claim の `stake_vault`/`escrow_vault`/`treasury`/`publisher_token_account` はそれぞれ別 PDA seed（stake_vault↔publisher、escrow_vault↔campaign、treasury↔config）に `has_one` で束縛され、同一アドレスを別パラメータに流用できない（resolve_claim.rs:27,49,16+has_one treasury）。
- OK: 不変条件（#10）— `escrow balance == budget_remaining + locked_budget` が submit（純会計、トークン移動なし: submit_claim.rs:60-67）/settle（escrow から amount 流出、LB-=amount: settle_claim.rs:65-83）/slash（stake から amount 流入、BR+=2*amount かつ LB-=amount: resolve_claim.rs:77-106）/close（残 BR 払戻し: close_campaign.rs:50-66）の各経路で保存される（手計算で検証済み）。LiteSVM テスト（`programs/urthr-net/tests/`）が correctness の正ゲート。
- OK: Token-2022 前提（#9, security.md #15）— `anchor_spl::token::{Token, TokenAccount, transfer_checked}` を使用し SPL classic 前提。ARCHITECTURE.md で単一 SPL mint 運用を明記、Token-2022 は post-MVP ロードマップ。現状の単一 classic mint 運用では transfer-fee/permanent-delegate 等の Token-2022 リスクは対象外。
- OK: revival/double-close（#6, security.md #8）— `close_campaign` は実際には close せず（O-03）、再実行は `status == Active` 制約で拒否されるため、closed-account revival 経路は存在しない。claim/campaign の状態遷移は status ガード（ClaimNotPending/ClaimNotChallenged 等）で冪等性を担保。
- OK: sysvar（security.md #10/#13）— 時刻取得は `Clock::get()`（submit_claim.rs:69, challenge_claim.rs:22, settle_claim.rs:58）で canonical sysvar を検証。Anchor の `Sysvar<'info, Rent>` も init 命令で型付き。

## app (kit / フロントエンド)

対象: `app/src/`（React + Vite localnet デバッグハーネス）。命令パネル 11 個（`debug/instructions/*.tsx`、オンチェーン命令と 1:1）＋ localnet ヘルパ 2 個（`debug/localnet/*.tsx`）＝ 計 13 個が共通の `DebugPanel`/`useInstructionRunner` を経由。account インスペクタ 4 種＋汎用 `AccountInspector`。監査基準は CLAUDE.md の「パネル契約」「kit バージョン乖離の共有ヘルパ経由」「送信前シミュレーション必須」不変条件。

### 所見

| ID | レイヤー | file:line | 重要度 | 内容 | 推奨修正 |
|----|---------|-----------|--------|------|----------|
| A-01 | app | useInstructionRunner.ts:111-123 / DebugPanel.tsx:57-76 | 低 | シミュレーション後に入力を変更しても `summary`/`canSend` が `simulated` のまま残り、ユーザが古いシミュレーション結果のまま「承認して送信」を押せる（`build` は送信時ではなく `simulate` 時にのみ実行されプール済み tx を送るため、表示中の入力ではなく直前にシミュレートした内容が送られる）。送信内容＝シミュレート内容なので不変条件（シミュレート済みのものだけ送信）自体は守られるが、UI 上の入力と乖離し得る。 | パネル入力 (`children` の値) 変更時に `runner.reset()` を呼ぶ、または `DebugPanel` に入力変更を検知して `simulated`→`idle` へ戻す仕組みを追加。最小対応として「送信は直近シミュレート時の入力が対象」と UI に明示。 |
| A-02 | app | DebugPanel.tsx:30-98 / useInstructionRunner.ts:75-81 | 低 | `useInstructionRunner` は `reset()` を公開するが `DebugPanel` がどこからも呼ばず、エラー/送信完了後に状態をクリアする UI が無い。`error`/`sent` 表示後の再操作は「シミュレート」再押下に依存（再シミュレートで内部状態は上書きされるため実害は小）。 | `DebugPanel` に「リセット」ボタンを足して `runner.reset()` を配線するか、`phase==="sent"|"error"` で簡易リセット導線を提示。 |
| A-03 | app | core/programs.ts:7-10 | 低 | `RENT_SYSVAR_ADDRESS` を export しているが、現行 11 パネルはいずれも rent sysvar を明示的に渡さない（生成クライアントが解決）。デッドコードで、誤って手動配線する温床になり得る。 | 不要なら削除。将来必要なら使用箇所と同時に復活させる。 |

### 網羅メモ (所見なしの観点)

- OK: 共有 prepare ヘルパ（#1）— 全送信は `useInstructionRunner` 経由で `pool.prepare(runnerPrepareOptions(signer, wallet))` を呼ぶ（useInstructionRunner.ts:103）。`runnerPrepareOptions` は `{ feePayer: signer, authority: wallet }` を返し（prepareOptions.ts:24）、fee payer は `build` に渡す signer と同一インスタンス。kit 5.5.1 の参照比較 dedup を満たす（prepareOptions.test.ts:27,34 で回帰固定）。
- OK: feePayer にアドレスを渡さない（#1）— `src` 全体（生成コード除く）で `feePayer:` は `prepareOptions.ts` とそのテストのみ。`pool.prepare` に `feePayer: wallet.account.address` を渡す箇所は存在しない（grep 済み）。
- OK: bigint 安全な stringify（#1）— RPC データ表示は全て `stringifyWithBigInt`（useInstructionRunner.ts:120 のシミュレーション err、AccountInspector.tsx:80 のアカウント data）。生成コード以外で素の `JSON.stringify` を RPC レスポンス/エラーに適用する箇所は無い（唯一の `JSON.stringify` は `lib/json.ts:10` のラッパ実装本体）。
- OK: 送信エラーの unwrap（#1）— 送信/シミュレーション例外は `errMessage`→`describeTransactionError`（useInstructionRunner.ts:46,126,147）で `transactionPlanResult` の失敗 leaf とプログラムログまで展開（txError.ts:16-63、txError.test.ts で回帰固定）。
- OK: シミュレート→承認→送信 不変条件（#2）— 全 13 パネルが `DebugPanel` を使用（instructions/*.tsx 各 build= の親、localnet/*.tsx も同様）。`send()` は `phase !== "simulated"` を弾き（useInstructionRunner.ts:135-139）、送信ボタンは `canSend=(phase==="simulated")` でのみ活性（DebugPanel.tsx:51, useInstructionRunner.ts:158）。シミュレーションは送信前に必ず実行され、err/CU/logs を要約表示（DebugPanel.tsx:57-76）。バイパス経路なし。
- OK: シミュレーション要約の提示（#2）— err（成功/失敗）、`unitsConsumed`（CU）、`logs` を `DebugPanel` が表示（DebugPanel.tsx:59-74）。失敗時は `phase="error"`＋bigint 安全なエラー文字列（useInstructionRunner.ts:116-120）。
- OK: permissionless 命令の signer 扱い（#2,#5）— `settle_claim` は signer アカウントを持たず `build` で `void signer`（SettleClaimPanel.tsx:32）。fee payer（ウォレット）が署名する設計に一致し、不要な signer を埋め込まない。
- OK: 生成クライアント ↔ IDL 整合（#3）— `pnpm codegen` 実行後 `git diff --stat src/generated` 差分なし（クリーン）。生成物の改変残渣なし（`git status --porcelain` 空）。
- OK: cluster 既定（#4）— `lib/rpc.ts:1-2` で既定 `http://127.0.0.1:8899` / `ws://127.0.0.1:8900`（localnet）。`VITE_RPC_URL`/`VITE_WS_URL` で上書き可。mainnet ハードコードなし。`providers.tsx` は `autoDiscover()` で wallet-standard コネクタ列挙、秘密鍵保持なし。
- OK: 秘密鍵を保存しない（#4, security ガードレール）— 署名はウォレット（`createWalletTransactionSigner(wallet)`、useInstructionRunner.ts:71）に委譲。`CreateMintPanel` の ad-hoc mint 鍵は `generateKeyPairSigner()` で生成しメモリ上のみ・永続化なし（CreateMintPanel.tsx:35）。秘密鍵/シードの localStorage 等への保存は無い。
- OK: 未信頼オンチェーンデータの扱い（#4）— アカウント表示は全て生成 `fetchMaybe*`（discriminator 検証付きデコーダ）経由で `exists` 判定後に表示（AccountInspector.tsx:73-81、各 Inspector は `fetchMaybeCampaign`/`Claim`/`Publisher`/`ProtocolConfig`）。`SubmitClaimPanel` の nonce も生成 `fetchCampaign` でデコードした `claimsCount` を使用（SubmitClaimPanel.tsx:39-40）。生バイト直読みなし。
- OK: PDA/ATA 導出（パネル契約）— PDA は `core/pdas.ts` の集中導出（canonical、seed は CLAUDE.md/オンチェーンと一致）、ATA は `client.splToken({ mint }).deriveAssociatedTokenAddress(owner)`（stake/unstake/fund/close/settle/resolve）または `findAssociatedTokenPda`（MintTo）。手動 seed 組み立ての散在なし。
- OK: 型付き入力（パネル契約）— 全パネルが `core/fields.tsx` の `parseU64`/`parseU16`/`parseBytes32Hex`/`parsePubkey`＋`TextField` を使用し、未パース時は `disabled` でシミュレート/送信を抑止。`create_campaign` は `price>0` をクライアント側でも弾く（CreateCampaignPanel.tsx:19-20、オンチェーン InvalidPrice と整合）。
- OK: エラー握り潰しなし（#5）— 全 `catch` は `setError`/`setMessage`/`setState({kind:"error"})` でメッセージを surface（AirdropButton.tsx:22, MintToPanel.tsx:43, AccountInspector.tsx:82, useInstructionRunner.ts:124,145）。空 catch は `parsePubkey`（fields.tsx:27、パース失敗を `{ok:false}` に正規化する正当な用途）のみ。
- OK: コンポーネントのエラー表示（#1 適用範囲）— `AirdropButton`/`MintToPanel` 残高取得の catch は RPC レスポンス JSON ではなく投げられた `Error` の `message` を表示するため `stringifyWithBigInt` 不要（bigint を含む RPC 値を stringify していない）。

## サマリ

- 重要度別件数（オンチェーン O-xx ＋ app A-xx 合算）: 高 0 / 中 3 / 低 6
  - 中（3）: O-01, O-02, O-03（いずれもオンチェーンの堅牢性/運用）
  - 低（6）: O-04, O-05, O-06, A-01, A-02, A-03
- 推奨対応順:
  1. O-02（permissionless initialize のフロントラン乗っ取り。運用/制約のいずれかで早急に対処、最低でも docs 明記）
  2. O-01（paused を立てる admin 命令が無く緊急停止が実効しない。設計意図との不整合）
  3. O-03（close_campaign が実際には close せず rent 常駐。命令名/docs と実態の整合 or 真の close 実装）
  4. O-04（vault に `token::mint` 明示制約を追加、defense-in-depth）
  5. A-01（シミュレート後の入力変更で stale シミュレーション送信を防ぐ UI 改善）
  6. A-02 / A-03 / O-05 / O-06（UI リセット導線・デッドコード削除・エラー意味付け・自己チャレンジ禁止。スタイル/軽微）
- 補足: 正当性（correctness）に直結する 高 重要度の所見は onchain/app ともに無し。オンチェーンの不変条件（escrow balance == budget_remaining + locked_budget）と署名者/PDA/算術/CPI の各検証は満たされ（O-xx 網羅メモ参照）、app は CLAUDE.md のパネル契約・kit 乖離共有ヘルパ・シミュレート必須不変条件をすべて遵守している。
