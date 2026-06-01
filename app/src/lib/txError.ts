/**
 * Extract a human-readable message from an error thrown while sending a
 * transaction.
 *
 * `@solana/kit`'s transaction-plan executor wraps the underlying failure in a
 * generic `SOLANA_ERROR__INSTRUCTION_PLANS__FAILED_TO_EXECUTE_TRANSACTION_PLAN`
 * ("The provided transaction plan failed to execute…"). The real cause — e.g. a
 * preflight failure with on-chain program logs — lives in
 * `error.context.transactionPlanResult`, a single/sequential/parallel tree whose
 * failed leaf is `{ kind: "single", status: { kind: "failed", error } }`.
 * Without unwrapping it, the UI only ever shows the useless generic message.
 *
 * These objects come from a different `@solana/kit` version than the app's, so
 * everything is treated as untyped and accessed defensively.
 */
export function describeTransactionError(error: unknown, depth = 0): string {
  if (depth > 6 || !error || typeof error !== "object") {
    return messageOf(error);
  }

  const context = (error as { context?: Record<string, unknown> }).context;

  // Prefer the real cause buried in a failed transaction plan.
  const nested = findFailedPlanError(context?.transactionPlanResult);
  if (nested !== undefined && nested !== error) {
    return describeTransactionError(nested, depth + 1);
  }

  const base = messageOf(error);

  // Preflight / simulation failures attach the program logs here.
  const logs = context?.logs;
  if (Array.isArray(logs) && logs.length > 0) {
    return `${base}\nLogs:\n${logs.join("\n")}`;
  }

  // Deprecated, but kit still sets `cause` on the plan-failure error today.
  const cause = (error as { cause?: unknown }).cause;
  if (cause && cause !== error) {
    const causeMessage = describeTransactionError(cause, depth + 1);
    if (causeMessage && causeMessage !== base) {
      return `${base}: ${causeMessage}`;
    }
  }

  return base;
}

/** Walk a transactionPlanResult tree and return the first failed leaf's error. */
function findFailedPlanError(result: unknown): unknown {
  if (!result || typeof result !== "object") return undefined;
  const node = result as { kind?: string; status?: { kind?: string; error?: unknown }; plans?: unknown[] };
  if (node.kind === "single") {
    return node.status?.kind === "failed" ? node.status.error : undefined;
  }
  if (Array.isArray(node.plans)) {
    for (const plan of node.plans) {
      const error = findFailedPlanError(plan);
      if (error !== undefined) return error;
    }
  }
  return undefined;
}

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && typeof (error as { message?: unknown }).message === "string") {
    return (error as { message: string }).message;
  }
  return String(error);
}
