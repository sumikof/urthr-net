import { useCallback, useMemo, useState } from "react";
import { useSolanaClient, useTransactionPool, useWalletConnection } from "@solana/react-hooks";
import { createWalletTransactionSigner } from "@solana/client";
import type { Base64EncodedWireTransaction, Instruction, TransactionSigner } from "@solana/kit";

/** What a panel's `build` callback must produce: a fully-formed instruction with
 *  its signer account(s) populated by the supplied wallet-derived signer. */
export type BuildInstruction = (
  signer: TransactionSigner,
) => Promise<Instruction | readonly Instruction[]> | Instruction | readonly Instruction[];

export type SimulationSummary = Readonly<{
  /** Transaction-level error from simulation, or `null` on success. */
  err: unknown;
  /** Compute units consumed, when the node reports them. */
  unitsConsumed: bigint | null;
  /** Program log lines, when available. */
  logs: readonly string[] | null;
}>;

export type RunnerPhase = "idle" | "simulated" | "sent" | "error";

export type InstructionRunner = Readonly<{
  phase: RunnerPhase;
  summary: SimulationSummary | null;
  signature: string | null;
  error: string | null;
  isSimulating: boolean;
  isSending: boolean;
  /** True once a successful simulation has occurred and a send is allowed. */
  canSend: boolean;
  /** Build → prepare → simulate. Never throws; on failure sets phase "error". */
  simulate: (build: BuildInstruction) => Promise<void>;
  /** Send the already-prepared transaction. Never throws; on failure sets phase "error". */
  send: () => Promise<void>;
  /** Reset back to the idle state, clearing any pooled instruction/result. */
  reset: () => void;
}>;

function errMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Simulate-before-send execution layer shared by every instruction panel.
 *
 * Signer strategy = EMBED: the connected wallet session is wrapped into a
 * `TransactionSigner` (via `createWalletTransactionSigner`) and handed to the
 * panel's `build(signer)`, which embeds it into the instruction's signer
 * account(s). The same wallet is the fee payer/authority for prepare + send.
 */
export function useInstructionRunner(): InstructionRunner {
  const client = useSolanaClient();
  const { wallet } = useWalletConnection();
  const pool = useTransactionPool();

  const [phase, setPhase] = useState<RunnerPhase>("idle");
  const [summary, setSummary] = useState<SimulationSummary | null>(null);
  const [signature, setSignature] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSimulating, setIsSimulating] = useState(false);

  // A TransactionSigner backed by the connected wallet session. Recomputed when
  // the session changes so panels always embed the live signer.
  const signer = useMemo<TransactionSigner | null>(
    () => (wallet ? createWalletTransactionSigner(wallet).signer : null),
    [wallet],
  );

  const reset = useCallback(() => {
    pool.clearInstructions();
    setPhase("idle");
    setSummary(null);
    setSignature(null);
    setError(null);
  }, [pool]);

  const simulate = useCallback(
    async (build: BuildInstruction) => {
      if (!wallet || !signer) {
        setPhase("error");
        setError("ウォレットが接続されていません");
        return;
      }
      setIsSimulating(true);
      setSummary(null);
      setSignature(null);
      setError(null);
      try {
        const built = await build(signer);
        const arr = Array.isArray(built) ? built : [built as Instruction];
        pool.clearInstructions();
        for (const ix of arr) pool.addInstruction(ix);
        // Prepare with the wallet as fee payer/authority so the pooled, prepared
        // transaction is ready for `send()` after approval.
        await pool.prepare({ feePayer: wallet.account.address, authority: wallet });
        // `toWire` returns a base64 string; the simulate RPC wants the branded
        // Base64EncodedWireTransaction. `replaceRecentBlockhash` (which implies
        // sigVerify=false) lets us simulate without fully-signed signatures.
        const wire = (await pool.toWire()) as Base64EncodedWireTransaction;
        const { value } = await client.runtime.rpc
          .simulateTransaction(wire, { encoding: "base64", replaceRecentBlockhash: true })
          .send();
        setSummary({
          err: value.err,
          unitsConsumed: value.unitsConsumed ?? null,
          logs: value.logs ?? null,
        });
        if (value.err) {
          setPhase("error");
          setError(`シミュレーション失敗: ${JSON.stringify(value.err)}`);
        } else {
          setPhase("simulated");
        }
      } catch (e) {
        setPhase("error");
        setError(errMessage(e));
      } finally {
        setIsSimulating(false);
      }
    },
    [client, pool, signer, wallet],
  );

  const send = useCallback(async () => {
    if (phase !== "simulated") {
      setPhase("error");
      setError("送信前に成功したシミュレーションが必要です");
      return;
    }
    setError(null);
    try {
      const sig = await pool.send();
      setSignature(String(sig));
      setPhase("sent");
    } catch (e) {
      setPhase("error");
      setError(errMessage(e));
    }
  }, [phase, pool]);

  return {
    phase,
    summary,
    signature,
    error,
    isSimulating,
    isSending: pool.isSending,
    canSend: phase === "simulated",
    simulate,
    send,
    reset,
  };
}
