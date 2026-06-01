# Project UrthrNet

A Solana-based **decentralized advertising network** that tackles ad fraud with a
**"skin in the game"** model: publishers stake collateral to list ad slots, and fraudulent
traffic gets their stake **slashed** and paid to the advertiser. The protocol runs on a tiny
fee with no middleman markup.

> **Status:** sub-project #1 — the on-chain protocol core — is implemented, and `app/` is a
> **localnet debug harness** that drives every instruction from the browser. Off-chain services
> and dashboards are planned. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
> [`features.md`](features.md).

## How it works

An advertiser escrows a budget per campaign. A publisher stakes collateral and submits batched
proof-of-performance **claims**. Each claim opens a **challenge window**: anyone can challenge
with fraud evidence. A configurable **attestor** resolves challenges — a valid claim **settles**
(publisher paid `amount − fee`, fee to treasury), a fraudulent one **slashes** the publisher's
stake into the advertiser's escrow. Unchallenged claims settle permissionlessly after the window.

Full design: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) ·
spec: [`docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md`](docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md)

## Tech stack

- **Program:** Anchor 1.0.2 (Rust), program `urthr_net`
  (`8CsDf7B1YU9HV136afSbYsY8eV2YeJUk5Sd2CpuLLiSb`)
- **Payments:** SPL Token, single configurable mint (USDC-like)
- **Tests:** LiteSVM (Rust, in-process, real SPL Token program) + a Surfpool integration smoke test
- **Frontend:** Vite + React + `@solana/client` / `@solana/react-hooks` — a localnet
  **debug harness** in [`app/`](app/) (one panel per instruction, simulate-before-send)

## Usage — the localnet debug harness

The fastest way to exercise the whole protocol end-to-end is the browser debug harness in
[`app/`](app/). It exposes **one panel per instruction** (all 11), four **account inspectors**,
and **localnet mint helpers**. Every panel **simulates the transaction first and shows the
result, then sends only on explicit approval**.

```bash
# 1. Start a localnet (separate shell)
NO_DNA=1 surfpool start                     # RPC :8899 / WS :8900

# 2. (Only if the IDL changed) regenerate the typed client
cd app && pnpm codegen

# 3. Run the dev server
cd app && pnpm dev                          # open http://localhost:5173
```

Then, in the page (point your wallet at the localnet RPC `http://127.0.0.1:8899`):

1. **ウォレット**: connect.
2. **localnet**: Airdrop SOL → **CreateMint** (generate a `payment_mint`) → **MintTo** (fund yourself).
3. **プロトコル設定**: `initialize_protocol` (attestor = your address, the mint you made) → check the **ProtocolConfig** inspector.
4. **パブリッシャー**: `register_publisher` → `stake`.
5. **キャンペーン**: `create_campaign` (price > 0) → `fund_campaign`.
6. **Claim**: `submit_claim` → `settle_claim` after the challenge window; or `challenge_claim` → `resolve_claim(fraud=true)` to see a slash.

Full walkthrough: [`app/README.md`](app/README.md). The harness is also the **development
contract**: any feature touching an on-chain instruction must ship a matching debug panel —
see [`CLAUDE.md`](CLAUDE.md).

## Build & test

```bash
# Build the program (produces target/deploy/urthr_net.so + IDL)
NO_DNA=1 anchor build
# If a stale deploy keypair makes the id check abort, preserve the source id:
#   NO_DNA=1 anchor build --ignore-keys

# Run the authoritative test suite (27 LiteSVM tests, full lifecycle incl. slash)
cargo test -p urthr-net

# Optional: surfnet integration smoke test (start a surfnet first)
NO_DNA=1 surfpool start      # separate shell
pnpm test:integration
```

## Layout

| Path | What |
|---|---|
| `programs/urthr-net/` | the `urthr_net` Anchor program (state, 11 instructions) + LiteSVM tests |
| `tests/lifecycle.mjs` | surfnet integration smoke test |
| `app/` | localnet **debug harness** frontend (panel per instruction) — see [`app/README.md`](app/README.md) |
| `CLAUDE.md` | development rules (incl. the mandatory debug-panel rule) |
| `features.md` | backlog of features still to build |
| `runbooks/` | Surfpool / txtx deployment runbooks |
| `docs/ARCHITECTURE.md` | system architecture & roadmap |

## Roadmap

1. ✅ Protocol core (on-chain) — this milestone
2. ⬜ Off-chain services (reporter / challenger / attestor / indexer)
3. ⬜ Advertiser & publisher dashboards (extending `app/`)
