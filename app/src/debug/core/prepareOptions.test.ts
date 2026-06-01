import { describe, expect, it } from "vitest";
import type { TransactionSigner } from "@solana/kit";
import type { WalletSession } from "@solana/client";
import { runnerPrepareOptions } from "./prepareOptions";

// Minimal stand-ins matching the shapes the runner actually wires together.
const ADDR = "92tfnBzbaNk1sSbAKR4YfDbujVUvvY7Y6Vda4MXhvEZK";
const signer = {
  address: ADDR,
  modifyAndSignTransactions: () => Promise.resolve([]),
  signTransactions: () => Promise.resolve([]),
} as unknown as TransactionSigner;
const wallet = {
  account: { address: ADDR },
  connector: {},
  disconnect: () => {},
} as unknown as WalletSession;

describe("runnerPrepareOptions", () => {
  it("uses the embedded signer INSTANCE as the fee payer (kit 5.5.1 dedups by reference)", () => {
    const opts = runnerPrepareOptions(signer, wallet);
    // The fee payer must be the very same TransactionSigner instance that panels
    // embed into instruction signer accounts. @solana/client bundles kit 5.5.1,
    // whose deduplicateSigners compares by reference only — a second, distinct
    // signer object for the same address throws
    // SOLANA_ERROR__SIGNER__ADDRESS_CANNOT_HAVE_MULTIPLE_SIGNERS.
    expect(opts.feePayer).toBe(signer);
  });

  it("does NOT pass the bare wallet address as fee payer (the regression)", () => {
    const opts = runnerPrepareOptions(signer, wallet);
    // Passing wallet.account.address (a string) makes the prepare recipe wrap a
    // distinct fee-payer signer, colliding with the embedded one.
    expect(opts.feePayer).not.toBe(wallet.account.address);
  });

  it("keeps the wallet session as authority so send/partial mode is resolved correctly", () => {
    const opts = runnerPrepareOptions(signer, wallet);
    expect(opts.authority).toBe(wallet);
  });
});
