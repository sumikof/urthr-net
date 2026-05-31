import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { configPda, campaignPda, escrowVaultPda } from "../core/pdas";
import { getCloseCampaignInstruction } from "../../generated";

export function CloseCampaignPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const [campaignId, setCampaignId] = useState("0");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pId = parseU64(campaignId),
    pMint = parsePubkey(mint);
  const disabled = !(pId.ok && pMint.ok);

  return (
    <DebugPanel
      title="close_campaign"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const pm = (pMint as Extract<typeof pMint, { ok: true }>).value;
        const [config, campaign, advertiserTokenAccount] = await Promise.all([
          configPda(),
          campaignPda(signer.address, id),
          client.splToken({ mint: pm }).deriveAssociatedTokenAddress(signer.address),
        ]);
        const escrowVault = await escrowVaultPda(campaign);
        return getCloseCampaignInstruction({
          advertiser: signer,
          config,
          campaign,
          escrowVault,
          advertiserTokenAccount,
          paymentMint: pm,
        });
      }}
    >
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
