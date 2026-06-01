import { describe, expect, it } from "vitest";
import { describeTransactionError } from "./txError";

// Shapes mirror @solana/kit's SolanaError: `.context` holds `__code` and, for a
// failed transaction plan, `transactionPlanResult` (a single/sequential/parallel
// tree whose failed leaf is `{ kind: "single", status: { kind: "failed", error } }`).
const planFailure = (result: unknown) => ({
  message: "The provided transaction plan failed to execute. See the `transactionPlanResult` attribute for more details.",
  context: { __code: 1, transactionPlanResult: result },
});

describe("describeTransactionError", () => {
  it("returns the message of a plain Error", () => {
    expect(describeTransactionError(new Error("boom"))).toBe("boom");
  });

  it("digs the real cause out of a failed single transaction plan", () => {
    const e = planFailure({
      kind: "single",
      status: { kind: "failed", error: new Error("custom program error: 0x1") },
    });
    expect(describeTransactionError(e)).toBe("custom program error: 0x1");
  });

  it("appends on-chain logs when the nested error carries them", () => {
    const nested = {
      message: "Transaction simulation failed",
      context: { logs: ["Program log: alloc", "Program failed: insufficient funds"] },
    };
    const e = planFailure({ kind: "single", status: { kind: "failed", error: nested } });
    expect(describeTransactionError(e)).toBe(
      "Transaction simulation failed\nLogs:\nProgram log: alloc\nProgram failed: insufficient funds",
    );
  });

  it("finds the failed leaf inside a sequential/parallel plan", () => {
    const e = planFailure({
      kind: "sequential",
      plans: [
        { kind: "single", status: { kind: "successful" } },
        { kind: "single", status: { kind: "failed", error: new Error("second tx failed") } },
      ],
    });
    expect(describeTransactionError(e)).toBe("second tx failed");
  });

  it("falls back to the deprecated cause when there is no plan result", () => {
    const e = { message: "wrapper", cause: new Error("the actual reason") } as unknown;
    expect(describeTransactionError(e)).toBe("wrapper: the actual reason");
  });

  it("stringifies non-error values", () => {
    expect(describeTransactionError("nope")).toBe("nope");
  });
});
