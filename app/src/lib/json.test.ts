import { describe, expect, it } from "vitest";
import { stringifyWithBigInt } from "./json";

describe("stringifyWithBigInt", () => {
  it("serializes bigint values as strings instead of throwing", () => {
    // Plain JSON.stringify throws 'Do not know how to serialize a BigInt'.
    expect(stringifyWithBigInt({ amount: 6000n })).toBe('{"amount":"6000"}');
  });

  it("handles bigints nested inside RPC-style error objects", () => {
    // kit upcasts integers in RPC responses to bigint, so a failed
    // simulation's `err` can look like this.
    const err = { InstructionError: [1n, { Custom: 6000n }] };
    expect(stringifyWithBigInt(err)).toBe('{"InstructionError":["1",{"Custom":"6000"}]}');
  });

  it("leaves non-bigint values untouched and honors the space argument", () => {
    expect(stringifyWithBigInt({ a: 1, b: "x", c: true, d: null }, 2)).toBe(
      '{\n  "a": 1,\n  "b": "x",\n  "c": true,\n  "d": null\n}',
    );
  });

  it("serializes a top-level bigint", () => {
    expect(stringifyWithBigInt(42n)).toBe('"42"');
  });
});
