import { useCallback, useState } from "react";
import { type Address } from "@solana/kit";
import { fetchMaybeClaim } from "../../generated";
import { TextField, parsePubkey, parseU64 } from "../core/fields";
import { claimPda } from "../core/pdas";
import { AccountInspector } from "./AccountInspector";

export function ClaimInspector() {
  const [campaign, setCampaign] = useState("");
  const [claimNonce, setClaimNonce] = useState("");

  const camp = parsePubkey(campaign);
  const nonce = parseU64(claimNonce);

  // Fresh closure per input change so AccountInspector re-derives the PDA.
  const defaultAddress = useCallback(async (): Promise<Address | undefined> => {
    const c = parsePubkey(campaign);
    const n = parseU64(claimNonce);
    return c.ok && n.ok ? await claimPda(c.value, n.value) : undefined;
  }, [campaign, claimNonce]);

  return (
    <AccountInspector
      title="Claim"
      defaultAddress={defaultAddress}
      fetchMaybe={fetchMaybeClaim}
    >
      <TextField
        label="campaign (PDA)"
        value={campaign}
        onChange={setCampaign}
        error={campaign && !camp.ok ? camp.error : undefined}
      />
      <TextField
        label="claim_nonce"
        value={claimNonce}
        onChange={setClaimNonce}
        error={claimNonce && !nonce.ok ? nonce.error : undefined}
      />
    </AccountInspector>
  );
}
