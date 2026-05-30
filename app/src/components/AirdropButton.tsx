import { useState } from "react";
import { useWalletActions, useWalletConnection } from "@solana/react-hooks";
import { address as toAddress, lamports } from "@solana/kit";

export function AirdropButton() {
  const { wallet, status } = useWalletConnection();
  const { requestAirdrop } = useWalletActions();
  const walletAddress = wallet?.account.address;
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  if (status !== "connected" || !walletAddress) return null;

  async function onAirdrop() {
    if (!walletAddress) return;
    setBusy(true);
    setMessage(null);
    try {
      // Goes through the framework client so the watched balance cache updates.
      const sig = await requestAirdrop(toAddress(walletAddress), lamports(2_000_000_000n));
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
