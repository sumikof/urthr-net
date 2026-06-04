import { useBalance, useWalletConnection } from "@solana/react-hooks";
import { formatSol } from "../lib/format";
import { CopyButton } from "./CopyButton";

export function AccountInfo() {
  const { wallet, status } = useWalletConnection();
  const address = wallet?.account.address;
  const { lamports, fetching } = useBalance(address);

  if (status !== "connected" || !address) return null;

  return (
    <div>
      <p className="addr-row">
        <span>アドレス:</span>
        <code className="addr">{address}</code>
        <CopyButton value={address} />
      </p>
      <p>残高: {fetching && lamports == null ? "取得中…" : `${formatSol(lamports ?? 0n)} SOL`}</p>
    </div>
  );
}
