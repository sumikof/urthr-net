import { useCallback, useState } from "react";
import { type Address } from "@solana/kit";
import { fetchMaybeCampaign } from "../../generated";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { campaignPda } from "../core/pdas";
import { AccountInspector } from "./AccountInspector";

export function CampaignInspector() {
  const [advertiser, setAdvertiser] = useState("");
  const [campaignId, setCampaignId] = useState("");

  const adv = parsePubkey(advertiser);
  const id = parseU64(campaignId);

  // Fresh closure per input change so AccountInspector re-derives the PDA.
  const defaultAddress = useCallback(async (): Promise<Address | undefined> => {
    const a = parsePubkey(advertiser);
    const i = parseU64(campaignId);
    return a.ok && i.ok ? await campaignPda(a.value, i.value) : undefined;
  }, [advertiser, campaignId]);

  return (
    <AccountInspector
      title="Campaign"
      defaultAddress={defaultAddress}
      fetchMaybe={fetchMaybeCampaign}
    >
      <TextField
        label="advertiser"
        value={advertiser}
        onChange={setAdvertiser}
        error={advertiser && !adv.ok ? adv.error : undefined}
      />
      <TextField
        label="campaign_id"
        value={campaignId}
        onChange={setCampaignId}
        error={campaignId && !id.ok ? id.error : undefined}
      />
    </AccountInspector>
  );
}
