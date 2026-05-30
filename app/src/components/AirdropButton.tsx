import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import { requestAirdrop } from "../lib/rpc";

export function AirdropButton() {
  const { wallet, status } = useWalletConnection();
  const address = wallet?.account.address;
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  if (status !== "connected" || !address) return null;

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
