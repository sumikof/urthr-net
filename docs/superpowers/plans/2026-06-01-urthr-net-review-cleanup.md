# UrthrNet レビュー＆クリーンアップ 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 現役ドキュメントを日本語へ統一し、Rust オンチェーンプログラムと `app/` を solana-dev のベストプラクティスに適合させる（監査→承認→修正）。

**Architecture:** 2 トラックを順次実行。先に Track B（プログラム監査レポート → 承認ゲート → 修正）で挙動を確定し、その最終状態を Track A（現役ドキュメント日本語化）で一度だけ反映する。これによりドキュメントの二度手間を避ける。

**Tech Stack:** Anchor 1.0.2 / Rust / LiteSVM、Vite + React + `@solana/client`（kit 5.5.1）/ `@solana/kit`（6.9.0）、Codama 生成クライアント、Markdown ドキュメント。

**設計書:** `docs/superpowers/specs/2026-06-01-urthr-net-review-cleanup-design.md`

---

## ファイル構成（作成・変更マップ）

**Track B 監査レポート（新規）:**
- 作成: `docs/audit/2026-06-01-solana-dev-audit.md` — 監査の所見・重要度・推奨修正を表形式で記録。承認ゲートの入力。

**Track B 修正（承認後・findings-driven）:**
- 変更: `programs/urthr-net/src/instructions/*.rs` / `state/*.rs` / `error.rs` / `constants.rs` / `util.rs`（承認された所見に対応する箇所のみ）
- 変更: `programs/urthr-net/tests/*.rs`（修正を保証する LiteSVM テスト）
- 変更: `app/src/debug/**`（命令変更時の対応パネル）/ `app/src/lib/**`（送信経路ヘルパ）
- 再生成: `app/src/generated/**`（IDL 変更時のみ `pnpm codegen`）

**Track A ドキュメント日本語化:**
- 変更: `docs/ARCHITECTURE.md`（英語→日本語、`--ignore-keys` 記述の訂正）
- 変更: `README.md`（英語→日本語、`--ignore-keys` 記述の訂正）
- 変更: `runbooks/README.md`（英語→日本語、Surfpool 説明文）
- 変更: `app/README.md`（既に日本語。重複行 25–26 の整理のみ）
- 変更: `features.md`（既にほぼ日本語。英語残渣があれば訂正）
- 確認のみ: `docs/debug-harness-runbook.md`（既に全文日本語。用語整合のみ確認）

**対象外:** `app/src/generated/**`、`runbooks/*.tx`、`docs/superpowers/{specs,plans}/**`、`CLAUDE.md`（既に日本語）。

---

## Track B — プログラム監査

### Task 1: オンチェーンプログラム監査（Rust / Anchor）

**Files:**
- 参照: `programs/urthr-net/src/lib.rs`, `programs/urthr-net/src/instructions/*.rs`, `programs/urthr-net/src/state/*.rs`, `error.rs`, `constants.rs`, `util.rs`
- 参照: solana-dev `references/security.md`（観点チェックリスト）
- 作成: `docs/audit/2026-06-01-solana-dev-audit.md`（このタスクで着手）

- [ ] **Step 1: 監査チェックリストを観点ごとに走査**

11 命令それぞれについて、以下の観点を `file:line` 付きで確認しメモする:
- 署名者チェック（`Signer` / `has_one` で各命令の意図した署名者か）
- 所有者・アカウント型検証（`Account<T>` / `token::*` 制約）
- PDA seeds/bump（canonical bump 保存・利用、`seeds`/`bump` 制約）
- 検算（`checked_*` / 128-bit、オーバーフロー時 `MathOverflow`）
- CPI 安全性（token transfer の `with_signer` seeds、`token::mint` 一致）
- アカウントクローズ（`close_campaign` の rent 返却先・復活防止）
- 再初期化（`init` vs `init_if_needed` の誤用有無）
- `paused` 緊急停止の網羅範囲（資金移動命令で `!config.paused`、exit 系は除外の妥当性）
- SPL Token vs Token-2022 前提（`token::TokenAccount` 等の Program 指定）
- エラー型網羅・重複可変アカウント混同

- [ ] **Step 2: 所見をレポートに表形式で記録**

`docs/audit/2026-06-01-solana-dev-audit.md` の「オンチェーン」節に追記する。各行のフォーマット:

```markdown
| ID | レイヤー | file:line | 重要度 | 内容 | 推奨修正 |
|----|---------|-----------|--------|------|----------|
| O-01 | onchain | submit_claim.rs:NN | 高/中/低 | <観察した事実> | <最小修正案> |
```

重要度基準: 高=correctness/security 直結、中=堅牢性・防御的検証、低=スタイル・可読性。
所見ゼロの観点は「OK: <観点>（根拠 file:line）」として明記し、網羅したことを残す。

- [ ] **Step 3: コミット（レポートはまだ未完なら部分コミット可）**

```bash
git add docs/audit/2026-06-01-solana-dev-audit.md
git commit -m "docs(audit): オンチェーンプログラムの solana-dev 監査所見"
```

---

### Task 2: app（kit / フロントエンド）監査

**Files:**
- 参照: `app/src/debug/core/DebugPanel.tsx`, `useInstructionRunner.ts`, `prepareOptions.ts`, `app/src/debug/instructions/*.tsx`, `app/src/lib/{json,txError,rpc}.ts`, `app/src/providers.tsx`
- 変更: `docs/audit/2026-06-01-solana-dev-audit.md`（「app」節を追記）

- [ ] **Step 1: app 観点を走査**

- kit 乖離ヘルパ徹底: 全送信箇所が `runnerPrepareOptions` / `stringifyWithBigInt` / `describeTransactionError` 経由か（素の `JSON.stringify`・`feePayer` 直渡しが無いか）
- 送信前シミュレーション不変条件: 全 `instructions/*.tsx` が `DebugPanel`（`useInstructionRunner`）経由で simulate→承認→送信を守るか（迂回が無いか）
- 生成クライアントと IDL の一致（`pnpm codegen` 差分が出ないか — Step 2 で確認）
- 秘密鍵非保持・既定 cluster = localnet/devnet（`providers.tsx` / `rpc.ts`）

- [ ] **Step 2: 生成クライアントの IDL 整合を確認**

Run:
```bash
cd app && pnpm codegen && git diff --stat app/src/generated
```
Expected: 差分なし（あれば所見 `A-xx` として記録し、コミット前に `git checkout` で戻す — 修正は承認後）。

- [ ] **Step 3: 所見をレポートに記録しコミット**

`docs/audit/2026-06-01-solana-dev-audit.md` の「app」節に Task 1 同様の表形式で追記。所見ゼロ観点は「OK: …」を明記。末尾に「## サマリ」を追加し、重要度別の件数と推奨対応順を記す。

```bash
git add docs/audit/2026-06-01-solana-dev-audit.md
git commit -m "docs(audit): app 送信経路・パネル契約の solana-dev 監査所見"
```

---

### ⛔ 承認ゲート（人間の確認）

監査レポート `docs/audit/2026-06-01-solana-dev-audit.md` をユーザに提示する。ユーザが**修正対象の所見を選定**する。以降の Task 3 は、選定された所見ごとに 1 回ずつ下記テンプレートを適用してインスタンス化する（未選定の所見は実施しない）。

---

### Task 3（テンプレート）: 承認された所見 `<ID>` の修正

> このテンプレートを承認された所見ごとに複製して使う。`<ID>` / `<file>` / 期待挙動を所見内容で置換する。
> ドキュメントのみ・テスト不能なスタイル所見は Step 1–2 を省略し Step 3 のみ実施してよい。

**Files:**
- 変更: `<所見の対象ファイル>`（例 `programs/urthr-net/src/instructions/<ix>.rs`）
- テスト: `programs/urthr-net/tests/<domain>.rs`
- 連動（命令変更時）: `app/src/debug/instructions/<Panel>.tsx`、IDL 変更時 `app/src/generated/**`

- [ ] **Step 1: 失敗するテストを追加（挙動を保証する所見の場合）**

所見が示す不正挙動を再現する、または期待挙動を表明する LiteSVM テストを `programs/urthr-net/tests/<domain>.rs` に追加する。既存ハーネス（`tests/common/mod.rs`）の PDA 導出・mint/token 構築ヘルパを使う。

```rust
// 例: 期待する拒否を表明する形（実際の所見に合わせて置換）
#[test]
fn rejects_<condition_for_ID>() {
    let mut ctx = common::setup();
    // ... 所見の前提条件を構築 ...
    let err = ctx.<call_failing_path>().unwrap_err();
    assert_eq!(common::anchor_err(&err), UrthrError::<ExpectedError> as u32);
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run:
```bash
NO_DNA=1 anchor build && cargo test -p urthr-net rejects_<condition_for_ID>
```
Expected: FAIL（修正前は所見の不正挙動により assert 不一致 / パニック）。

- [ ] **Step 3: 推奨修正を最小実装**

監査レポートの「推奨修正」に従い `<file>` を変更する。例（制約追加・検算修正・seeds 検証など、所見に応じて）。命令シグネチャ／アカウント構造を変えた場合は、CLAUDE.md の DoD に従い対応する `app/src/debug/instructions/<Panel>.tsx`（必要ならアカウントインスペクタ）も更新する。

- [ ] **Step 4: テスト緑とビルドを確認**

Run:
```bash
NO_DNA=1 anchor build && cargo test -p urthr-net
```
Expected: PASS（新テスト含む全 LiteSVM テスト緑）。

- [ ] **Step 5: IDL 変更時はクライアント再生成**

IDL（命令・アカウント・型）を変更した場合のみ:
```bash
cd app && pnpm codegen && git add src/generated
```
変更なしなら本ステップは省略。

- [ ] **Step 6: app 側の検証**

Run:
```bash
cd app && pnpm build && pnpm test && pnpm lint
```
Expected: すべて PASS（生成コードは lint 対象外）。

- [ ] **Step 7: コミット**

```bash
git add programs/urthr-net app docs/audit
git commit -m "fix(<layer>): 所見 <ID> 対応 — <一行要約>"
```

---

## Track A — ドキュメント日本語化

### Task 4: `docs/ARCHITECTURE.md` を日本語化

**Files:**
- 変更: `docs/ARCHITECTURE.md`

- [ ] **Step 1: 散文を日本語へ翻訳**

全英語散文を日本語化する。**コード識別子・ファイルパス・コマンド・型名・命令名・エラー名・表中のフィールド名（`protocol_fee_bps:u16` 等）は英語/monospace のまま**保持。設計書の用語集を適用（publisher=パブリッシャー / advertiser=広告主 / stake=ステーク（担保）/ slash=スラッシュ / claim=クレーム / attestor=アテスター / challenge=チャレンジ / settle=決済 / escrow=エスクロー / treasury=トレジャリ / vault=vault）。図（ASCII ダイアグラム）はラベルを保持し、説明文のみ日本語化。

- [ ] **Step 2: 陳腐化記述を訂正**

`## Testing` 節の `--ignore-keys` 記述（現 138–139 行付近）を削除し、CLAUDE.md の鍵統一に合わせて「鍵と `declare_id!` が一致するため `--ignore-keys` は不要」と修正する。Track B で挙動が変わった箇所（命令・制約・テスト数等）があれば最新化する。

- [ ] **Step 3: 翻訳の網羅とコマンド非破壊を確認**

Run:
```bash
grep -nP '[\x{3040}-\x{30ff}\x{4e00}-\x{9faf}]' docs/ARCHITECTURE.md | wc -l   # 大きく増えていること
grep -n 'ignore-keys' docs/ARCHITECTURE.md                                    # 0 件（訂正済み）
grep -nE 'anchor build|cargo test -p urthr-net' docs/ARCHITECTURE.md          # コマンドが原形で残存
```
Expected: 日本語行が大幅増、`ignore-keys` 0 件、コマンドは英語原形のまま。

- [ ] **Step 4: コミット**

```bash
git add docs/ARCHITECTURE.md
git commit -m "docs: ARCHITECTURE.md を日本語化し --ignore-keys 記述を訂正"
```

---

### Task 5: root `README.md` を日本語化

**Files:**
- 変更: `README.md`

- [ ] **Step 1: 散文を日本語へ翻訳**

見出し・本文・引用ブロック・表の説明列を日本語化。Task 4 と同じ用語集・保持ルールを適用。コードブロック（`surfpool start` / `anchor build` / `pnpm` 各コマンド）と URL・パスは原形保持。既に日本語の手順ラベル（「ウォレット」「localnet」「プロトコル設定」等）はそのまま。

- [ ] **Step 2: 陳腐化記述を訂正**

`## Build & test` の `--ignore-keys` コメント（現 69 行付近）を削除し、CLAUDE.md の鍵統一に合わせて不要である旨に修正。

- [ ] **Step 3: 確認**

Run:
```bash
grep -n 'ignore-keys' README.md                       # 0 件
grep -nE 'NO_DNA=1 anchor build|cargo test -p urthr-net|pnpm (dev|codegen)' README.md  # コマンド原形で残存
```
Expected: `ignore-keys` 0 件、コマンドが英語原形で残る。

- [ ] **Step 4: コミット**

```bash
git add README.md
git commit -m "docs: root README を日本語化し --ignore-keys 記述を訂正"
```

---

### Task 6: `runbooks/README.md` を日本語化

**Files:**
- 変更: `runbooks/README.md`

- [ ] **Step 1: 散文を日本語へ翻訳**

Surfpool 紹介文・各節の説明を日本語化。**インストール／起動コマンド（`curl … | bash`、`surfpool start` 等）、URL、バッジ、`surfpool ls` 出力例は原形保持**。見出し（`## Available Runbooks` 等）は日本語見出しに置換可（例「## 利用可能な Runbook」）。`deployment` 等の Runbook 名は原形のまま。

- [ ] **Step 2: 確認**

Run:
```bash
grep -nP '[\x{3040}-\x{30ff}\x{4e00}-\x{9faf}]' runbooks/README.md | wc -l   # > 0
grep -nE 'surfpool (start|ls|run)' runbooks/README.md                        # コマンド原形で残存
```
Expected: 日本語行あり、コマンド原形で残存。

- [ ] **Step 3: コミット**

```bash
git add runbooks/README.md
git commit -m "docs: runbooks/README を日本語化"
```

---

### Task 7: `app/README.md` の整理 と `features.md` の英語残渣訂正

**Files:**
- 変更: `app/README.md`
- 変更: `features.md`

- [ ] **Step 1: `app/README.md` の重複行を除去**

`app/README.md` は既に全文日本語。重複している箇所（現 25–26 行: 「3.」と「RPC URL に `http://127.0.0.1:8899` を設定」が 2 回）を 1 つに統合する。`## Phantom を localnet に向ける` 節が手順 1→2 の 2 ステップになるよう修正。

- [ ] **Step 2: `features.md` の英語残渣を確認・訂正**

Run:
```bash
grep -nP '^(?!.*[\x{3040}-\x{30ff}\x{4e00}-\x{9faf}]).*[A-Za-z]{4,}' features.md | grep -vE '`|http|^\s*\||^#|^- \[|:.*=' | head -40
```
散文として英語のまま残っている行があれば日本語化する（コード識別子・コマンド・命令名・表のフィールドのみの行は対象外）。残渣が無ければ変更なしでよい。

- [ ] **Step 3: 確認**

Run:
```bash
grep -nc 'RPC URL に' app/README.md     # 1 件（重複解消）
```
Expected: `RPC URL に` が 1 件。

- [ ] **Step 4: コミット**

```bash
git add app/README.md features.md
git commit -m "docs: app/README の重複整理と features.md の表記統一"
```

---

### Task 8: `docs/debug-harness-runbook.md` の用語整合確認

**Files:**
- 確認のみ（必要時のみ変更）: `docs/debug-harness-runbook.md`

- [ ] **Step 1: 用語集との整合を確認**

このファイルは既に全文日本語。設計書の用語集（パブリッシャー / 広告主 / スラッシュ 等）と表記揺れが無いか確認する。揺れがあれば統一、無ければ変更しない。Track B で挙動・命令が変わった場合は該当手順（特に「全命令網羅チェックリスト」表）を最新化する。

- [ ] **Step 2: 変更があればコミット**

```bash
git add docs/debug-harness-runbook.md
git commit -m "docs: debug-harness-runbook の用語統一"
```
変更が無ければ本タスクはコミット不要。

---

## Task 9: 最終検証

**Files:** なし（検証のみ）

- [ ] **Step 1: オンチェーン検証**

Run:
```bash
NO_DNA=1 anchor build && cargo test -p urthr-net
```
Expected: ビルド成功、全 LiteSVM テスト PASS。

- [ ] **Step 2: フロントエンド検証**

Run:
```bash
cd app && pnpm build && pnpm test && pnpm lint
```
Expected: build/test/lint すべて PASS。

- [ ] **Step 3: ドキュメント言語の最終確認**

Run:
```bash
for f in README.md docs/ARCHITECTURE.md runbooks/README.md app/README.md docs/debug-harness-runbook.md features.md; do
  echo "$f: ja=$(grep -cP '[\x{3040}-\x{30ff}\x{4e00}-\x{9faf}]' "$f")"
done
grep -rn 'ignore-keys' README.md docs/ARCHITECTURE.md   # 0 件
```
Expected: 各現役ドキュメントに日本語行が十分あり、`ignore-keys` の陳腐化記述が 0 件。

- [ ] **Step 4: 完了報告**

`docs/audit/2026-06-01-solana-dev-audit.md` の承認済み所見がすべて修正済みであること、現役ドキュメントが日本語統一されたことを確認し、ユーザに最終報告する。

---

## 自己レビュー結果

- **Spec coverage:** Track B 監査（Task 1–2）＋承認ゲート＋修正テンプレート（Task 3）、Track A 全対象ファイル（Task 4–8: ARCHITECTURE / README / runbooks / app README / features / debug-harness-runbook）、最終検証（Task 9）で設計書の全節をカバー。
- **Placeholder scan:** Track B の修正コードは監査前のため未確定だが、これは「監査→承認→修正」フローの本質。Task 3 はテンプレートとして手順・コマンド・受け入れ基準を具体化済み（プレースホルダではなく findings-driven インスタンス化）。
- **Type consistency:** 監査レポートのパス（`docs/audit/2026-06-01-solana-dev-audit.md`）、所見 ID 体系（O-xx / A-xx）、用語集を全タスクで統一。
