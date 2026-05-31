import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parseBytes32Hex, parsePubkey } from "../core/fields";
import { configPda, publisherPda, stakeVaultPda } from "../core/pdas";
import { getRegisterPublisherInstruction } from "../../generated";

export function RegisterPublisherPanel() {
  const { wallet, status } = useWalletConnection();
  const [metadata, setMetadata] = useState("");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pMeta = parseBytes32Hex(metadata),
    pMint = parsePubkey(mint);
  const disabled = !(pMeta.ok && pMint.ok);

  return (
    <DebugPanel
      title="register_publisher"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const [publisher, stakeVault, config] = await Promise.all([
          publisherPda(signer.address),
          stakeVaultPda(signer.address),
          configPda(),
        ]);
        return getRegisterPublisherInstruction({
          authority: signer,
          publisher,
          stakeVault,
          config,
          paymentMint: (pMint as Extract<typeof pMint, { ok: true }>).value,
          metadata: (pMeta as Extract<typeof pMeta, { ok: true }>).value,
        });
      }}
    >
      <TextField label="metadata (bytes32 hex)" value={metadata} onChange={setMetadata} error={!pMeta.ok ? pMeta.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
