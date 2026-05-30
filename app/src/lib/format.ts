const LAMPORTS_PER_SOL = 1_000_000_000n;

export function formatSol(lamports: bigint | number): string {
  const value = typeof lamports === "bigint" ? lamports : BigInt(Math.trunc(lamports));
  const whole = value / LAMPORTS_PER_SOL;
  const frac = value % LAMPORTS_PER_SOL;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(9, "0").replace(/0+$/, "");
  return `${whole.toString()}.${fracStr}`;
}

export function shortenAddress(address: string): string {
  if (address.length < 9) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}
