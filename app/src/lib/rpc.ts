import { address, createSolanaRpc, devnet, lamports, type Address } from "@solana/kit";

export const RPC_URL = import.meta.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
export const WS_URL = import.meta.env.VITE_WS_URL ?? "ws://127.0.0.1:8900";

export async function requestAirdrop(recipient: string, sol: number): Promise<string> {
  // Use devnet() to brand the URL so the RPC type includes requestAirdrop.
  // Surfnet/localnet exposes the same airdrop endpoint as devnet.
  const rpc = createSolanaRpc(devnet(RPC_URL));
  const recipientAddress: Address = address(recipient);
  const signature = await rpc
    .requestAirdrop(recipientAddress, lamports(BigInt(sol) * 1_000_000_000n), {
      commitment: "confirmed",
    })
    .send();
  return signature;
}
