# Project UrthrNet

A Solana-based **decentralized advertising network** that tackles ad fraud with a
**"skin in the game"** model: publishers stake collateral to list ad slots, and fraudulent
traffic gets their stake **slashed** and paid to the advertiser. The protocol runs on a tiny
fee with no middleman markup.

> **Status:** sub-project #1 — the on-chain protocol core — is implemented. Off-chain services
> and dashboards are planned. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

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
  (`3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR`)
- **Payments:** SPL Token, single configurable mint (USDC-like)
- **Tests:** LiteSVM (Rust, in-process, real SPL Token program) + a Surfpool integration smoke test
- **Frontend:** Vite + React + `@solana/client` / `@solana/react-hooks` (in [`app/`](app/))

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
| `app/` | wallet-connect frontend — see [`app/README.md`](app/README.md) |
| `runbooks/` | Surfpool / txtx deployment runbooks |
| `docs/ARCHITECTURE.md` | system architecture & roadmap |

## Roadmap

1. ✅ Protocol core (on-chain) — this milestone
2. ⬜ Off-chain services (reporter / challenger / attestor / indexer)
3. ⬜ Advertiser & publisher dashboards (extending `app/`)
