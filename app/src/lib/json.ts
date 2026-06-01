/**
 * `JSON.stringify` that won't throw on `bigint` values (the native serializer
 * rejects them with "Do not know how to serialize a BigInt").
 *
 * On-chain data and kit's RPC responses surface integers as `bigint` (account
 * fields, compute units, error codes), so any time we stringify that data for
 * display we must convert bigints to strings first.
 */
export function stringifyWithBigInt(value: unknown, space?: number): string {
  return JSON.stringify(value, (_key, v) => (typeof v === "bigint" ? v.toString() : v), space);
}
