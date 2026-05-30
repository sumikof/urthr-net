import { address, createSolanaRpc, lamports, type Address } from "@solana/kit";

export const RPC_URL = import.meta.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
export const WS_URL = import.meta.env.VITE_WS_URL ?? "ws://127.0.0.1:8900";

export async function requestAirdrop(recipient: string, sol: number): Promise<string> {
  const rpc = createSolanaRpc(RPC_URL);
  const recipientAddress: Address = address(recipient);
  const signature = await rpc
    .requestAirdrop(recipientAddress, lamports(BigInt(sol) * 1_000_000_000n), {
      commitment: "confirmed",
    })
    .send();
  return signature;
}
