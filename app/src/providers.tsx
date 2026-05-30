import React from "react";
import { autoDiscover, createClient } from "@solana/client";
import { SolanaProvider } from "@solana/react-hooks";
import { RPC_URL, WS_URL } from "./lib/rpc";

export const solanaClient = createClient({
  endpoint: RPC_URL,
  websocketEndpoint: WS_URL,
  walletConnectors: autoDiscover(),
});

export function Providers({ children }: { children: React.ReactNode }) {
  return <SolanaProvider client={solanaClient}>{children}</SolanaProvider>;
}
