import { describe, expect, it } from "vitest";
import { formatSol, shortenAddress } from "./format";

describe("formatSol", () => {
  it("converts lamports (bigint) to SOL string with up to 9 decimals", () => {
    expect(formatSol(1_000_000_000n)).toBe("1");
    expect(formatSol(1_500_000_000n)).toBe("1.5");
    expect(formatSol(0n)).toBe("0");
  });
  it("accepts number input", () => {
    expect(formatSol(2_000_000_000)).toBe("2");
  });
});

describe("shortenAddress", () => {
  it("keeps first 4 and last 4 chars joined by an ellipsis", () => {
    expect(shortenAddress("3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR")).toBe("3CmD…RPoR");
  });
  it("returns the input unchanged when shorter than 9 chars", () => {
    expect(shortenAddress("abc")).toBe("abc");
  });
});
