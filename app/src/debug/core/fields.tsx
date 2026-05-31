import { useState } from "react";
import { address, type Address } from "@solana/kit";

export type Parsed<T> = { ok: true; value: T } | { ok: false; error: string };

export function parseU64(s: string): Parsed<bigint> {
  if (!/^\d+$/.test(s.trim())) return { ok: false, error: "正の整数を入力" };
  const v = BigInt(s.trim());
  if (v > 2n ** 64n - 1n) return { ok: false, error: "u64 範囲外" };
  return { ok: true, value: v };
}
export function parseU16(s: string): Parsed<number> {
  const r = parseU64(s);
  if (!r.ok) return r;
  if (r.value > 65535n) return { ok: false, error: "u16 範囲外" };
  return { ok: true, value: Number(r.value) };
}
export function parseBytes32Hex(s: string): Parsed<Uint8Array> {
  const hex = s.trim().replace(/^0x/i, "");
  if (!/^[0-9a-fA-F]{64}$/.test(hex)) return { ok: false, error: "32バイトのhex(64桁)を入力" };
  const out = new Uint8Array(32);
  for (let i = 0; i < 32; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return { ok: true, value: out };
}
export function parsePubkey(s: string): Parsed<Address> {
  try { return { ok: true, value: address(s.trim()) }; }
  catch { return { ok: false, error: "不正なアドレス" }; }
}

/** Labeled text input. Reports value + optional error string. */
export function TextField(props: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string; error?: string;
}) {
  return (
    <label style={{ display: "block", margin: "0.25rem 0" }}>
      <span style={{ display: "inline-block", minWidth: 180 }}>{props.label}</span>
      <input value={props.value} placeholder={props.placeholder}
        onChange={(e) => props.onChange(e.target.value)} style={{ width: 360 }} />
      {props.error && <span style={{ color: "crimson", marginLeft: 8 }}>{props.error}</span>}
    </label>
  );
}

/** Controlled string field hook to keep panels short. */
export function useField(initial = "") {
  const [value, setValue] = useState(initial);
  return { value, setValue };
}
