import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstruction,
  getMintToInstruction,
} from "@solana-program/token";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parsePubkey, parseU64, useField } from "../core/fields";
import { TOKEN_PROGRAM_ADDRESS } from "../core/programs";

type Balance = Readonly<{ exists: boolean; amount: bigint; uiAmount: string }>;

/**
 * Mint tokens from an existing mint (where the connected wallet is the mint
 * authority) into the wallet's own ATA, creating the ATA idempotently.
 */
export function MintToPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const mintField = useField("");
  const amountField = useField("");
  const [balance, setBalance] = useState<Balance | null>(null);
  const [fetching, setFetching] = useState(false);

  if (status !== "connected" || !wallet) return null;

  const owner = wallet.account.address;
  const mintParsed = parsePubkey(mintField.value);
  const amountParsed = parseU64(amountField.value);
  const valid = mintParsed.ok && amountParsed.ok;

  // DebugPanel owns the runner and does not surface the send result, so the
  // wallet's post-mint balance is fetched on demand via this button.
  async function onRefreshBalance() {
    if (!mintParsed.ok) return;
    setFetching(true);
    try {
      const bal = await client.splToken({ mint: mintParsed.value }).fetchBalance(owner);
      setBalance({ exists: bal.exists, amount: bal.amount, uiAmount: bal.uiAmount });
    } catch {
      setBalance(null);
    } finally {
      setFetching(false);
    }
  }

  return (
    <DebugPanel
      title="mintTo → 自分のATA (localnet)"
      disabled={!valid}
      build={async (walletSigner) => {
        if (!mintParsed.ok || !amountParsed.ok) {
          throw new Error("mint と amount を入力してください");
        }
        const mint = mintParsed.value;
        const [ata] = await findAssociatedTokenPda({
          owner,
          mint,
          tokenProgram: TOKEN_PROGRAM_ADDRESS,
        });
        return [
          getCreateAssociatedTokenIdempotentInstruction({
            payer: walletSigner,
            ata,
            owner,
            mint,
          }),
          getMintToInstruction({
            mint,
            token: ata,
            mintAuthority: walletSigner,
            amount: amountParsed.value,
          }),
        ];
      }}
    >
      <TextField
        label="mint"
        value={mintField.value}
        onChange={mintField.setValue}
        placeholder="mint address"
        error={mintParsed.ok ? undefined : mintField.value ? mintParsed.error : undefined}
      />
      <TextField
        label="amount (base units)"
        value={amountField.value}
        onChange={amountField.setValue}
        placeholder="1000000"
        error={amountParsed.ok ? undefined : amountField.value ? amountParsed.error : undefined}
      />
      <div style={{ margin: "0.25rem 0" }}>
        <button
          type="button"
          onClick={() => void onRefreshBalance()}
          disabled={!mintParsed.ok || fetching}
        >
          {fetching ? "取得中…" : "残高を更新"}
        </button>
        {balance && (
          <span style={{ marginLeft: 8, fontSize: "0.9em" }}>
            {balance.exists
              ? `残高: ${balance.uiAmount} (${balance.amount.toString()} base units)`
              : "ATA がまだ存在しません"}
          </span>
        )}
      </div>
    </DebugPanel>
  );
}
