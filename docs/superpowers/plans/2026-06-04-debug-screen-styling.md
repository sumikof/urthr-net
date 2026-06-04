# デバッグ画面スタイル刷新＋アドレスコピーボタン Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** UrthrNet の localnet デバッグ画面（`app/`）の見た目を既存 `index.css` のデザイン変数に寄せて統一し、生成 mint アドレスと接続ウォレットアドレスにコピーボタンを追加する。

**Architecture:** 各パネルは共通コア（`DebugPanel` / `TextField` / `AccountInspector`）経由で描画されるため、コア3つ＋localnet 2パネル＋AccountInfo＋App.tsx のインライン style を `index.css` のクラスへ置換するだけで画面全面に効く。パネルの DOM 構造・ロジック・送信前シミュレーション必須の不変条件は変更しない。`CopyButton` を新規追加して mint/ウォレットアドレスに設置する。

**Tech Stack:** React 19、TypeScript、Vite、`@solana/kit` / `@solana/react-hooks`、vitest（純ロジックのみ。コンポーネントテスト基盤は無い）。

**検証方針（重要）:** 本作業は CSS＋JSX マークアップが主体で、現行ハーネスにはコンポーネントテスト基盤（jsdom/testing-library）が無い。spec の非ゴールに従い**テスト基盤は新設しない**。各タスクは次で検証する:
- `cd app && pnpm build`（`tsc -b` で型・参照を検証＋vite ビルド）
- `cd app && pnpm lint`（eslint。`app/src/generated` は対象外）
- `cd app && pnpm test`（既存の純ロジックテストが緑のまま）
- 全タスク完了後に localnet（surfpool）で目視確認。

spec: `docs/superpowers/specs/2026-06-03-debug-screen-styling-design.md`

---

## File Structure

- Modify: `app/src/index.css` — デバッグ画面用クラス群を追記（変数ベース）
- Create: `app/src/components/CopyButton.tsx` — 共通コピーボタン
- Modify: `app/src/debug/core/DebugPanel.tsx` — `box`／ボタン／結果表示を `className` 化
- Modify: `app/src/debug/core/fields.tsx` — `TextField` を `className` 化
- Modify: `app/src/debug/accounts/AccountInspector.tsx` — `box`・結果 `pre` を `className` 化
- Modify: `app/src/debug/localnet/CreateMintPanel.tsx` — 生成 mint アドレスに `CopyButton`
- Modify: `app/src/components/AccountInfo.tsx` — ウォレットアドレスに `CopyButton`
- Modify: `app/src/App.tsx` — レイアウト整合を `className` 化

各タスクは独立して成立し、`pnpm build`／`pnpm lint`／`pnpm test` が緑のまま進む。

---

## Task 1: index.css にデバッグ画面クラスを追加

**Files:**
- Modify: `app/src/index.css`（末尾に追記）

- [ ] **Step 1: クラス群を index.css の末尾に追記**

`app/src/index.css` の最終行の後ろに以下をそのまま追加する（既存の CSS 変数のみ使用。状態色 green/crimson は可読性優先で固定色据え置き）:

```css

/* ===== debug screen ===== */

.app {
  max-width: 920px;
  margin: 0 auto;
  padding: 24px 20px 64px;
  text-align: left;
}
.app section {
  margin-top: 28px;
}
.app section > h2 {
  border-bottom: 1px solid var(--border);
  padding-bottom: 6px;
}
.app-intro {
  color: var(--text);
}

.panel {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px 20px;
  margin: 16px 0;
  background: var(--bg);
  box-shadow: var(--shadow);
  text-align: left;
}
.panel > legend {
  font-weight: 600;
  color: var(--text-h);
  padding: 0 6px;
}

.field {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 8px 0;
}
.field > .field-label {
  min-width: 180px;
  color: var(--text);
}
.field > input {
  flex: 1;
  max-width: 380px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text-h);
  font: inherit;
}
.field > input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-bg);
}
.field-error {
  color: crimson;
  font-size: 0.9em;
}

.btn {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text-h);
  font: inherit;
  cursor: pointer;
  transition:
    background 0.12s,
    border-color 0.12s;
}
.btn:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--accent-bg);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-primary {
  border-color: var(--accent-border);
  background: var(--accent-bg);
  color: var(--accent);
}
.btn-primary:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.btn-copy {
  font-size: 0.8em;
  padding: 2px 8px;
}
.panel-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.result {
  margin-top: 12px;
  font-size: 0.9em;
}
.result-ok {
  color: green;
}
.result-err {
  color: crimson;
}
.result pre,
.inspector-json {
  background: var(--code-bg);
  border-radius: 6px;
  padding: 10px;
  font: 0.8em var(--mono);
  white-space: pre-wrap;
  word-break: break-all;
}
.addr {
  font: 0.9em var(--mono);
  background: var(--code-bg);
  padding: 2px 6px;
  border-radius: 4px;
  word-break: break-all;
}
.addr-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
```

- [ ] **Step 2: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: いずれも成功（CSS 追記のみ。型・lint に影響なし）。

- [ ] **Step 3: Commit**

```bash
git add app/src/index.css
git commit -m "style(app): デバッグ画面用クラスを index.css に追加 (変数ベース)"
```

---

## Task 2: CopyButton コンポーネントを新規作成

**Files:**
- Create: `app/src/components/CopyButton.tsx`

- [ ] **Step 1: CopyButton を作成**

`app/src/components/CopyButton.tsx` を新規作成し、以下をそのまま書く:

```tsx
import { useState } from "react";

export type CopyButtonProps = Readonly<{
  /** クリップボードへ書き込む実値（アドレス等）。 */
  value: string;
  /** ボタン表示ラベル。既定は「コピー」。 */
  label?: string;
}>;

/**
 * 値をクリップボードへコピーする小さなボタン。成功すると 1.2 秒だけ
 * 「✓ コピー済」に変化し自動で戻る。クリップボード不可（古いブラウザや
 * 非セキュアコンテキスト）では例外を握りつぶす（テスト画面のため致命的でない）。
 */
export function CopyButton({ value, label = "コピー" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      // clipboard 不可環境では無反応
    }
  }

  return (
    <button
      type="button"
      className="btn btn-copy"
      onClick={() => void onCopy()}
      title={value}
      aria-label={`${label}: ${value}`}
    >
      {copied ? "✓ コピー済" : label}
    </button>
  );
}
```

- [ ] **Step 2: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功（未使用なので type チェックのみ通ればよい）。

- [ ] **Step 3: Commit**

```bash
git add app/src/components/CopyButton.tsx
git commit -m "feat(app): アドレス用 CopyButton コンポーネントを追加"
```

---

## Task 3: DebugPanel をクラス化

**Files:**
- Modify: `app/src/debug/core/DebugPanel.tsx`

- [ ] **Step 1: `box` 定数を削除し JSX を className 化**

`app/src/debug/core/DebugPanel.tsx` で、まず `box` 定数の宣言ブロックを削除する。削除対象（コメント含む）:

```tsx
const box: React.CSSProperties = {
  border: "1px solid #ccc",
  borderRadius: 6,
  padding: "0.75rem 1rem",
  margin: "0.75rem 0",
};
```

- [ ] **Step 2: `return` 内の各要素を className に置換**

`return (...)` ブロック全体を以下に置換する（構造・ロジックは不変。`style` を `className` に置換しただけ）:

```tsx
  return (
    <fieldset className="panel">
      <legend>{title}</legend>

      {children}

      <div className="panel-actions">
        <button
          type="button"
          className="btn"
          onClick={() => void runner.simulate(build)}
          disabled={disabled || runner.isSimulating}
        >
          {runner.isSimulating ? "シミュレート中…" : "シミュレート"}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void runner.send()}
          disabled={disabled || !runner.canSend || runner.isSending}
        >
          {runner.isSending ? "送信中…" : "承認して送信"}
        </button>
      </div>

      {summary && (
        <div className="result">
          <div className={summary.err ? "result-err" : "result-ok"}>
            {summary.err ? "シミュレーション: 失敗" : "シミュレーション: 成功"}
            {summary.unitsConsumed != null && (
              <span style={{ marginLeft: 8, color: "var(--text)" }}>
                CU: {summary.unitsConsumed.toString()}
              </span>
            )}
          </div>
          {summary.logs && summary.logs.length > 0 && (
            <details style={{ marginTop: 4 }}>
              <summary>ログ ({summary.logs.length})</summary>
              <pre>{summary.logs.join("\n")}</pre>
            </details>
          )}
        </div>
      )}

      {signature && phase === "sent" && (
        <p className="result result-ok" style={{ wordBreak: "break-all" }}>
          署名: {signature}
        </p>
      )}

      {error && phase === "error" && (
        <p
          className="result result-err"
          style={{ wordBreak: "break-all", whiteSpace: "pre-wrap" }}
        >
          エラー: {error}
        </p>
      )}

      {(phase === "sent" || phase === "error") && (
        <div style={{ marginTop: "0.5rem" }}>
          <button type="button" className="btn" onClick={() => runner.reset()}>
            リセット
          </button>
        </div>
      )}
    </fieldset>
  );
```

- [ ] **Step 3: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功。`box` 未使用エラーが出ないこと（削除済みのため）。

- [ ] **Step 4: Commit**

```bash
git add app/src/debug/core/DebugPanel.tsx
git commit -m "style(app): DebugPanel を index.css クラスに移行"
```

---

## Task 4: TextField をクラス化

**Files:**
- Modify: `app/src/debug/core/fields.tsx`

- [ ] **Step 1: `TextField` の JSX を className 化**

`app/src/debug/core/fields.tsx` の `TextField` 関数の `return` を以下に置換する（ロジック・props は不変。インライン style を class に置換）:

```tsx
  return (
    <label className="field">
      <span className="field-label">{props.label}</span>
      <input
        value={props.value}
        placeholder={props.placeholder}
        onChange={(e) => props.onChange(e.target.value)}
      />
      {props.error && <span className="field-error">{props.error}</span>}
    </label>
  );
```

- [ ] **Step 2: ビルド・lint・既存テストで検証**

Run: `cd app && pnpm build && pnpm lint && pnpm test`
Expected: すべて成功。`fields.test.ts`（パーサのテスト）は緑のまま（パーサ関数は未変更）。

- [ ] **Step 3: Commit**

```bash
git add app/src/debug/core/fields.tsx
git commit -m "style(app): TextField を index.css クラスに移行"
```

---

## Task 5: AccountInspector をクラス化

**Files:**
- Modify: `app/src/debug/accounts/AccountInspector.tsx`

- [ ] **Step 1: `box` 定数を削除**

`app/src/debug/accounts/AccountInspector.tsx` の以下の宣言ブロックを削除する:

```tsx
const box: React.CSSProperties = {
  border: "1px solid #ccc",
  borderRadius: 6,
  padding: "0.75rem 1rem",
  margin: "0.75rem 0",
};
```

- [ ] **Step 2: `return` 内を className 化**

`return (...)` ブロックを以下に置換する（構造・ロジックは不変）:

```tsx
  return (
    <fieldset className="panel">
      <legend>{title}</legend>

      {children}

      <TextField
        label="アドレス"
        value={addr}
        onChange={setAddr}
        error={addr && !parsed.ok ? parsed.error : undefined}
      />

      <div className="panel-actions">
        <button
          type="button"
          className="btn"
          onClick={() => void onFetch()}
          disabled={!parsed.ok || state.kind === "loading"}
        >
          {state.kind === "loading" ? "取得中…" : "取得"}
        </button>
      </div>

      {state.kind === "missing" && <p style={{ marginTop: "0.5rem" }}>未作成</p>}

      {state.kind === "ok" && <pre className="inspector-json">{state.json}</pre>}

      {state.kind === "error" && (
        <p className="result result-err" style={{ wordBreak: "break-all" }}>
          デコード不可: {state.message}
        </p>
      )}
    </fieldset>
  );
```

- [ ] **Step 3: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功。`box` 未使用エラーが出ないこと。

- [ ] **Step 4: Commit**

```bash
git add app/src/debug/accounts/AccountInspector.tsx
git commit -m "style(app): AccountInspector を index.css クラスに移行"
```

---

## Task 6: CreateMintPanel に CopyButton を設置

**Files:**
- Modify: `app/src/debug/localnet/CreateMintPanel.tsx`

- [ ] **Step 1: CopyButton を import**

`app/src/debug/localnet/CreateMintPanel.tsx` の import 群の末尾（`import { TOKEN_PROGRAM_ADDRESS } ...` の次の行）に追加:

```tsx
import { CopyButton } from "../../components/CopyButton";
```

- [ ] **Step 2: 生成 mint アドレス表示を addr＋CopyButton に置換**

現状の以下のブロック:

```tsx
        {mintSigner && (
          <span style={{ marginLeft: 8, wordBreak: "break-all", fontSize: "0.9em" }}>
            mint: {mintSigner.address}
          </span>
        )}
```

を次に置換する:

```tsx
        {mintSigner && (
          <span className="addr-row" style={{ marginLeft: 8 }}>
            <span>mint:</span>
            <code className="addr">{mintSigner.address}</code>
            <CopyButton value={mintSigner.address} />
          </span>
        )}
```

- [ ] **Step 3: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功。`CopyButton` が使用されていること（未使用 import エラーが出ない）。

- [ ] **Step 4: Commit**

```bash
git add app/src/debug/localnet/CreateMintPanel.tsx
git commit -m "feat(app): 生成 mint アドレスにコピーボタンを追加"
```

---

## Task 7: AccountInfo に CopyButton を設置

**Files:**
- Modify: `app/src/components/AccountInfo.tsx`

- [ ] **Step 1: CopyButton を import**

`app/src/components/AccountInfo.tsx` の import 群の末尾（`import { formatSol } ...` の次の行）に追加:

```tsx
import { CopyButton } from "./CopyButton";
```

- [ ] **Step 2: アドレス行を addr＋CopyButton に置換**

現状の以下の行:

```tsx
      <p>アドレス: {address}</p>
```

を次に置換する:

```tsx
      <p className="addr-row">
        <span>アドレス:</span>
        <code className="addr">{address}</code>
        <CopyButton value={address} />
      </p>
```

- [ ] **Step 3: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功。

- [ ] **Step 4: Commit**

```bash
git add app/src/components/AccountInfo.tsx
git commit -m "feat(app): ウォレットアドレスにコピーボタンを追加"
```

---

## Task 8: App.tsx のレイアウトをクラス化

**Files:**
- Modify: `app/src/App.tsx`

- [ ] **Step 1: `<main>` と intro を className 化**

`app/src/App.tsx` の以下:

```tsx
    <main style={{ maxWidth: 880, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <h1>urthr-net — デバッグハーネス</h1>
      <p style={{ color: "#666" }}>
        localnet 用デバッグ画面。各命令をシミュレートしてから送信します。
      </p>
```

を次に置換する（`text-align: left` を持つ `.app` で `#root` の中央寄せに勝たせる）:

```tsx
    <main className="app">
      <h1>urthr-net — デバッグハーネス</h1>
      <p className="app-intro">
        localnet 用デバッグ画面。各命令をシミュレートしてから送信します。
      </p>
```

- [ ] **Step 2: ビルドと lint で検証**

Run: `cd app && pnpm build && pnpm lint`
Expected: 成功。

- [ ] **Step 3: Commit**

```bash
git add app/src/App.tsx
git commit -m "style(app): App レイアウトを index.css クラスに移行"
```

---

## Task 9: 最終検証（ビルド・テスト・lint・localnet 目視）

**Files:** なし（検証のみ）

- [ ] **Step 1: フルゲートを通す**

Run: `cd app && pnpm build && pnpm test && pnpm lint`
Expected: すべて成功。生成コード（`app/src/generated`）は lint 対象外。

- [ ] **Step 2: localnet で目視確認**

surfpool（localnet）でアプリを起動し、以下を確認する:
- 各パネルが新スタイル（角丸枠・影・入力フォーカスリング・ボタン主従）で表示される。
- 「シミュレート」が標準ボタン、「承認して送信」が紫の primary ボタンで表示される。
- mint 作成 → 生成 mint の「コピー」ボタン押下で「✓ コピー済」に変化 → mintTo パネルへ貼り付けできる。
- ウォレットアドレスの「コピー」ボタンが動作する。
- OS のダークモード切替で配色が破綻しない（変数経由のため自動追従）。

- [ ] **Step 3: ブランチ完了処理**

実装完了後、`superpowers:finishing-a-development-branch` で main への統合方法（merge / PR）を決める。

---

## Self-Review

**1. Spec coverage:**
- 見た目を index.css 変数に統一 → Task 1（クラス定義）＋ Task 3〜5,8（コア/レイアウトの適用）。
- 生成 mint アドレスのコピー → Task 6。
- ウォレットアドレスのコピー → Task 7。
- CopyButton 仕様（1.2 秒トグル・例外握りつぶし・title/aria-label・全長表示） → Task 2。
- レイアウト整合（#root 中央寄せに勝つ `.app`） → Task 1（`.app`）＋ Task 8。
- 送信前シミュレーション必須の不変条件維持 → Task 3 はロジック不変、style のみ置換。
- 非ゴール（署名/JSON 内アドレスのコピー、状態色変数化、テスト基盤新設、個別パネル手入れ）には対応するタスクを作らない。すべてカバー済み。

**2. Placeholder scan:** TBD/TODO/「後で」なし。各コード手順に完全なコードを記載済み。

**3. Type/命名整合:** `CopyButton` の props（`value` 必須・`label` 任意）は Task 2 の定義と Task 6/7 の使用（`value` のみ渡す）で一致。クラス名（`.panel`/`.field`/`.field-label`/`.btn`/`.btn-primary`/`.btn-copy`/`.panel-actions`/`.result`/`.result-ok`/`.result-err`/`.inspector-json`/`.addr`/`.addr-row`/`.app`/`.app-intro`）は Task 1 の定義と各適用タスクの使用で一致。
