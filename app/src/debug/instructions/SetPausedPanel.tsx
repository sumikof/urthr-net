import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { configPda } from "../core/pdas";
import { getSetPausedInstruction } from "../../generated";

export function SetPausedPanel() {
  const { wallet, status } = useWalletConnection();
  const [paused, setPaused] = useState(false);
  if (status !== "connected" || !wallet) return null;

  return (
    <DebugPanel
      title="set_paused"
      disabled={false}
      resetKey={String(paused)}
      build={async (signer: TransactionSigner) => {
        const config = await configPda();
        return getSetPausedInstruction({ admin: signer, config, paused });
      }}
    >
      <label>
        <input type="checkbox" checked={paused} onChange={(e) => setPaused(e.target.checked)} /> paused
      </label>
    </DebugPanel>
  );
}
