import { useState } from "react";
import { useTransactionPool, useWalletConnection } from "@solana/react-hooks";
import { getInitializeInstruction } from "../generated";

export function InitializeButton() {
  const { wallet, status } = useWalletConnection();
  const { addInstruction, clearInstructions, prepareAndSend, isSending } = useTransactionPool();
  const [signature, setSignature] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (status !== "connected" || !wallet) return null;

  async function onInitialize() {
    if (!wallet) return;
    setSignature(null);
    setError(null);
    try {
      clearInstructions();
      addInstruction(getInitializeInstruction());
      const result = await prepareAndSend({ authority: wallet });
      setSignature(result);
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
