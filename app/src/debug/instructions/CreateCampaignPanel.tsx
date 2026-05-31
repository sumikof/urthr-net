import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { configPda, campaignPda, escrowVaultPda } from "../core/pdas";
import { getCreateCampaignInstruction } from "../../generated";

export function CreateCampaignPanel() {
  const { wallet, status } = useWalletConnection();
  const [campaignId, setCampaignId] = useState("0");
  const [pricePerEvent, setPricePerEvent] = useState("1");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pId = parseU64(campaignId),
    pPrice = parseU64(pricePerEvent),
    pMint = parsePubkey(mint);
  const priceZero = pPrice.ok && pPrice.value === 0n;
  const disabled = !(pId.ok && pPrice.ok && pMint.ok) || priceZero;

  return (
    <DebugPanel
      title="create_campaign"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const pm = (pMint as Extract<typeof pMint, { ok: true }>).value;
        const [config, campaign] = await Promise.all([configPda(), campaignPda(signer.address, id)]);
        const escrowVault = await escrowVaultPda(campaign);
        return getCreateCampaignInstruction({
          advertiser: signer,
          config,
          campaign,
          escrowVault,
          paymentMint: pm,
          campaignId: id,
          pricePerEvent: (pPrice as Extract<typeof pPrice, { ok: true }>).value,
        });
      }}
    >
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField
        label="price_per_event (u64, must be > 0)"
        value={pricePerEvent}
        onChange={setPricePerEvent}
        error={!pPrice.ok ? pPrice.error : priceZero ? "must be > 0 (on-chain InvalidPrice)" : undefined}
      />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
