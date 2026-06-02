import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64, parseBytes32Hex } from "../core/fields";
import { campaignPda, claimPda, publisherPda } from "../core/pdas";
import { getChallengeClaimInstruction } from "../../generated";

export function ChallengeClaimPanel() {
  const { wallet, status } = useWalletConnection();
  const [advertiser, setAdvertiser] = useState("");
  const [campaignId, setCampaignId] = useState("0");
  const [claimNonce, setClaimNonce] = useState("0");
  const [publisherAuthority, setPublisherAuthority] = useState("");
  const [evidenceHash, setEvidenceHash] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAdv = parsePubkey(advertiser),
    pId = parseU64(campaignId),
    pNonce = parseU64(claimNonce),
    pPub = parsePubkey(publisherAuthority),
    pEvidence = parseBytes32Hex(evidenceHash);
  const disabled = !(pAdv.ok && pId.ok && pNonce.ok && pPub.ok && pEvidence.ok);

  return (
    <DebugPanel
      title="challenge_claim"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const adv = (pAdv as Extract<typeof pAdv, { ok: true }>).value;
        const id = (pId as Extract<typeof pId, { ok: true }>).value;
        const nonce = (pNonce as Extract<typeof pNonce, { ok: true }>).value;
        const pubAuth = (pPub as Extract<typeof pPub, { ok: true }>).value;
        const evidence = (pEvidence as Extract<typeof pEvidence, { ok: true }>).value;
        const campaign = await campaignPda(adv, id);
        const [claim, publisher] = await Promise.all([claimPda(campaign, nonce), publisherPda(pubAuth)]);
        return getChallengeClaimInstruction({
          challenger: signer,
          claim,
          publisher,
          evidenceHash: evidence,
        });
      }}
    >
      <TextField label="advertiser (pubkey)" value={advertiser} onChange={setAdvertiser} error={!pAdv.ok ? pAdv.error : undefined} />
      <TextField label="campaign_id (u64)" value={campaignId} onChange={setCampaignId} error={!pId.ok ? pId.error : undefined} />
      <TextField label="claim_nonce (u64)" value={claimNonce} onChange={setClaimNonce} error={!pNonce.ok ? pNonce.error : undefined} />
      <TextField label="publisher_authority (pubkey)" value={publisherAuthority} onChange={setPublisherAuthority} error={!pPub.ok ? pPub.error : undefined} />
      <TextField label="evidence_hash (bytes32 hex)" value={evidenceHash} onChange={setEvidenceHash} error={!pEvidence.ok ? pEvidence.error : undefined} />
    </DebugPanel>
  );
}
