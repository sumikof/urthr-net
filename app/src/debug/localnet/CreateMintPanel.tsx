import { useState } from "react";
import { useSolanaClient, useWalletConnection } from "@solana/react-hooks";
import { generateKeyPairSigner, type KeyPairSigner } from "@solana/kit";
import { getCreateAccountInstruction } from "@solana-program/system";
import { getInitializeMintInstruction, getMintSize } from "@solana-program/token";
import { DebugPanel } from "../core/DebugPanel";
import { TextField, parseU16, useField } from "../core/fields";
import { TOKEN_PROGRAM_ADDRESS } from "../core/programs";

/**
 * Create a fresh SPL mint on localnet, with the connected wallet as mint
 * authority. Generates an ad-hoc mint keypair (embedded as the `newAccount`
 * signer) so the kit pipeline collects it at prepare time.
 */
export function CreateMintPanel() {
  const client = useSolanaClient();
  const { wallet, status } = useWalletConnection();
  const decimalsField = useField("6");
  const [mintSigner, setMintSigner] = useState<KeyPairSigner | null>(null);
  const [generating, setGenerating] = useState(false);

  if (status !== "connected" || !wallet) return null;

  const decimalsParsed = parseU16(decimalsField.value);
  const decimalsValid = decimalsParsed.ok && decimalsParsed.value <= 9;
  const decimalsError = !decimalsParsed.ok
    ? decimalsParsed.error
    : decimalsParsed.value > 9
      ? "0〜9 を入力"
      : undefined;

  async function onGenerate() {
    setGenerating(true);
    try {
      setMintSigner(await generateKeyPairSigner());
    } finally {
      setGenerating(false);
    }
  }

  const ready = mintSigner != null && decimalsValid;

  return (
    <DebugPanel
      title="mint作成 (localnet)"
      disabled={!ready}
      build={async (walletSigner) => {
        if (!mintSigner || !decimalsParsed.ok) {
          throw new Error("mint鍵を生成し、decimals を入力してください");
        }
        const space = getMintSize();
        const lamports = await client.runtime.rpc
          .getMinimumBalanceForRentExemption(BigInt(space))
          .send();
        return [
          getCreateAccountInstruction({
            payer: walletSigner,
            newAccount: mintSigner,
            lamports,
            space,
            programAddress: TOKEN_PROGRAM_ADDRESS,
          }),
          getInitializeMintInstruction({
            mint: mintSigner.address,
            decimals: decimalsParsed.value,
            mintAuthority: walletSigner.address,
          }),
        ];
      }}
    >
      <TextField
        label="decimals (0〜9)"
        value={decimalsField.value}
        onChange={decimalsField.setValue}
        placeholder="6"
        error={decimalsError}
      />
      <div style={{ margin: "0.25rem 0" }}>
        <button type="button" onClick={() => void onGenerate()} disabled={generating}>
          {generating ? "生成中…" : "mint鍵を生成"}
        </button>
        {mintSigner && (
          <span style={{ marginLeft: 8, wordBreak: "break-all", fontSize: "0.9em" }}>
            mint: {mintSigner.address}
          </span>
        )}
      </div>
    </DebugPanel>
  );
}
