# UrthrNet デバッグ・ハーネス 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 既存 `app/` を、UrthrNet 全11命令を叩け・4アカウントを覗ける localnet デバッグ画面に拡張し、「各機能はデバッグパネルを備えて完了」という開発ルールを `CLAUDE.md`/`features.md` に確立する。

**Architecture:** framework-kit（`@solana/client`/`@solana/react-hooks`/`@solana/kit`）。再利用可能な `DebugPanel` + `useInstructionRunner`（送信前シミュレーション必須）+ 入力部品 + PDA導出を `app/src/debug/core/` に置き、命令ごとに薄い宣言的パネル（`app/src/debug/instructions/`）、アカウントごとにインスペクタ（`app/src/debug/accounts/`）、localnet mint ヘルパー（`app/src/debug/localnet/`）を実装。`App.tsx` をドメイン別セクションに再編。

**Tech Stack:** React 19 + Vite + TypeScript、`@solana/kit`、`@solana/client`、`@solana/react-hooks`、生成クライアント（codama）、localnet 用に `@solana-program/token` + `@solana-program/system`、テストは vitest。

**参照:** spec = `docs/superpowers/specs/2026-05-31-urthr-net-debug-harness-design.md`。コマンドは原則 `cd /workspace/app` で実行。

---

## File Structure

```
app/src/
  generated/                 # 再生成（11命令ビルダー + 4デコーダ）
  debug/
    core/
      pdas.ts  pdas.test.ts
      fields.tsx  fields.test.ts
      useInstructionRunner.ts
      DebugPanel.tsx
      programs.ts            # token/system/rent の定数アドレス集約
    instructions/            # 11パネル
    accounts/                # 4インスペクタ + 共通 AccountInspector
    localnet/                # CreateMintPanel, MintToPanel
  App.tsx                    # 再編
CLAUDE.md                    # 新規・開発ルール
features.md                  # 受け入れ条件フォーマットにルール追記
```

---

## Task 1: 依存追加とクライアント再生成

**Files:**
- Modify: `app/package.json`
- Regenerate: `app/src/generated/**`
- Delete: `app/src/components/InitializeButton.tsx`（旧 `getInitializeInstruction` を使い再生成で壊れるため。新 `InitializeProtocolPanel` が後続Taskで置換）
- Modify: `app/src/App.tsx`（一時的に `InitializeButton` の import/使用を削除し、ビルドを通す）

- [ ] **Step 1: token/system プログラムプラグインを追加**

Run:
```bash
cd /workspace/app && pnpm add @solana-program/token @solana-program/system
```
Expected: `package.json` の dependencies に両者が追加される。

- [ ] **Step 2: 最新IDLからクライアント再生成**

Run:
```bash
cd /workspace/app && pnpm codegen
```
Expected: `Generated client at .../src/generated`。`app/src/generated/instructions/` に11命令ファイルが生成される。

- [ ] **Step 3: 11命令が揃ったことを確認**

Run:
```bash
cd /workspace/app && ls src/generated/instructions/ && grep -rl "export function get" src/generated/instructions/ | wc -l
```
Expected: `initializeProtocol`,`registerPublisher`,`stake`,`unstake`,`createCampaign`,`fundCampaign`,`submitClaim`,`challengeClaim`,`resolveClaim`,`settleClaim`,`closeCampaign`（＋index）。命令ファイルが11個。
（生成される正確な関数名・アカウント引数名は `src/generated/instructions/*.ts` を開いて確認し、後続Taskで使用する。）

- [ ] **Step 4: 壊れる旧コンポーネントを削除し App.tsx を一時修正**

`app/src/components/InitializeButton.tsx` を削除。`app/src/App.tsx` から `InitializeButton` の import と `<InitializeButton />` を削除（「プログラム」セクションは後続で差し替えるので一旦空でよい）。

- [ ] **Step 5: ビルドが通ることを確認**

Run: `cd /workspace/app && pnpm build`
Expected: tsc + vite ビルド成功（未使用 import 等のエラーなし）。

- [ ] **Step 6: Commit**

```bash
cd /workspace && git add app/package.json app/pnpm-lock.yaml app/src/generated app/src/App.tsx && git rm app/src/components/InitializeButton.tsx && git commit -m "chore(app): regenerate client for 11 instructions, add token/system plugins"
```

---

## Task 2: PDA 導出（`debug/core/pdas.ts`）— TDD

オンチェーン seed（`programs/urthr-net/src/constants.rs`）のミラー。導出は `@solana/kit` の `getProgramDerivedAddress`。プログラムIDは生成クライアントの定数（`src/generated/programs/` の `URTHR_NET_PROGRAM_ADDRESS` 等。正確名は開いて確認）。

**Files:**
- Create: `app/src/debug/core/pdas.ts`
- Test: `app/src/debug/core/pdas.test.ts`

- [ ] **Step 1: 失敗するテストを書く**

```ts
// app/src/debug/core/pdas.test.ts
import { describe, it, expect } from "vitest";
import { address } from "@solana/kit";
import { configPda, treasuryPda, publisherPda, stakeVaultPda, campaignPda, escrowVaultPda, claimPda } from "./pdas";

const ADV = address("11111111111111111111111111111112");

describe("pdas", () => {
  it("config/treasury are stable, valid base58 addresses", async () => {
    const c = await configPda();
    const t = await treasuryPda();
    expect(typeof c).toBe("string");
    expect(c).not.toBe(t);
  });
  it("campaign derivation depends on advertiser + id", async () => {
    const a = await campaignPda(ADV, 1n);
    const b = await campaignPda(ADV, 2n);
    expect(a).not.toBe(b);
  });
  it("escrow/claim derive from the campaign pda", async () => {
    const camp = await campaignPda(ADV, 1n);
    const e = await escrowVaultPda(camp);
    const k = await claimPda(camp, 0n);
    expect(e).not.toBe(k);
  });
});
```

- [ ] **Step 2: 失敗を確認**

Run: `cd /workspace/app && pnpm test src/debug/core/pdas.test.ts`
Expected: FAIL（`./pdas` 未実装）。

- [ ] **Step 3: 実装**

```ts
// app/src/debug/core/pdas.ts
import { getProgramDerivedAddress, getAddressEncoder, type Address } from "@solana/kit";
import { URTHR_NET_PROGRAM_ADDRESS } from "../../generated"; // ← 正確名を generated で確認

const enc = getAddressEncoder();
const u64le = (n: bigint) => {
  const b = new Uint8Array(8);
  new DataView(b.buffer).setBigUint64(0, n, true);
  return b;
};
const utf8 = (s: string) => new TextEncoder().encode(s);

async function pda(seeds: (Uint8Array)[]): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({ programAddress: URTHR_NET_PROGRAM_ADDRESS, seeds });
  return addr;
}

export const configPda = () => pda([utf8("config")]);
export const treasuryPda = () => pda([utf8("treasury")]);
export const publisherPda = (authority: Address) => pda([utf8("publisher"), enc.encode(authority)]);
export const stakeVaultPda = (authority: Address) => pda([utf8("stake_vault"), enc.encode(authority)]);
export const campaignPda = (advertiser: Address, campaignId: bigint) =>
  pda([utf8("campaign"), enc.encode(advertiser), u64le(campaignId)]);
export const escrowVaultPda = (campaign: Address) => pda([utf8("escrow_vault"), enc.encode(campaign)]);
export const claimPda = (campaign: Address, claimNonce: bigint) =>
  pda([utf8("claim"), enc.encode(campaign), u64le(claimNonce)]);
```

注: seed prefix は `constants.rs` と一致（config/treasury/publisher/stake_vault/campaign/escrow_vault/claim）。`publisher` と `stake_vault` はともに **authority（ウォレット）** でキー。`escrow_vault`/`claim` は **campaign PDA** でキー。

- [ ] **Step 4: テスト通過を確認**

Run: `cd /workspace/app && pnpm test src/debug/core/pdas.test.ts`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
cd /workspace && git add app/src/debug/core/pdas.ts app/src/debug/core/pdas.test.ts && git commit -m "feat(app): PDA derivation helpers for debug harness"
```

---

## Task 3: 入力部品とパーサ（`debug/core/fields.tsx`）— TDD

引数のパース/検証（pubkey / u64 / u16 / bool / bytes32 hex）と、対応する React 入力部品。

**Files:**
- Create: `app/src/debug/core/fields.tsx`
- Test: `app/src/debug/core/fields.test.ts`

- [ ] **Step 1: 失敗するテストを書く（パーサ）**

```ts
// app/src/debug/core/fields.test.ts
import { describe, it, expect } from "vitest";
import { parseU64, parseBytes32Hex, parsePubkey } from "./fields";

describe("field parsers", () => {
  it("parseU64 accepts range, rejects negatives and overflow", () => {
    expect(parseU64("0")).toEqual({ ok: true, value: 0n });
    expect(parseU64("1000000")).toEqual({ ok: true, value: 1000000n });
    expect(parseU64("-1").ok).toBe(false);
    expect(parseU64("notanumber").ok).toBe(false);
    expect(parseU64((2n ** 64n).toString()).ok).toBe(false);
  });
  it("parseBytes32Hex requires 32 bytes", () => {
    expect(parseBytes32Hex("0x" + "00".repeat(32)).ok).toBe(true);
    expect(parseBytes32Hex("00".repeat(32)).ok).toBe(true);
    expect(parseBytes32Hex("00".repeat(31)).ok).toBe(false);
    expect(parseBytes32Hex("zz".repeat(32)).ok).toBe(false);
  });
  it("parsePubkey validates base58 address", () => {
    expect(parsePubkey("11111111111111111111111111111112").ok).toBe(true);
    expect(parsePubkey("not-an-address").ok).toBe(false);
  });
});
```

- [ ] **Step 2: 失敗を確認**

Run: `cd /workspace/app && pnpm test src/debug/core/fields.test.ts`
Expected: FAIL。

- [ ] **Step 3: 実装**

```tsx
// app/src/debug/core/fields.tsx
import { useState } from "react";
import { address, type Address } from "@solana/kit";

export type Parsed<T> = { ok: true; value: T } | { ok: false; error: string };

export function parseU64(s: string): Parsed<bigint> {
  if (!/^\d+$/.test(s.trim())) return { ok: false, error: "正の整数を入力" };
  const v = BigInt(s.trim());
  if (v > 2n ** 64n - 1n) return { ok: false, error: "u64 範囲外" };
  return { ok: true, value: v };
}
export function parseU16(s: string): Parsed<number> {
  const r = parseU64(s);
  if (!r.ok) return r;
  if (r.value > 65535n) return { ok: false, error: "u16 範囲外" };
  return { ok: true, value: Number(r.value) };
}
export function parseBytes32Hex(s: string): Parsed<Uint8Array> {
  const hex = s.trim().replace(/^0x/i, "");
  if (!/^[0-9a-fA-F]{64}$/.test(hex)) return { ok: false, error: "32バイトのhex(64桁)を入力" };
  const out = new Uint8Array(32);
  for (let i = 0; i < 32; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return { ok: true, value: out };
}
export function parsePubkey(s: string): Parsed<Address> {
  try { return { ok: true, value: address(s.trim()) }; }
  catch { return { ok: false, error: "不正なアドレス" }; }
}

/** ラベル付きテキスト入力。値とエラーを呼び出し側に伝える。 */
export function TextField(props: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string; error?: string;
}) {
  return (
    <label style={{ display: "block", margin: "0.25rem 0" }}>
      <span style={{ display: "inline-block", minWidth: 180 }}>{props.label}</span>
      <input value={props.value} placeholder={props.placeholder}
        onChange={(e) => props.onChange(e.target.value)} style={{ width: 360 }} />
      {props.error && <span style={{ color: "crimson", marginLeft: 8 }}>{props.error}</span>}
    </label>
  );
}

/** 制御された文字列入力フック（パネルの実装を短くする補助）。 */
export function useField(initial = "") {
  const [value, setValue] = useState(initial);
  return { value, setValue };
}
```

- [ ] **Step 4: テスト通過を確認**

Run: `cd /workspace/app && pnpm test src/debug/core/fields.test.ts`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
cd /workspace && git add app/src/debug/core/fields.tsx app/src/debug/core/fields.test.ts && git commit -m "feat(app): typed field inputs + parsers for debug harness"
```

---

## Task 4: 実行ランナーと DebugPanel（`debug/core/`）

送信前シミュレーション→要約表示→明示承認→送信。`useTransactionPool`（`prepare`/`toWire`/`prepareAndSend`）と `useSolanaClient().runtime.rpc.simulateTransaction` を使用。プログラム定数は `programs.ts` に集約。

**Files:**
- Create: `app/src/debug/core/programs.ts`
- Create: `app/src/debug/core/useInstructionRunner.ts`
- Create: `app/src/debug/core/DebugPanel.tsx`

- [ ] **Step 1: プログラム定数**

```ts
// app/src/debug/core/programs.ts
import { TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";
import { address } from "@solana/kit";
export { TOKEN_PROGRAM_ADDRESS, SYSTEM_PROGRAM_ADDRESS };
export const RENT_SYSVAR_ADDRESS = address("SysvarRent111111111111111111111111111111111");
```

- [ ] **Step 2: 実行ランナー**

`build` で組んだ命令をプールに入れて prepare → wire(base64) を取り、`simulateTransaction` を実行して要約を返す。承認後に `prepareAndSend(signers)` で送信。
（`prepare`/`toWire`/`prepareAndSend` と `transactionToBase64` の正確なシグネチャは `node_modules/@solana/react-hooks/dist/types/hooks.d.ts` と `@solana/client` の `transactions/base64` を参照して合わせること。シミュレーションは `client.runtime.rpc.simulateTransaction(wireBase64, { encoding: "base64", sigVerify: false, replaceRecentBlockhash: true }).send()`。）

```ts
// app/src/debug/core/useInstructionRunner.ts
import { useState } from "react";
import { useSolanaClient, useTransactionPool, useWalletConnection } from "@solana/react-hooks";

export type SimSummary = { err: unknown; unitsConsumed?: bigint | null; logs: string[] };
type Phase = "idle" | "simulated" | "sent" | "error";

export function useInstructionRunner() {
  const { wallet } = useWalletConnection();
  const client = useSolanaClient();
  const pool = useTransactionPool();
  const [phase, setPhase] = useState<Phase>("idle");
  const [summary, setSummary] = useState<SimSummary | null>(null);
  const [signature, setSignature] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function simulate(build: () => Promise<unknown> | unknown) {
    setError(null); setSignature(null); setSummary(null);
    try {
      pool.clearInstructions();
      pool.addInstruction((await build()) as never);
      const prepared = await pool.prepare({ feePayer: wallet } as never);
      const wire = await pool.toWire({} as never); // base64 wire（実シグネチャに合わせる）
      const res = await client.runtime.rpc
        .simulateTransaction(wire as never, { encoding: "base64", sigVerify: false, replaceRecentBlockhash: true } as never)
        .send();
      const v = (res as { value: { err: unknown; unitsConsumed?: bigint | null; logs: string[] | null } }).value;
      setSummary({ err: v.err, unitsConsumed: v.unitsConsumed ?? null, logs: v.logs ?? [] });
      setPhase(v.err ? "error" : "simulated");
      void prepared;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e)); setPhase("error");
    }
  }

  async function send(signers: Record<string, unknown>) {
    setError(null);
    try {
      const sig = await pool.prepareAndSend(signers as never);
      setSignature(String(sig)); setPhase("sent");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e)); setPhase("error");
    }
  }

  return { phase, summary, signature, error, simulate, send, isSending: pool.isSending };
}
```

注: framework-kit の `prepare`/`toWire` の正確な使い方は型定義で確認。要は「**送信前に必ず simulate し、その要約（err/logs/units）を UI に出す**」「**承認クリックで初めて send**」の2点を満たすこと。`prepare`/`toWire` が困難なら、`build()` した命令を `client.transaction` ヘルパや `prepareTransaction` でラップして base64 化してもよい。

- [ ] **Step 3: DebugPanel**

```tsx
// app/src/debug/core/DebugPanel.tsx
import type { ReactNode } from "react";
import { useInstructionRunner } from "./useInstructionRunner";

export function DebugPanel(props: {
  title: string;
  children: ReactNode;                       // 引数・アカウント入力
  build: () => Promise<unknown> | unknown;   // 命令を返す
  signers: () => Record<string, unknown>;    // 例: () => ({ admin: wallet })
  disabled?: boolean;                        // 入力検証エラー時 true
}) {
  const r = useInstructionRunner();
  return (
    <fieldset style={{ margin: "0.75rem 0", padding: "0.75rem" }}>
      <legend><strong>{props.title}</strong></legend>
      {props.children}
      <div style={{ marginTop: 8 }}>
        <button disabled={props.disabled} onClick={() => r.simulate(props.build)}>シミュレート</button>
        <button disabled={props.disabled || r.phase !== "simulated"} onClick={() => r.send(props.signers())}
          style={{ marginLeft: 8 }}>{r.isSending ? "送信中…" : "承認して送信"}</button>
      </div>
      {r.summary && (
        <div style={{ marginTop: 8, fontFamily: "monospace", fontSize: 12 }}>
          <div>simulate: {r.summary.err ? `エラー: ${JSON.stringify(r.summary.err)}` : "OK"}（CU: {String(r.summary.unitsConsumed)}）</div>
          <details><summary>logs</summary><pre>{r.summary.logs.join("\n")}</pre></details>
        </div>
      )}
      {r.signature && <p style={{ color: "green" }}>署名: {r.signature}</p>}
      {r.error && <p style={{ color: "crimson" }}>エラー: {r.error}</p>}
    </fieldset>
  );
}
```

- [ ] **Step 4: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功（このTaskではまだ画面に未配線でよいが、型エラーがないこと）。

- [ ] **Step 5: Commit**

```bash
cd /workspace && git add app/src/debug/core/programs.ts app/src/debug/core/useInstructionRunner.ts app/src/debug/core/DebugPanel.tsx && git commit -m "feat(app): simulate-before-send runner + reusable DebugPanel"
```

---

## Task 5: localnet ヘルパー（mint 作成 / トークン付与）

`initialize_protocol` 以降は SPL mint と残高が必要。`@solana-program/token`/`@solana-program/system` + `@solana/kit` の `generateKeyPairSigner` を使い、mint 作成・自分への mint を行う。送信は同じ「シミュ→承認→送信」フローに乗せる（DebugPanel を再利用、または専用ボタンで `useInstructionRunner` を利用）。

**Files:**
- Create: `app/src/debug/localnet/CreateMintPanel.tsx`
- Create: `app/src/debug/localnet/MintToPanel.tsx`

- [ ] **Step 1: CreateMintPanel**

要件: `generateKeyPairSigner()` で mint 鍵を生成 → `getCreateAccountInstruction`（system、space=`getMintSize()`、owner=token program、lamports=rent 免除額は `client.runtime.rpc.getMinimumBalanceForRentExemption(...)`）→ `getInitializeMintInstruction`（decimals 入力、mintAuthority=接続ウォレット）。両命令を一度に build。生成 mint アドレスを画面に表示し、コピーできるようにする。mint 鍵は ad-hoc signer として命令に埋め込む（Kit は埋め込み signer を prepare 時に回収）。fee payer は接続ウォレット。
正確な関数名（`getCreateAccountInstruction`/`getInitializeMintInstruction`/`getMintSize`）は `node_modules/@solana-program/token` と `@solana-program/system` の型定義で確認。

- [ ] **Step 2: MintToPanel**

要件: 入力＝mint アドレス・付与量。`findAssociatedTokenPda`/`getCreateAssociatedTokenIdempotentInstruction`（owner=接続ウォレット）＋ `getMintToInstruction`（mintAuthority=接続ウォレット）を build。実行後、`client.splToken({ mint }).fetchBalance(wallet)` で残高を表示。

- [ ] **Step 3: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功。

- [ ] **Step 4: Commit**

```bash
cd /workspace && git add app/src/debug/localnet && git commit -m "feat(app): localnet mint + mint-to helpers"
```

---

## Task 6: アカウントインスペクタ（4種）

任意アドレス or 導出PDAでアカウントを取得し、生成デコーダで復号して表示。`@solana/react-hooks` の `useAccount`（または `client.runtime.rpc.getAccountInfo`）で取得し、生成された `fetch*`/`decode*`（例 `fetchProtocolConfig` / `getProtocolConfigDecoder`、正確名は `src/generated/accounts/` で確認）で復号。

**Files:**
- Create: `app/src/debug/accounts/AccountInspector.tsx`（共通: アドレス入力＋取得＋JSON表示＋再取得）
- Create: `app/src/debug/accounts/ProtocolConfigInspector.tsx`
- Create: `app/src/debug/accounts/PublisherInspector.tsx`
- Create: `app/src/debug/accounts/CampaignInspector.tsx`
- Create: `app/src/debug/accounts/ClaimInspector.tsx`

- [ ] **Step 1: 共通 AccountInspector**

要件: props＝`title`・`defaultAddress?`（導出PDAをデフォルト表示）・`decode: (data) => unknown`。アドレス入力欄＋「取得」ボタン＋結果を `JSON.stringify(_, bigint→string, 2)` で表示。取得失敗（未作成）は「未作成 / デコード不可」を表示。オンチェーンデータは untrusted: 取得後、生成デコーダ（discriminator 検証付き）に通してから表示する。

- [ ] **Step 2: 4インスペクタ**

各々が `AccountInspector` を使い、対応する生成デコーダと既定PDAを渡す:
- ProtocolConfig: 既定 = `configPda()`。
- Publisher: 入力 authority → `publisherPda(authority)` を既定に。
- Campaign: 入力 advertiser + campaign_id → `campaignPda(...)`。
- Claim: 入力 campaign + claim_nonce → `claimPda(...)`。

- [ ] **Step 3: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功。

- [ ] **Step 4: Commit**

```bash
cd /workspace && git add app/src/debug/accounts && git commit -m "feat(app): account inspectors (config/publisher/campaign/claim)"
```

---

## 命令パネル共通仕様（Task 7〜9 で使用）

各パネルは `DebugPanel` を使い、`build()` で生成命令ビルダーを呼ぶ。アカウントの解決規則:

- **signer 役**（admin/advertiser/authority/publisher_authority/challenger/attestor）: 接続ウォレットを充てる。`signers()` は `{ <役名>: wallet }` を返す（複数役は同一ウォレットで可。`prepareAndSend` の request キーは命令の signer アカウント名）。
- **PDA**（config/treasury/publisher/stake_vault/campaign/escrow_vault/claim）: `pdas.ts` で導出して渡す。読み取り専用表示＋手動上書き可。
- **ATA**（authority_token_account/advertiser_token_account/publisher_token_account）: `client.splToken({ mint }).deriveAssociatedTokenAddress(owner)` で導出（owner は該当 authority）。
- **payment_mint**: ユーザ入力（localnet で作成した mint）。
- **token_program/system_program/rent**: `programs.ts` の定数。生成ビルダーが既定値を持つ場合は省略可（生成コードを確認）。

各パネルは入力検証エラー時に `DebugPanel` の `disabled` を真にする。下表が各パネルの「完全な仕様」（args とアカウント種別）。命令関数名・アカウント引数名は生成コードで最終確認すること。

| パネル | 命令 | args | signer | PDA | ATA | mint入力 |
|---|---|---|---|---|---|---|
| InitializeProtocolPanel | initialize_protocol | attestor:pubkey, protocol_fee_bps:u16, min_publisher_stake:u64, challenge_window:u64 | admin | config, treasury | — | payment_mint |
| RegisterPublisherPanel | register_publisher | metadata:bytes32 | authority | config, publisher, stake_vault | — | payment_mint |
| StakePanel | stake | amount:u64 | authority | config, publisher | stake_vault(※), authority_token_account | payment_mint |
| UnstakePanel | unstake | amount:u64 | authority | config, publisher | stake_vault(※), authority_token_account | payment_mint |
| CreateCampaignPanel | create_campaign | campaign_id:u64, price_per_event:u64 | advertiser | config, campaign, escrow_vault | — | payment_mint |
| FundCampaignPanel | fund_campaign | amount:u64 | advertiser | config, campaign | escrow_vault(※), advertiser_token_account | payment_mint |
| CloseCampaignPanel | close_campaign | （なし） | advertiser | config, campaign | escrow_vault(※), advertiser_token_account | payment_mint |
| SubmitClaimPanel | submit_claim | event_count:u64, merkle_root:bytes32 | publisher_authority | config, publisher, campaign, claim | — | — |
| ChallengeClaimPanel | challenge_claim | evidence_hash:bytes32 | challenger | claim | — | — |
| ResolveClaimPanel | resolve_claim | fraud:bool | attestor | config, campaign, claim, escrow_vault, treasury, publisher, stake_vault | publisher_token_account | payment_mint |
| SettleClaimPanel | settle_claim | （なし） | （なし／permissionless、fee payer=wallet） | config, campaign, claim, escrow_vault, treasury, publisher | publisher_token_account | payment_mint |

(※) `stake_vault`/`escrow_vault` はトークン vault だが PDA（`pdas.ts` で導出）。「ATA」列はそれと別の、ユーザ／パブリッシャーの通常トークンアカウント。

SubmitClaim の `claim` PDA は **`claimPda(campaignPda, campaign.claims_count)`**。`claims_count` は CampaignInspector で確認するか、対象 campaign を取得して読む（パネル内で `fetchCampaign` し現在値を使う）。Challenge/Resolve/Settle の `claim` は対象の `claim_nonce` 入力から `claimPda` を導出。

---

## Task 7: 命令パネル — プロトコル設定 & パブリッシャー

**Files:**
- Create: `app/src/debug/instructions/InitializeProtocolPanel.tsx`
- Create: `app/src/debug/instructions/RegisterPublisherPanel.tsx`
- Create: `app/src/debug/instructions/StakePanel.tsx`
- Create: `app/src/debug/instructions/UnstakePanel.tsx`

- [ ] **Step 1: InitializeProtocolPanel を実装（リファレンス実装）**

以下を雛形とし、他パネルは「命令共通仕様」表に従って同型で実装する:

```tsx
// app/src/debug/instructions/InitializeProtocolPanel.tsx
import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU16, parseU64 } from "../core/fields";
import { configPda, treasuryPda } from "../core/pdas";
import { TOKEN_PROGRAM_ADDRESS, SYSTEM_PROGRAM_ADDRESS, RENT_SYSVAR_ADDRESS } from "../core/programs";
import { getInitializeProtocolInstruction } from "../../generated"; // ← 正確名を確認

export function InitializeProtocolPanel() {
  const { wallet, status } = useWalletConnection();
  const [attestor, setAttestor] = useState("");
  const [feeBps, setFeeBps] = useState("50");
  const [minStake, setMinStake] = useState("1000000");
  const [window_, setWindow] = useState("60");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAtt = parsePubkey(attestor), pFee = parseU16(feeBps), pMin = parseU64(minStake), pWin = parseU64(window_), pMint = parsePubkey(mint);
  const disabled = !(pAtt.ok && pFee.ok && pMin.ok && pWin.ok && pMint.ok);

  return (
    <DebugPanel title="initialize_protocol" disabled={disabled}
      signers={() => ({ admin: wallet })}
      build={async () => {
        const config = await configPda(); const treasury = await treasuryPda();
        return getInitializeProtocolInstruction({
          admin: wallet,             // signer（framework-kit が wallet を signer に解決）
          config, treasury,
          paymentMint: (pMint as { value: unknown }).value,
          tokenProgram: TOKEN_PROGRAM_ADDRESS, systemProgram: SYSTEM_PROGRAM_ADDRESS, rent: RENT_SYSVAR_ADDRESS,
          attestor: (pAtt as { value: unknown }).value,
          protocolFeeBps: (pFee as { value: number }).value,
          minPublisherStake: (pMin as { value: bigint }).value,
          challengeWindow: (pWin as { value: bigint }).value,
        } as never);
      }}>
      <TextField label="attestor (pubkey)" value={attestor} onChange={setAttestor} error={!pAtt.ok ? pAtt.error : undefined} />
      <TextField label="protocol_fee_bps (u16)" value={feeBps} onChange={setFeeBps} error={!pFee.ok ? pFee.error : undefined} />
      <TextField label="min_publisher_stake (u64)" value={minStake} onChange={setMinStake} error={!pMin.ok ? pMin.error : undefined} />
      <TextField label="challenge_window (u64 秒)" value={window_} onChange={setWindow} error={!pWin.ok ? pWin.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
```

注: 生成ビルダーの引数名（`paymentMint` 等のキャメル化、signer 引数が `TransactionSigner` を要求するか `Address` か）は生成コードで確認し合わせること。signer は接続ウォレット（`wallet`）を渡す。

- [ ] **Step 2: RegisterPublisher / Stake / Unstake を同型で実装**

「命令共通仕様」表どおり。ATA は `await client.splToken({ mint }).deriveAssociatedTokenAddress(wallet.account.address)`（`useSolanaClient()` から `client` 取得）。Stake/Unstake の `stake_vault` は `stakeVaultPda(wallet.account.address)`。

- [ ] **Step 3: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功。

- [ ] **Step 4: Commit**

```bash
cd /workspace && git add app/src/debug/instructions && git commit -m "feat(app): instruction panels — protocol setup & publisher"
```

---

## Task 8: 命令パネル — キャンペーン

**Files:**
- Create: `app/src/debug/instructions/CreateCampaignPanel.tsx`
- Create: `app/src/debug/instructions/FundCampaignPanel.tsx`
- Create: `app/src/debug/instructions/CloseCampaignPanel.tsx`

- [ ] **Step 1: 3パネルを「命令共通仕様」表どおりに実装**

- `campaign = campaignPda(wallet.account.address, campaignId)`、`escrow_vault = escrowVaultPda(campaign)`。
- Fund/Close の `advertiser_token_account = client.splToken({ mint }).deriveAssociatedTokenAddress(wallet.account.address)`。
- CreateCampaign は `price_per_event > 0` をクライアント側でも検証（0 はオンチェーンで `InvalidPrice`）。

- [ ] **Step 2: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功。

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add app/src/debug/instructions && git commit -m "feat(app): instruction panels — campaign lifecycle"
```

---

## Task 9: 命令パネル — Claim ライフサイクル

**Files:**
- Create: `app/src/debug/instructions/SubmitClaimPanel.tsx`
- Create: `app/src/debug/instructions/ChallengeClaimPanel.tsx`
- Create: `app/src/debug/instructions/ResolveClaimPanel.tsx`
- Create: `app/src/debug/instructions/SettleClaimPanel.tsx`

- [ ] **Step 1: 4パネルを実装**

- SubmitClaim: 入力＝対象 campaign の advertiser + campaign_id + event_count + merkle_root。`campaign = campaignPda(advertiser, id)`、現在の `claims_count` を取得（`fetchCampaign(client.runtime.rpc, campaign)` 等、生成 fetch を使用）し `claim = claimPda(campaign, claims_count)`。signer = publisher_authority(=wallet)。`publisher = publisherPda(wallet.account.address)`。
- ChallengeClaim: 入力＝campaign(advertiser+id) + claim_nonce + evidence_hash。`claim = claimPda(campaignPda(...), nonce)`。signer = challenger(=wallet)。
- ResolveClaim: 入力＝対象特定（advertiser+id+claim_nonce）＋ publisher authority + mint + fraud:bool。PDA一式 + `publisher_token_account = ATA(publisherAuthority, mint)`。signer = attestor(=wallet)。
- SettleClaim: 入力＝対象特定＋ publisher authority + mint。permissionless（signer 役なし、fee payer=wallet）。`signers()` は `{}`（fee payer は prepare の feePayer=wallet で供給）。

- [ ] **Step 2: 型チェック**

Run: `cd /workspace/app && pnpm build`
Expected: 成功。

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add app/src/debug/instructions && git commit -m "feat(app): instruction panels — claim lifecycle"
```

---

## Task 10: App.tsx 再編（全パネル配線）

**Files:**
- Modify: `app/src/App.tsx`

- [ ] **Step 1: ドメイン別セクションで全パネルを配置**

```tsx
import { ConnectButton } from "./components/ConnectButton";
import { AccountInfo } from "./components/AccountInfo";
import { AirdropButton } from "./components/AirdropButton";
import { CreateMintPanel } from "./debug/localnet/CreateMintPanel";
import { MintToPanel } from "./debug/localnet/MintToPanel";
import { InitializeProtocolPanel } from "./debug/instructions/InitializeProtocolPanel";
import { ProtocolConfigInspector } from "./debug/accounts/ProtocolConfigInspector";
// …(register/stake/unstake/create/fund/close/submit/challenge/resolve/settle + publisher/campaign/claim inspectors)

function App() {
  return (
    <main style={{ maxWidth: 880, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <h1>urthr-net — デバッグハーネス</h1>
      <section><h2>ウォレット</h2><ConnectButton /><AccountInfo /></section>
      <section><h2>localnet</h2><AirdropButton /><CreateMintPanel /><MintToPanel /></section>
      <section><h2>プロトコル設定</h2><InitializeProtocolPanel /><ProtocolConfigInspector /></section>
      <section><h2>パブリッシャー</h2>{/* Register/Stake/Unstake + PublisherInspector */}</section>
      <section><h2>キャンペーン</h2>{/* Create/Fund/Close + CampaignInspector */}</section>
      <section><h2>Claim ライフサイクル</h2>{/* Submit/Challenge/Resolve/Settle + ClaimInspector */}</section>
    </main>
  );
}
export default App;
```

全11命令パネル・4インスペクタ・2 localnetヘルパーを import して配置する。

- [ ] **Step 2: ビルド & テスト**

Run: `cd /workspace/app && pnpm build && pnpm test && pnpm lint`
Expected: ビルド成功、vitest 緑（core テスト）、lint クリーン。

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add app/src/App.tsx && git commit -m "feat(app): wire full debug harness into App.tsx"
```

---

## Task 11: 開発ルールを CLAUDE.md に明記（新規）

**Files:**
- Create: `CLAUDE.md`（リポジトリ直下）

- [ ] **Step 1: CLAUDE.md を作成**

内容（要旨。spec の文面に準拠）:
- **デバッグパネル必須ルール:** オンチェーン命令を追加/変更する機能は、完了の定義として `app/src/debug/instructions/` に対応パネルを追加/更新し、`App.tsx` に配線し、localnet で手動実行確認する。新アカウント型は `app/src/debug/accounts/` にインスペクタも追加。パネルは `DebugPanel` 契約（型付き引数 / アカウント入力＋PDA自動導出 / **送信前シミュレーション必須** / 結果表示）に従う。
- **クライアント再生成:** IDL 変更時は `cd app && pnpm codegen`。
- **ビルド/テストの正:** `NO_DNA=1 anchor build --ignore-keys` でビルド、`cargo test -p urthr-net`（LiteSVM 27本）が correctness の正ゲート。`tests/lifecycle.mjs` は surfnet スモークのみ。フロントは `cd app && pnpm build && pnpm test`。
- **安全:** 既定 cluster は localnet/devnet。mainnet 非対象。秘密鍵は保存しない。送信前シミュレーション必須。

- [ ] **Step 2: Commit**

```bash
cd /workspace && git add CLAUDE.md && git commit -m "docs: add CLAUDE.md with mandatory debug-panel development rule"
```

---

## Task 12: features.md にルールを反映

**Files:**
- Modify: `features.md`

- [ ] **Step 1: 「各項目のフォーマット」節に一文追記**

受け入れ条件の説明に次を追加:
> **デバッグパネル必須:** オンチェーン命令を伴う機能は、`app/src/debug/instructions/` に対応するデバッグパネル（新アカウント型なら `accounts/` にインスペクタも）を追加し、`App.tsx` に配線、localnet で手動確認することを受け入れ条件に含める（詳細は `CLAUDE.md`）。

- [ ] **Step 2: Commit**

```bash
cd /workspace && git add features.md && git commit -m "docs: reference mandatory debug-panel rule in features.md"
```

---

## Task 13: 最終検証（ビルド/テスト/手動フロー手順）

**Files:**
- Modify: `app/README.md`（デバッグ画面の使い方を追記）

- [ ] **Step 1: 全自動チェック**

Run: `cd /workspace/app && pnpm build && pnpm test && pnpm lint`
Expected: すべて成功。

- [ ] **Step 2: 手動フロー手順を README に追記**

`app/README.md` に「localnet デバッグ手順」を追記:
1. `NO_DNA=1 surfpool start`（別シェル）
2. `cd app && pnpm dev`
3. ウォレット接続 → Airdrop → CreateMint → MintTo（自分に付与）
4. initialize_protocol（attestor=自分、payment_mint=作成した mint）→ ProtocolConfigInspector で確認
5. register_publisher → stake → create_campaign → fund_campaign → submit_claim → settle_claim（challenge_window 経過後）

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add app/README.md && git commit -m "docs(app): document localnet debug-harness flow"
```

---

## Self-Review チェック

- **Spec カバレッジ:** 再生成(T1)/基盤(T2-4)/localnet(T5)/インスペクタ(T6)/11パネル(T7-9)/配線(T10)/CLAUDE.md(T11)/features.md(T12)/検証(T13) — spec の受け入れ条件を網羅。
- **型整合:** `parseU64/parseU16/parseBytes32Hex/parsePubkey`、`configPda/treasuryPda/publisherPda/stakeVaultPda/campaignPda/escrowVaultPda/claimPda`、`DebugPanel{title,children,build,signers,disabled}`、`useInstructionRunner{phase,summary,signature,error,simulate,send,isSending}` を全Taskで一貫使用。
- **生成コード依存:** 命令関数名・アカウント引数キャメル名・`fetch*`/decoder 名・`URTHR_NET_PROGRAM_ADDRESS` は再生成後に `src/generated/` で最終確認する旨を各Taskに明記（IDL は確定済みのため名称はブレないが、codama の命名規約に正確に合わせる）。
- **framework-kit 実API:** `useTransactionPool(prepare/toWire/prepareAndSend/clearInstructions/addInstruction/isSending)`・`useSolanaClient().runtime.rpc.simulateTransaction`・`useWalletConnection`・`client.splToken(...)` は型定義で確認済み。`prepare`/`toWire` の細部は実装時に型定義へ最終整合。
