import { useEffect, useState } from "react";
import { useSolanaClient } from "@solana/react-hooks";
import { type Address } from "@solana/kit";
import { fetchMaybeProtocolConfig } from "../../generated";
import { TextField, parsePubkey } from "../core/fields";
import { stringifyWithBigInt } from "../../lib/json";

// The rpc type the generated fetchers accept; derived from a fetcher's first arg
// so it stays in sync with codama output rather than being hand-written.
type Rpc = Parameters<typeof fetchMaybeProtocolConfig>[0];

export type AccountInspectorProps = Readonly<{
  title: string;
  /**
   * Optional: derive the address to prefill. Re-derived whenever this callback's
   * identity changes, so input-driven wrappers pass a fresh closure capturing
   * their current inputs (return `undefined` when inputs are incomplete/invalid).
   */
  defaultAddress?: () => Promise<Address | undefined>;
  fetchMaybe: (
    rpc: Rpc,
    addr: Address,
  ) => Promise<{ exists: boolean; address: Address; data?: unknown }>;
  /** Extra controls (e.g. derivation inputs) rendered above the address field. */
  children?: React.ReactNode;
}>;

type State =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "missing" }
  | { kind: "ok"; json: string }
  | { kind: "error"; message: string };

/** Reusable read-only account card: fetch + decode via the generated decoder. */
export function AccountInspector({
  title,
  defaultAddress,
  fetchMaybe,
  children,
}: AccountInspectorProps) {
  const client = useSolanaClient();
  const [addr, setAddr] = useState("");
  const [state, setState] = useState<State>({ kind: "idle" });

  // Prefill from an async-derived address. Re-runs when `defaultAddress` changes
  // identity (input-driven wrappers pass a fresh closure per input change).
  // All setState happens inside the async `.then`, never synchronously here.
  useEffect(() => {
    if (!defaultAddress) return;
    let cancelled = false;
    void defaultAddress().then((d) => {
      if (!cancelled && d) setAddr(d);
    });
    return () => {
      cancelled = true;
    };
  }, [defaultAddress]);

  const parsed = parsePubkey(addr);

  async function onFetch() {
    if (!parsed.ok) return;
    setState({ kind: "loading" });
    try {
      const acc = await fetchMaybe(client.runtime.rpc, parsed.value);
      if (!acc.exists) {
        setState({ kind: "missing" });
        return;
      }
      // All on-chain data is treated as untrusted; it is only surfaced after the
      // generated decoder validated the discriminator inside fetchMaybe.
      const json = stringifyWithBigInt(acc.data, 2);
      setState({ kind: "ok", json });
    } catch (err) {
      setState({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

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
}
