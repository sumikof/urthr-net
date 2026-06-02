import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { configPda, treasuryPda, publisherPda, campaignPda, escrowVaultPda, claimPda } from "../core/pdas";
import { getSettleClaimInstruction } from "../../generated";

export function SettleClaimPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const [advertiser, setAdvertiser] = useState("");
  const [campaignId, setCampaignId] = useState("0");
  const [claimNonce, setClaimNonce] = useState("0");
  const [publisherAuthority, setPublisherAuthority] = useState("");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAdv = parsePubkey(advertiser),
    pId = parseU64(campaignId),
    pNonce = parseU64(claimNonce),
    pPub = parsePubkey(publisherAuthority),
    pMint = parsePubkey(mint);
  const disabled = !(pAdv.ok && pId.ok && pNonce.ok && pPub.ok && pMint.ok);

  return (
    <DebugPanel
      title="settle_claim"
      disabled={disabled}
      resetKey={JSON.stringify([advertiser, campaignId, claimNonce, publisherAuthority, mint])}
      // permissionless: no signer account — the runner sets the wallet as fee payer (which signs).
      build={async (signer: TransactionSigner) => {
        void signer; // signer intentionally unused: settle_claim has no signer account.
        const adv = (pAdv as Extract<typeof pAdv, { ok: true }>).value;
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const nonce = (pNonce as Extract<typeof pNonce, { ok: true }>).value;
        const pubAuth = (pPub as Extract<typeof pPub, { ok: true }>).value;
        const pm = (pMint as Extract<typeof pMint, { ok: true }>).value;
        const [config, treasury, campaign, publisher, publisherTokenAccount] = await Promise.all([
          configPda(),
          treasuryPda(),
          campaignPda(adv, id),
          publisherPda(pubAuth),
          client.splToken({ mint: pm }).deriveAssociatedTokenAddress(pubAuth),
        ]);
        const [claim, escrowVault] = await Promise.all([claimPda(campaign, nonce), escrowVaultPda(campaign)]);
        return getSettleClaimInstruction({
          config,
          campaign,
          claim,
          escrowVault,
          treasury,
          publisher,
          publisherTokenAccount,
          paymentMint: pm,
        });
      }}
    >
      <TextField label="advertiser (pubkey)" value={advertiser} onChange={setAdvertiser} error={!pAdv.ok ? pAdv.error : undefined} />
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField label="claim_nonce (u64)" value={claimNonce} onChange={setClaimNonce} error={!pNonce.ok ? pNonce.error : undefined} />
      <TextField label="publisher_authority (pubkey)" value={publisherAuthority} onChange={setPublisherAuthority} error={!pPub.ok ? pPub.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
