import { describe, it, expect } from "vitest";
import { parseU64, parseU16, parseBytes32Hex, parsePubkey } from "./fields";

describe("field parsers", () => {
  it("parseU64 accepts range, rejects negatives/garbage/overflow", () => {
    expect(parseU64("0")).toEqual({ ok: true, value: 0n });
    expect(parseU64("1000000")).toEqual({ ok: true, value: 1000000n });
    expect(parseU64("-1").ok).toBe(false);
    expect(parseU64("notanumber").ok).toBe(false);
    expect(parseU64((2n ** 64n).toString()).ok).toBe(false);
  });
  it("parseU16 rejects values above 65535", () => {
    expect(parseU16("50")).toEqual({ ok: true, value: 50 });
    expect(parseU16("65536").ok).toBe(false);
  });
  it("parseBytes32Hex requires exactly 32 bytes, with or without 0x", () => {
    expect(parseBytes32Hex("0x" + "00".repeat(32)).ok).toBe(true);
    expect(parseBytes32Hex("ab".repeat(32)).ok).toBe(true);
    expect(parseBytes32Hex("00".repeat(31)).ok).toBe(false);
    expect(parseBytes32Hex("zz".repeat(32)).ok).toBe(false);
  });
  it("parsePubkey validates base58 address", () => {
    expect(parsePubkey("11111111111111111111111111111112").ok).toBe(true);
    expect(parsePubkey("not-an-address").ok).toBe(false);
  });
});
