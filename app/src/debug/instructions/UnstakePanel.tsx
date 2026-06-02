import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { configPda, publisherPda, stakeVaultPda } from "../core/pdas";
import { getUnstakeInstruction } from "../../generated";

export function UnstakePanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const [amount, setAmount] = useState("1000000");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAmt = parseU64(amount),
    pMint = parsePubkey(mint);
  const disabled = !(pAmt.ok && pMint.ok);

  return (
    <DebugPanel
      title="unstake"
      disabled={disabled}
      resetKey={JSON.stringify([amount, mint])}
      build={async (signer: TransactionSigner) => {
        const pm = (pMint as Extract<typeof pMint, { ok: true }>).value;
        const [config, publisher, stakeVault, authorityTokenAccount] = await Promise.all([
          configPda(),
          publisherPda(signer.address),
          stakeVaultPda(signer.address),
          client.splToken({ mint: pm }).deriveAssociatedTokenAddress(signer.address),
        ]);
        return getUnstakeInstruction({
          authority: signer,
          config,
          publisher,
          stakeVault,
          authorityTokenAccount,
          paymentMint: pm,
          amount: (pAmt as Extract<typeof pAmt, { ok: true }>).value,
        });
      }}
    >
      <TextField label="amount (u64)" value={amount} onChange={setAmount} error={!pAmt.ok ? pAmt.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
