import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { configPda, campaignPda, escrowVaultPda } from "../core/pdas";
import { getFundCampaignInstruction } from "../../generated";

export function FundCampaignPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const [campaignId, setCampaignId] = useState("0");
  const [amount, setAmount] = useState("1000000");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pId = parseU64(campaignId),
    pAmt = parseU64(amount),
    pMint = parsePubkey(mint);
  const disabled = !(pId.ok && pAmt.ok && pMint.ok);

  return (
    <DebugPanel
      title="fund_campaign"
      disabled={disabled}
      resetKey={JSON.stringify([campaignId, amount, mint])}
      build={async (signer: TransactionSigner) => {
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const pm = (pMint as Extract<typeof pMint, { ok: true }>).value;
        const [config, campaign, advertiserTokenAccount] = await Promise.all([
          configPda(),
          campaignPda(signer.address, id),
          client.splToken({ mint: pm }).deriveAssociatedTokenAddress(signer.address),
        ]);
        const escrowVault = await escrowVaultPda(campaign);
        return getFundCampaignInstruction({
          advertiser: signer,
          config,
          campaign,
          escrowVault,
          advertiserTokenAccount,
          paymentMint: pm,
          amount: (pAmt as Extract<typeof pAmt, { ok: true }>).value,
        });
      }}
    >
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField label="amount (u64)" value={amount} onChange={setAmount} error={!pAmt.ok ? pAmt.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
