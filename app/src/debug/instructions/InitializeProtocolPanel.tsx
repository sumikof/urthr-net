import { useState } from "react";
import { useWalletConnection } from "@solana/react-hooks";
import type { TransactionSigner } from "@solana/kit";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU16, parseU64 } from "../core/fields";
import { configPda, treasuryPda, programDataPda } from "../core/pdas";
import { getInitializeProtocolInstruction, URTHR_NET_PROGRAM_ADDRESS } from "../../generated";

export function InitializeProtocolPanel() {
  const { wallet, status } = useWalletConnection();
  const [attestor, setAttestor] = useState("");
  const [feeBps, setFeeBps] = useState("50");
  const [minStake, setMinStake] = useState("1000000");
  const [window_, setWindow] = useState("60");
  const [mint, setMint] = useState("");
  if (status !== "connected" || !wallet) return null;

  const pAtt = parsePubkey(attestor),
    pFee = parseU16(feeBps),
    pMin = parseU64(minStake),
    pWin = parseU64(window_),
    pMint = parsePubkey(mint);
  const disabled = !(pAtt.ok && pFee.ok && pMin.ok && pWin.ok && pMint.ok);

  return (
    <DebugPanel
      title="initialize_protocol"
      disabled={disabled}
      build={async (signer: TransactionSigner) => {
        const [config, treasury, programData] = await Promise.all([configPda(), treasuryPda(), programDataPda()]);
        return getInitializeProtocolInstruction({
          admin: signer,
          config,
          treasury,
          program: URTHR_NET_PROGRAM_ADDRESS,
          programData,
          paymentMint: (pMint as Extract<typeof pMint, { ok: true }>).value,
          attestor: (pAtt as Extract<typeof pAtt, { ok: true }>).value,
          protocolFeeBps: (pFee as Extract<typeof pFee, { ok: true }>).value,
          minPublisherStake: (pMin as Extract<typeof pMin, { ok: true }>).value,
          challengeWindow: (pWin as Extract<typeof pWin, { ok: true }>).value,
        });
      }}
    >
      <TextField label="attestor (pubkey)" value={attestor} onChange={setAttestor} error={!pAtt.ok ? pAtt.error : undefined} />
      <TextField label="protocol_fee_bps (u16)" value={feeBps} onChange={setFeeBps} error={!pFee.ok ? pFee.error : undefined} />
      <TextField label="min_publisher_stake (u64)" value={minStake} onChange={setMinStake} error={!pMin.ok ? pMin.error : undefined} />
      <TextField label="challenge_window (u64 秒)" value={window_} onChange={setWindow} error={!pWin.ok ? pWin.error : undefined} />
      <TextField label="payment_mint (pubkey)" value={mint} onChange={setMint} error={!pMint.ok ? pMint.error : undefined} />
    </DebugPanel>
  );
}
