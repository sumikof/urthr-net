import { describe, it, expect } from "vitest";
import { address } from "@solana/kit";
import { configPda, treasuryPda, publisherPda, stakeVaultPda, campaignPda, escrowVaultPda, claimPda } from "./pdas";

const ADV = address("11111111111111111111111111111112");

describe("pdas", () => {
  it("config/treasury are stable, valid, distinct addresses", async () => {
    const c = await configPda();
    const t = await treasuryPda();
    expect(typeof c).toBe("string");
    expect(c).not.toBe(t);
    // deterministic
    expect(await configPda()).toBe(c);
  });
  it("publisher and stake_vault both key off authority and differ", async () => {
    const p = await publisherPda(ADV);
    const s = await stakeVaultPda(ADV);
    expect(p).not.toBe(s);
  });
  it("campaign derivation depends on advertiser + id", async () => {
    expect(await campaignPda(ADV, 1n)).not.toBe(await campaignPda(ADV, 2n));
  });
  it("escrow/claim derive from the campaign pda", async () => {
    const camp = await campaignPda(ADV, 1n);
    expect(await escrowVaultPda(camp)).not.toBe(await claimPda(camp, 0n));
  });
});
