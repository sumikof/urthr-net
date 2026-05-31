# UrthrNet デバッグ・ハーネス 設計（spec）

**日付:** 2026-05-31
**対象サブプロジェクト:** #3（フロントエンド）の土台 / 全機能横断の開発インフラ

## 目的

既存の `app/`（ウォレット接続 + initialize が動作確認済み）を、**UrthrNet の全オンチェーン機能を手動で叩けるデバッグ／テスト画面**に拡張する。あわせて「**各機能は対応するデバッグパネルを備えて初めて完了とする**」という開発ルールを確立し、`CLAUDE.md` と `features.md` に明記する。

このハーネスは、各機能の実装に着手する前の土台として整備し、以降に開発するすべての機能（[`features.md`](../../../features.md) の A/B/C 各項目）がこの画面から操作・検証できる状態を保つ。

## 背景・現状

- `app/` は Vite + React 19 + framework-kit（`@solana/client` / `@solana/react-hooks` / `@solana/kit`）構成。1コンポーネント＝1機能の素直なパターン（`InitializeButton` 等）。
- **`app/src/generated/` は旧スキャフォールドの IDL から生成されており `initialize` 1命令しか無い。** 現行 Protocol Core の11命令・4アカウントは未生成。
- 最新 IDL（11命令入り）は `target/idl/urthr_net.json` に存在するため、**`pnpm codegen` の再実行のみ**で全命令の型付きクライアントが得られる（プログラム再ビルド不要）。
- 既存 `InitializeButton` は**シミュレーションせず即送信**している。デバッグ用途では solana-dev ガードレール（送信前シミュレーション＋Tx要約表示）を全パネルで強制する。
- `initialize_protocol` 等は SPL `payment_mint` とトークンアカウントを要求するため、localnet 上に **テスト用 mint 作成・トークン付与のヘルパー**が無いと実際には叩けない。

## スコープ

### 含む（完全ハーネス）
1. 生成クライアントの再生成（11命令ビルダー + 4アカウントデコーダ）。
2. 共有デバッグ基盤（`DebugPanel`、`useInstructionRunner`、入力部品、PDA導出）。
3. 既存11命令すべてのデバッグパネル。
4. 4アカウント（ProtocolConfig / Publisher / Campaign / Claim）のインスペクタ。
5. localnet ヘルパー（テスト mint 作成 / ATA 作成 / 自分へトークン mint）。
6. `App.tsx` のドメイン別セクション再編。
7. 開発ルールの `CLAUDE.md`（新規・リポジトリ直下）＋ `features.md` への記述。
8. `debug/core` ロジックの vitest 単体テスト。

### 含まない
- 本番向けのUX・スタイリング・国際化（デバッグ用途のため最小限の実用UIに留める）。
- mainnet 対応（既定は localnet/devnet）。
- 自動E2E（実トランザクションは localnet 手動確認が本来用途。描画はスモークのみ）。
- 将来機能（A1管理命令、A2イベント等）のパネル — それらは各機能の開発時に本ルールに従って追加する。

## アーキテクチャ

### ディレクトリ構成

```
app/src/
  generated/                 # pnpm codegen で 11命令 + 4アカウント に再生成
  debug/
    core/
      DebugPanel.tsx          # 共通カード（後述の契約を提供）
      useInstructionRunner.ts # シミュ→要約→承認→送信 を担うフック
      fields.tsx              # text / number / pubkey / bytes32(hex) 入力部品 + 検証
      pdas.ts                 # オンチェーン seed をミラーした PDA 導出
      pdas.test.ts            # PDA 導出の単体テスト
      fields.test.ts          # 入力パース/検証の単体テスト
    instructions/             # 1命令 = 1パネル（11枚）
      InitializeProtocolPanel.tsx
      RegisterPublisherPanel.tsx
      StakePanel.tsx
      UnstakePanel.tsx
      CreateCampaignPanel.tsx
      FundCampaignPanel.tsx
      SubmitClaimPanel.tsx
      ChallengeClaimPanel.tsx
      ResolveClaimPanel.tsx
      SettleClaimPanel.tsx
      CloseCampaignPanel.tsx
    accounts/                 # 4インスペクタ
      ProtocolConfigInspector.tsx
      PublisherInspector.tsx
      CampaignInspector.tsx
      ClaimInspector.tsx
    localnet/
      CreateMintPanel.tsx     # テスト mint 作成
      MintToPanel.tsx         # ATA 作成 + 自分へトークン mint
  App.tsx                     # ドメイン別セクションに再編
```

### 共通契約（`DebugPanel`）— 開発ルールの実体

すべての命令パネルは `DebugPanel` を用い、以下を備える：

1. **引数入力** — 型付き・検証付き（`fields.tsx` の部品を使用）。
2. **アカウント入力** — PDA は `pdas.ts` で自動導出して表示、ユーザ指定アカウント（mint・相手 wallet・トークンアカウント等）は入力欄を提供。すべて手動上書き可。
3. **送信前シミュレーション必須** — `useInstructionRunner` 経由で `simulateTransaction` を先に実行し、要約（プログラム / 署名者 / 書込アカウント / 手数料 / ログ）を表示。ユーザの明示承認後に送信。
4. **結果表示** — 署名 / プログラムログ / エラーメッセージ。

アカウントを新設する機能は `accounts/` に対応インスペクタも追加する。

### `useInstructionRunner` の責務

framework-kit の `useTransactionPool` をラップし、次のフローを提供する：

- `build(): instruction` を受け取り、トランザクションを組み立てる。
- **必ず先に** `simulateTransaction` を実行し、結果を要約オブジェクトに整形して返す（UIが表示）。
- シミュ失敗時はログ・エラーを表示し、送信ボタンを無効化。
- ユーザ承認（明示クリック）後にのみ `prepareAndSend` で送信し、署名を返す。
- 既定 cluster は `rpc.ts` の localnet/devnet。mainnet は扱わない。

### PDA 導出（`pdas.ts`）

オンチェーン seed（`programs/urthr-net/src/constants.rs`）をミラーして導出する：

| PDA | seeds |
|---|---|
| config | `["config"]` |
| treasury | `["treasury"]` |
| publisher | `["publisher", authority]` |
| stake_vault | `["stake_vault", authority]` |
| campaign | `["campaign", advertiser, campaign_id(u64 LE)]` |
| escrow_vault | `["escrow_vault", campaign_pda]` |
| claim | `["claim", campaign_pda, claim_nonce(u64 LE)]` |

プログラム ID は生成クライアントの定数を使用する。導出は `getProgramDerivedAddress`（`@solana/kit`）で実装し、単体テストで既知の入力に対する出力を固定する。

### localnet ヘルパー

`initialize_protocol` 以降を実行するには SPL mint とトークン残高が必要なため：

- **CreateMintPanel** — 新規 SPL mint（decimals 指定可、mint authority = 接続ウォレット）を作成し、アドレスを表示。
- **MintToPanel** — 指定 mint の自分の ATA を作成（無ければ）し、指定量を mint する。

実装は `@solana-program/token`（無ければ追加）または生成済みトークン命令を用い、`useInstructionRunner` と同じシミュ→送信フローに通す。

### `App.tsx` レイアウト

ドメイン別セクションに再編（折りたたみ可の単純なセクションで可）：

1. ウォレット（既存 Connect / AccountInfo）
2. localnet（Airdrop / CreateMint / MintTo）
3. プロトコル設定（InitializeProtocol / ProtocolConfigInspector）
4. パブリッシャー（Register / Stake / Unstake / PublisherInspector）
5. キャンペーン（Create / Fund / Close / CampaignInspector）
6. Claim ライフサイクル（Submit / Challenge / Resolve / Settle / ClaimInspector）

## エラーハンドリング

- シミュレーション失敗・送信失敗は各パネル内にエラー文字列とログを表示し、握りつぶさない。
- アカウントデコード失敗（未作成・不正データ）はインスペクタに「未作成 / デコード不可」を表示。オンチェーンデータは untrusted として、所有者・データ長・discriminator を確認してから生成デコーダに渡す（生成クライアントが discriminator を検証）。
- 引数パース失敗（不正な pubkey、範囲外の数値、長さ不一致の bytes32）は送信前に入力欄でブロック。

## テスト

- **`debug/core` 単体テスト（vitest）：** PDA 導出（既知入力→固定出力）、引数パース/検証（pubkey / u64 範囲 / bytes32 hex 長）、シミュ要約整形。
- **パネル描画スモーク：** モック署名・モッククライアントでレンダリングが落ちないこと（最小限）。
- **手動確認（本来用途）：** localnet（surfpool）で代表フロー（initialize → mint → register → stake → create_campaign → fund → submit_claim → settle）を画面から実行できること。
- 既存の `pnpm test`（vitest）が緑であること。

## 開発ルール（成果物）

### `CLAUDE.md`（リポジトリ直下・新規）

開発ルールとして次を明記する：

> **デバッグパネル必須ルール:** オンチェーン命令を追加または変更する機能は、その**完了の定義**として、`app/src/debug/instructions/` に対応するデバッグパネルを追加（または更新）し、`App.tsx` に配線し、localnet で手動実行確認すること。新しいアカウント型を追加する機能は `app/src/debug/accounts/` にインスペクタも追加する。パネルは `DebugPanel` 契約（型付き引数入力 / アカウント入力＋PDA自動導出 / 送信前シミュレーション必須 / 結果表示）に従う。生成クライアントは IDL 変更時に `cd app && pnpm codegen` で再生成する。

加えて、ビルド/テストの要点（`NO_DNA=1 anchor build --ignore-keys`、`cargo test -p urthr-net` が正の correctness ゲート）を簡潔に記載する。

### `features.md`

各機能の「受け入れ条件」に「対応するデバッグパネル（必要ならインスペクタ）を追加し localnet で確認」を含める旨を、冒頭フォーマット説明に追記する。

## 受け入れ条件（全体）

- [ ] `cd app && pnpm codegen` 後、`app/src/generated/instructions/` に11命令が存在する。
- [ ] `app/src/debug/` に基盤・11命令パネル・4インスペクタ・localnetヘルパーが実装されている。
- [ ] すべての命令パネルが送信前シミュレーション＋要約表示を行い、明示承認後に送信する。
- [ ] `App.tsx` がドメイン別セクションで全パネルを配線している。
- [ ] localnet で initialize→mint→register→stake→create_campaign→fund→submit_claim→settle を画面から通せる。
- [ ] `pnpm test`（vitest）が緑、`pnpm build`（tsc + vite）が通る。
- [ ] `CLAUDE.md` に「デバッグパネル必須ルール」が記載されている。
- [ ] `features.md` のフォーマット説明にルールが反映されている。
