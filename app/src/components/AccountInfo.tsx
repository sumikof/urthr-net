import { useBalance, useWalletConnection } from "@solana/react-hooks";
import { formatSol } from "../lib/format";

export function AccountInfo() {
  const { wallet, status } = useWalletConnection();
  const address = wallet?.account.address;
  const { lamports, fetching } = useBalance(address);

  if (status !== "connected" || !address) return null;

  return (
    <div>
      <p>アドレス: {address}</p>
      <p>残高: {fetching && lamports == null ? "取得中…" : `${formatSol(lamports ?? 0n)} SOL`}</p>
    </div>
  );
}
