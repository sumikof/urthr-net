# urthr-net ウォレット接続 初期構築 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. 実装中は `solana-dev` スキルを参照すること。

**Goal:** Anchorプログラム `urthr_net` をSurfpool localnetにデプロイし、Vite+React Webから wallet-standard ウォレットを接続・Airdrop・`initialize` 命令実行までできるようにする。

**Architecture:** 既存Anchorプログラムはそのまま。Surfpool(surfnet)を `127.0.0.1:8899` で起動し `runbooks/deployment` でデプロイ。Webは `app/` 配下の独立pnpmプロジェクトで、framework-kit (`@solana/client` + `@solana/react-hooks`) によるwallet-standard接続、Codama生成クライアントによる `initialize` 命令送信を行う。

**Tech Stack:** Anchor 1.0.2 / Surfpool 1.2.1 / Vite + React + TypeScript / `@solana/client` / `@solana/react-hooks` / `@solana/kit` / Codama / Vitest

---

## 確定事項（実装前提）

- Program ID: `3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR`（`declare_id!`/`Anchor.toml`/`target/deploy/urthr_net-keypair.json`/IDL すべて一致済み）。
- IDL: `target/idl/urthr_net.json`。`initialize` 命令は accounts/args 共に空。
- ビルド成果物 `target/deploy/urthr_net.so` は既に存在（再ビルド可）。
- 既存テスト `programs/urthr-net/tests/test_initialize.rs`（LiteSVM）は維持。
- localnet RPC: `http://127.0.0.1:8899`、WS: `ws://127.0.0.1:8900`。
- agentがCLIを叩く際は `NO_DNA=1` を前置する。

## ファイル構成

| パス | 責務 |
|---|---|
| `programs/urthr-net/` | 既存プログラム（変更なし） |
| `app/` | Vite+React SPA（独立pnpmプロジェクト） |
| `app/.env` | `VITE_RPC_URL` / `VITE_WS_URL` |
| `app/scripts/codegen.mjs` | IDL→Codama TSクライアント生成スクリプト |
| `app/src/generated/` | Codama生成クライアント（`getInitializeInstruction` 等） |
| `app/src/lib/format.ts` | 純粋関数（`formatSol`, `shortenAddress`） |
| `app/src/lib/format.test.ts` | `format.ts` のVitestテスト |
| `app/src/lib/rpc.ts` | RPCエンドポイント定数 + Airdrop用kit RPC生成 |
| `app/src/providers.tsx` | `createClient` + `SolanaProvider` |
| `app/src/components/ConnectButton.tsx` | ウォレット接続/切断 |
| `app/src/components/AccountInfo.tsx` | アドレス + SOL残高表示 |
| `app/src/components/AirdropButton.tsx` | localnet Airdrop |
| `app/src/components/InitializeButton.tsx` | `initialize` 命令の構築→署名→送信 |
| `app/src/App.tsx` | 上記コンポーネントの配置 |
| `app/README.md` | 起動・デプロイ・Phantom localnet設定手順 |

---

## Task 1: プログラムのビルドとIDL生成

**Files:**
- 確認のみ: `target/idl/urthr_net.json`, `target/deploy/urthr_net.so`

- [ ] **Step 1: Program IDの整合を確認（idempotent）**

Run: `NO_DNA=1 anchor keys sync`
Expected: 変更なし（IDは既に一致）。差分が出た場合はそのままコミット対象とする。

- [ ] **Step 2: ビルドしてIDLを生成**

Run: `NO_DNA=1 anchor build`
Expected: ビルド成功。`target/idl/urthr_net.json` と `target/deploy/urthr_net.so` が更新される。

- [ ] **Step 3: IDLに `initialize`（accounts/args 空）が含まれることを確認**

Run: `cat target/idl/urthr_net.json`
Expected: `"instructions"` に `"name": "initialize"`、`"accounts": []`、`"args": []` が存在。

- [ ] **Step 4: 既存のLiteSVMテストがグリーンであることを確認**

Run: `cargo test -p urthr-net`
Expected: `test_initialize ... ok`。

- [ ] **Step 5: コミット（変更があった場合のみ）**

```bash
git add -A
git commit -m "build: rebuild urthr_net program and IDL" || echo "no changes to commit"
```

---

## Task 2: Surfpool localnet へのデプロイ検証

**Files:**
- 既存: `txtx.yml`, `runbooks/deployment/main.tx`, `runbooks/deployment/signers.localnet.tx`

- [ ] **Step 1: surfnetをバックグラウンド起動（自動デプロイ）**

Run: `NO_DNA=1 surfpool start --ci` をバックグラウンドで起動する（実行ツールのバックグラウンド機能を使用）。surfpoolは `txtx.yml` を検出し `deployment` runbookを自動実行する（`--no-deploy` 未指定のため自動デプロイ有効）。
代替（明示デプロイ）: surfnet起動後に別シェルで `NO_DNA=1 surfpool run deployment --unsupervised --output-json ./outputs/` を実行。
Expected: surfnetが `http://127.0.0.1:8899` で待受開始。

- [ ] **Step 2: RPCが応答することを確認**

Run:
```bash
curl -s -X POST http://127.0.0.1:8899 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```
Expected: `{"jsonrpc":"2.0","result":"ok","id":1}`。

- [ ] **Step 3: プログラムがデプロイされたことを確認**

Run:
```bash
curl -s -X POST http://127.0.0.1:8899 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR",{"encoding":"base64"}]}'
```
Expected: `result.value` が `null` ではなく、`executable: true` のアカウントが返る。

- [ ] **Step 4: デプロイ手順を確認（次タスク以降はsurfnetを起動したまま進める）**

surfnetは開発中は起動したままにする。停止する場合はバックグラウンドプロセスを終了する。

---

## Task 3: Vite + React + TypeScript アプリのscaffold

**Files:**
- Create: `app/`（Viteテンプレート一式）
- Modify: `.gitignore`

- [ ] **Step 1: Viteプロジェクトを生成**

Run: `pnpm create vite@latest app -- --template react-ts`
Expected: `app/` に `package.json`, `index.html`, `src/`, `vite.config.ts`, `tsconfig.json` が生成される。

- [ ] **Step 2: 依存をインストール**

Run:
```bash
cd app && pnpm install && cd ..
```
Expected: `app/node_modules` が作成される。

- [ ] **Step 3: framework-kit / kit / Codama / Vitest を追加**

Run:
```bash
cd app && \
pnpm add @solana/client @solana/react-hooks @solana/kit && \
pnpm add -D vitest codama @codama/nodes-from-anchor @codama/renderers-js && \
cd ..
```
Expected: `app/package.json` の dependencies / devDependencies に各パッケージが追加される。

- [ ] **Step 4: `.gitignore` に app の生成物を追加**

`/.gitignore` に以下を追記（既存行は残す）:
```
app/node_modules
app/dist
app/.env
```

- [ ] **Step 5: 開発サーバが起動することを確認して停止**

Run: `cd app && timeout 8 pnpm dev || true; cd ..`
Expected: `Local: http://localhost:5173/` が表示される（timeoutで自動停止）。

- [ ] **Step 6: コミット**

```bash
git add app .gitignore
git commit -m "feat: scaffold vite react app with framework-kit deps"
```

---

## Task 4: Codama によるクライアント生成

**Files:**
- Create: `app/scripts/codegen.mjs`
- Modify: `app/package.json`（scripts）
- Create（生成物）: `app/src/generated/`

- [ ] **Step 1: codegenスクリプトを作成**

Create `app/scripts/codegen.mjs`:
```js
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";

const here = dirname(fileURLToPath(import.meta.url));
const idlPath = resolve(here, "../../target/idl/urthr_net.json");
const outDir = resolve(here, "../src/generated");

const anchorIdl = JSON.parse(readFileSync(idlPath, "utf-8"));
const codama = createFromRoot(rootNodeFromAnchor(anchorIdl));
codama.accept(renderVisitor(outDir));

console.log(`Generated client at ${outDir}`);
```

- [ ] **Step 2: `app/package.json` の scripts に codegen を追加**

`app/package.json` の `"scripts"` に以下を追加:
```json
"codegen": "node scripts/codegen.mjs"
```

- [ ] **Step 3: クライアントを生成**

Run: `cd app && pnpm codegen && cd ..`
Expected: `Generated client at .../app/src/generated` と表示され、`app/src/generated/` 配下に `instructions/`, `programs/`, `index.ts` 等が生成される。

- [ ] **Step 4: `getInitializeInstruction` がエクスポートされていることを確認**

Run: `grep -r "getInitializeInstruction" app/src/generated | head`
Expected: `instructions/initialize.ts` に `export function getInitializeInstruction` が存在。

- [ ] **Step 5: コミット**

```bash
git add app/scripts/codegen.mjs app/package.json app/src/generated
git commit -m "feat: generate codama client from urthr_net IDL"
```

---

## Task 5: 純粋ユーティリティ関数（TDD）

**Files:**
- Create: `app/src/lib/format.ts`
- Test: `app/src/lib/format.test.ts`
- Modify: `app/package.json`（test script）

- [ ] **Step 1: test script を追加**

`app/package.json` の `"scripts"` に追加:
```json
"test": "vitest run"
```

- [ ] **Step 2: 失敗するテストを書く**

Create `app/src/lib/format.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { formatSol, shortenAddress } from "./format";

describe("formatSol", () => {
  it("converts lamports (bigint) to SOL string with up to 9 decimals", () => {
    expect(formatSol(1_000_000_000n)).toBe("1");
    expect(formatSol(1_500_000_000n)).toBe("1.5");
    expect(formatSol(0n)).toBe("0");
  });

  it("accepts number input", () => {
    expect(formatSol(2_000_000_000)).toBe("2");
  });
});

describe("shortenAddress", () => {
  it("keeps first 4 and last 4 chars joined by an ellipsis", () => {
    expect(shortenAddress("3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR")).toBe("3CmD…RPoR");
  });

  it("returns the input unchanged when shorter than 9 chars", () => {
    expect(shortenAddress("abc")).toBe("abc");
  });
});
```

- [ ] **Step 3: テストを実行して失敗を確認**

Run: `cd app && pnpm test src/lib/format.test.ts; cd ..`
Expected: FAIL（`./format` が存在しない / 関数未定義）。

- [ ] **Step 4: 最小実装を書く**

Create `app/src/lib/format.ts`:
```ts
const LAMPORTS_PER_SOL = 1_000_000_000n;

export function formatSol(lamports: bigint | number): string {
  const value = typeof lamports === "bigint" ? lamports : BigInt(Math.trunc(lamports));
  const whole = value / LAMPORTS_PER_SOL;
  const frac = value % LAMPORTS_PER_SOL;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(9, "0").replace(/0+$/, "");
  return `${whole.toString()}.${fracStr}`;
}

export function shortenAddress(address: string): string {
  if (address.length < 9) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}
```

- [ ] **Step 5: テストを実行して成功を確認**

Run: `cd app && pnpm test src/lib/format.test.ts; cd ..`
Expected: PASS（4 tests）。

- [ ] **Step 6: コミット**

```bash
git add app/src/lib/format.ts app/src/lib/format.test.ts app/package.json
git commit -m "feat: add formatSol and shortenAddress utils with tests"
```

---

## Task 6: RPC定数 + Provider セットアップ

**Files:**
- Create: `app/.env`
- Create: `app/src/lib/rpc.ts`
- Create: `app/src/providers.tsx`

- [ ] **Step 1: `.env` を作成**

Create `app/.env`:
```
VITE_RPC_URL=http://127.0.0.1:8899
VITE_WS_URL=ws://127.0.0.1:8900
```

- [ ] **Step 2: RPC定数 + Airdrop用 kit RPC を作成**

Create `app/src/lib/rpc.ts`:
```ts
import { address, createSolanaRpc, lamports, type Address } from "@solana/kit";

export const RPC_URL = import.meta.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
export const WS_URL = import.meta.env.VITE_WS_URL ?? "ws://127.0.0.1:8900";

export async function requestAirdrop(recipient: string, sol: number): Promise<string> {
  const rpc = createSolanaRpc(RPC_URL);
  const recipientAddress: Address = address(recipient);
  const signature = await rpc
    .requestAirdrop(recipientAddress, lamports(BigInt(sol) * 1_000_000_000n), {
      commitment: "confirmed",
    })
    .send();
  return signature;
}
```

- [ ] **Step 3: Provider を作成**

Create `app/src/providers.tsx`:
```tsx
import React from "react";
import { autoDiscover, createClient } from "@solana/client";
import { SolanaProvider } from "@solana/react-hooks";
import { RPC_URL, WS_URL } from "./lib/rpc";

export const solanaClient = createClient({
  endpoint: RPC_URL,
  websocketEndpoint: WS_URL,
  walletConnectors: autoDiscover(),
});

export function Providers({ children }: { children: React.ReactNode }) {
  return <SolanaProvider client={solanaClient}>{children}</SolanaProvider>;
}
```

- [ ] **Step 4: 型チェックが通ることを確認**

Run: `cd app && pnpm exec tsc --noEmit; cd ..`
Expected: エラーなし。（プロパティ名が型と異なる場合は、インストール済みパッケージの型定義に合わせて修正する。）

- [ ] **Step 5: コミット**

```bash
git add app/src/lib/rpc.ts app/src/providers.tsx
git commit -m "feat: add rpc constants, airdrop helper, and solana provider"
```

---

## Task 7: ConnectButton コンポーネント

**Files:**
- Create: `app/src/components/ConnectButton.tsx`

- [ ] **Step 1: ConnectButton を作成**

Create `app/src/components/ConnectButton.tsx`:
```tsx
import { useWalletConnection } from "@solana/react-hooks";
import { shortenAddress } from "../lib/format";

export function ConnectButton() {
  const { connectors, connect, disconnect, wallet, status } = useWalletConnection();
  const address = wallet?.account.address;

  if (status === "connected" && address) {
    return (
      <div>
        <span>接続中: {shortenAddress(address)}</span>
        <button onClick={() => disconnect()}>切断</button>
      </div>
    );
  }

  if (connectors.length === 0) {
    return <p>wallet-standard対応ウォレット（Phantom等）が見つかりません。拡張機能をインストールしてください。</p>;
  }

  return (
    <div>
      {connectors.map((connector) => (
        <button key={connector.id} onClick={() => connect(connector.id)}>
          {connector.name} で接続
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: 型チェックを確認**

Run: `cd app && pnpm exec tsc --noEmit; cd ..`
Expected: エラーなし。（`connector.id` / `connector.name` / `wallet.account.address` が型と異なる場合は型定義に合わせて修正。）

- [ ] **Step 3: コミット**

```bash
git add app/src/components/ConnectButton.tsx
git commit -m "feat: add ConnectButton component"
```

---

## Task 8: AccountInfo コンポーネント（アドレス + 残高）

**Files:**
- Create: `app/src/components/AccountInfo.tsx`

- [ ] **Step 1: AccountInfo を作成**

Create `app/src/components/AccountInfo.tsx`:
```tsx
import { useBalance, useWalletConnection } from "@solana/react-hooks";
import { formatSol } from "../lib/format";

export function AccountInfo() {
  const { wallet, status } = useWalletConnection();
  const address = wallet?.account.address;
  const { lamports, fetching } = useBalance(address);

  if (status !== "connected" || !address) {
    return null;
  }

  return (
    <div>
      <p>アドレス: {address}</p>
      <p>残高: {fetching && lamports == null ? "取得中…" : `${formatSol(lamports ?? 0n)} SOL`}</p>
    </div>
  );
}
```

- [ ] **Step 2: 型チェックを確認**

Run: `cd app && pnpm exec tsc --noEmit; cd ..`
Expected: エラーなし。（`useBalance` が `undefined` を受け付けない場合は、`address` がある時のみ描画する内側コンポーネントへ分割する。）

- [ ] **Step 3: コミット**

```bash
git add app/src/components/AccountInfo.tsx
git commit -m "feat: add AccountInfo component showing address and balance"
```

---

## Task 9: AirdropButton コンポーネント

**Files:**
- Create: `app/src/components/AirdropButton.tsx`

- [ ] **Step 1: AirdropButton を作成**

Create `app/src/components/AirdropButton.tsx`:
```tsx
import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import { requestAirdrop } from "../lib/rpc";

export function AirdropButton() {
  const { wallet, status } = useWalletConnection();
  const address = wallet?.account.address;
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  if (status !== "connected" || !address) {
    return null;
  }

  async function onAirdrop() {
    if (!address) return;
    setBusy(true);
    setMessage(null);
    try {
      const sig = await requestAirdrop(address, 2);
      setMessage(`Airdrop成功: ${sig}`);
    } catch (err) {
      setMessage(`Airdrop失敗: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <button onClick={onAirdrop} disabled={busy}>
        {busy ? "Airdrop中…" : "2 SOL Airdrop (localnet)"}
      </button>
      {message && <p>{message}</p>}
    </div>
  );
}
```

- [ ] **Step 2: 型チェックを確認**

Run: `cd app && pnpm exec tsc --noEmit; cd ..`
Expected: エラーなし。

- [ ] **Step 3: コミット**

```bash
git add app/src/components/AirdropButton.tsx
git commit -m "feat: add AirdropButton for localnet funding"
```

---

## Task 10: InitializeButton コンポーネント（命令送信）

**Files:**
- Create: `app/src/components/InitializeButton.tsx`

- [ ] **Step 1: InitializeButton を作成**

Create `app/src/components/InitializeButton.tsx`:
```tsx
import { useState } from "react";
import { useTransactionPool, useWalletConnection } from "@solana/react-hooks";
import { getInitializeInstruction } from "../generated";

export function InitializeButton() {
  const { wallet, status } = useWalletConnection();
  const { addInstruction, clearInstructions, prepareAndSend, isSending } = useTransactionPool();
  const [signature, setSignature] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (status !== "connected" || !wallet) {
    return null;
  }

  async function onInitialize() {
    setSignature(null);
    setError(null);
    try {
      clearInstructions();
      addInstruction(getInitializeInstruction());
      const result = await prepareAndSend({ authority: wallet });
      setSignature(typeof result === "string" ? result : (result?.signature ?? "送信完了"));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div>
      <button onClick={onInitialize} disabled={isSending}>
        {isSending ? "送信中…" : "initialize を実行"}
      </button>
      {signature && <p>署名: {signature}</p>}
      {error && <p>エラー: {error}</p>}
    </div>
  );
}
```

- [ ] **Step 2: `getInitializeInstruction` のインポート元を確認**

Run: `grep -rn "export.*getInitializeInstruction" app/src/generated`
Expected: `app/src/generated/index.ts` 経由でエクスポートされている。違う場合はインポートパスを生成物に合わせて修正する。

- [ ] **Step 3: 型チェックを確認**

Run: `cd app && pnpm exec tsc --noEmit; cd ..`
Expected: エラーなし。（`prepareAndSend` の戻り値型・`authority` 引数が型と異なる場合は型定義に合わせて修正。）

- [ ] **Step 4: コミット**

```bash
git add app/src/components/InitializeButton.tsx
git commit -m "feat: add InitializeButton to send initialize instruction"
```

---

## Task 11: App 組み立て + README

**Files:**
- Modify: `app/src/App.tsx`
- Modify: `app/src/main.tsx`
- Create: `app/README.md`

- [ ] **Step 1: `main.tsx` を Providers でラップ**

`app/src/main.tsx` を以下に置き換える（既存のimport/StrictModeは維持）:
```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Providers } from "./providers";
import App from "./App.tsx";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Providers>
      <App />
    </Providers>
  </StrictMode>,
);
```

- [ ] **Step 2: `App.tsx` を置き換える**

`app/src/App.tsx`:
```tsx
import { ConnectButton } from "./components/ConnectButton";
import { AccountInfo } from "./components/AccountInfo";
import { AirdropButton } from "./components/AirdropButton";
import { InitializeButton } from "./components/InitializeButton";

function App() {
  return (
    <main style={{ maxWidth: 640, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <h1>urthr-net</h1>
      <section>
        <h2>ウォレット</h2>
        <ConnectButton />
        <AccountInfo />
      </section>
      <section>
        <h2>localnet</h2>
        <AirdropButton />
      </section>
      <section>
        <h2>プログラム</h2>
        <InitializeButton />
      </section>
    </main>
  );
}

export default App;
```

- [ ] **Step 3: 型チェック + ビルドを確認**

Run: `cd app && pnpm exec tsc --noEmit && pnpm build; cd ..`
Expected: 型エラーなし、`app/dist` にビルド成功。

- [ ] **Step 4: README を作成**

Create `app/README.md`:
```markdown
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
```

- [ ] **Step 5: コミット**

```bash
git add app/src/App.tsx app/src/main.tsx app/README.md
git commit -m "feat: wire up app shell and add localnet README"
```

---

## Task 12: E2E 手動検証

**Files:** なし（検証のみ）

- [ ] **Step 1: surfnet が起動していることを確認**

Run: `curl -s -X POST http://127.0.0.1:8899 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'`
Expected: `"result":"ok"`。

- [ ] **Step 2: dev サーバを起動**

Run: `cd app && pnpm dev`（フォアグラウンドで起動したまま、ブラウザで検証）
Expected: http://localhost:5173 が表示。

- [ ] **Step 3: ブラウザで一連を確認**

手順:
1. PhantomをCustom RPC `http://127.0.0.1:8899` に設定。
2. 「接続」→ アドレスが短縮表示される。
3. 「2 SOL Airdrop」→ 成功メッセージ、残高が `2 SOL` 付近に更新。
4. 「initialize を実行」→ Phantomで署名 → 署名(signature)が表示される。

Expected: 4まで成功し、署名が表示される。

- [ ] **Step 4: 送信した initialize トランザクションが成立したことを確認**

ブラウザに表示された署名を `<SIG>` として:
```bash
curl -s -X POST http://127.0.0.1:8899 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":["<SIG>",{"encoding":"json","maxSupportedTransactionVersion":0}]}'
```
Expected: `result.meta.err` が `null`。

- [ ] **Step 5: 最終確認のメモを残す（必要なら）**

検証結果（成功/失敗、署名）を作業ログに記録する。

---

## 完了条件（DoD）

- [ ] `anchor build` 成功、IDL生成、`cargo test -p urthr-net` グリーン
- [ ] surfnet にプログラムがデプロイされ、`getAccountInfo` で executable アカウントが返る
- [ ] `app/` がビルド成功（`pnpm build`）し、`pnpm test` グリーン
- [ ] ブラウザで「接続 → Airdrop → initialize 実行 → 署名表示」までE2Eで成功
- [ ] `app/README.md` に起動・Phantom localnet設定手順を記載
