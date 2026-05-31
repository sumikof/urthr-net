import { useCallback, useState } from "react";
import { type Address } from "@solana/kit";
import { fetchMaybePublisher } from "../../generated";
import { TextField, parsePubkey } from "../core/fields";
import { publisherPda } from "../core/pdas";
import { AccountInspector } from "./AccountInspector";

export function PublisherInspector() {
  const [authority, setAuthority] = useState("");
  const parsed = parsePubkey(authority);

  // Fresh closure per `authority` change so AccountInspector re-derives the PDA.
  const defaultAddress = useCallback(async (): Promise<Address | undefined> => {
    const p = parsePubkey(authority);
    return p.ok ? await publisherPda(p.value) : undefined;
  }, [authority]);

  return (
    <AccountInspector
      title="Publisher"
      defaultAddress={defaultAddress}
      fetchMaybe={fetchMaybePublisher}
    >
      <TextField
        label="authority"
        value={authority}
        onChange={setAuthority}
        error={authority && !parsed.ok ? parsed.error : undefined}
      />
    </AccountInspector>
  );
}
