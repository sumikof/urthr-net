import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64, parseBytes32Hex } from "../core/fields";
import { configPda, publisherPda, campaignPda, claimPda } from "../core/pdas";
import { getSubmitClaimInstruction, fetchCampaign } from "../../generated";

export function SubmitClaimPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const [advertiser, setAdvertiser] = useState("");
  const [campaignId, setCampaignId] = useState("0");
  const [eventCount, setEventCount] = useState("1");
  const [merkleRoot, setMerkleRoot] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAdv = parsePubkey(advertiser),
    pId = parseU64(campaignId),
    pEvt = parseU64(eventCount),
    pRoot = parseBytes32Hex(merkleRoot);
  const disabled = !(pAdv.ok && pId.ok && pEvt.ok && pRoot.ok);

  return (
    <DebugPanel
      title="submit_claim"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const adv = (pAdv as Extract<typeof pAdv, { ok: true }>).value;
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const evt = (pEvt as Extract<typeof pEvt, { ok: true }>).value;
        const root = (pRoot as Extract<typeof pRoot, { ok: true }>).value;
        const [config, publisher, campaign] = await Promise.all([
          configPda(),
          publisherPda(signer.address),
          campaignPda(adv, id),
        ]);
        // claim PDA uses the campaign's CURRENT claims counter as the nonce.
        const camp = await fetchCampaign(client.runtime.rpc, campaign);
        const nonce = camp.data.claimsCount;
        const claim = await claimPda(campaign, nonce);
        return getSubmitClaimInstruction({
          publisherAuthority: signer,
          config,
          publisher,
          campaign,
          claim,
          eventCount: evt,
          merkleRoot: root,
        });
      }}
    >
      <TextField label="advertiser (pubkey)" value={advertiser} onChange={setAdvertiser} error={!pAdv.ok ? pAdv.error : undefined} />
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField label="event_count (u64)" value={eventCount} onChange={setEventCount} error={!pEvt.ok ? pEvt.error : undefined} />
      <TextField label="merkle_root (bytes32 hex)" value={merkleRoot} onChange={setMerkleRoot} error={!pRoot.ok ? pRoot.error : undefined} />
    </DebugPanel>
  );
}
