import type { TransactionSigner } from "@solana/kit";
import type { WalletSession } from "@solana/client";

/**
 * Fee-payer / authority options for the shared transaction pool's `prepare`.
 *
 * In nearly every panel the connected wallet is BOTH the fee payer AND an
 * instruction signer (admin / authority / advertiser / payer). The fee payer
 * must therefore be the *exact same* `TransactionSigner` instance that the
 * panel embeds into its instruction signer account(s).
 *
 * Why this matters: `@solana/client` bundles `@solana/kit` 5.5.1, whose
 * `deduplicateSigners` compares signers by REFERENCE only (the structural
 * fallback was added in kit 6.x). If we pass the wallet *address* as the fee
 * payer instead, the prepare recipe wraps a second, distinct signer object for
 * the same address — and kit throws
 * `SOLANA_ERROR__SIGNER__ADDRESS_CANNOT_HAVE_MULTIPLE_SIGNERS`
 * ("Multiple distinct signers were identified for address ...").
 *
 * `authority` stays the wallet session so the send/partial signing mode is
 * resolved from the wallet's real capabilities.
 */
export function runnerPrepareOptions(signer: TransactionSigner, wallet: WalletSession) {
  return { feePayer: signer, authority: wallet };
}
