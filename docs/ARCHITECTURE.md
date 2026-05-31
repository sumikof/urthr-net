# UrthrNet Architecture

## Vision

UrthrNet is a Solana-based decentralized advertising network — an alternative to the
incumbent ad platforms that runs autonomously on a tiny protocol fee. Its central problem
is **ad fraud** (bot clicks / fake conversions), solved with a **"skin in the game"** model:
publishers must stake collateral to list ad slots, and fraudulent traffic results in their
stake being **slashed** and used to compensate the advertiser.

## System Map

```
┌──────────────────────────────────────────────────────────────┐
│ On-chain program `urthr_net`  (this repo — IMPLEMENTED)        │
│   ① Protocol Config / Treasury                                │
│   ② Publisher Registry + Stake vault                          │
│   ③ Campaign Escrow                                           │
│   ④ Attribution & Settlement (Claim challenge/slash/settle)   │
└──────────────────────────────────────────────────────────────┘
        ▲ IDL / codegen client            ▲ signed transactions
┌────────────────────────────┐   ┌────────────────────────────────┐
│ Off-chain services (#2)     │   │ Frontend dashboards (#3)         │
│  • Reporter (aggregate +    │   │  • Advertiser dashboard          │
│    submit claims)           │   │  • Publisher dashboard           │
│  • Challenger (bot detect)  │   │  (extends existing `app/`)       │
│  • Attestor (resolve)       │   │                                  │
│  • Indexer (read state)     │   │                                  │
└────────────────────────────┘   └────────────────────────────────┘
```

Only the on-chain program is built in this milestone (sub-project #1). Off-chain services
(#2) and dashboards (#3) are planned — see the Roadmap.

## Trust Model

Off-chain fraud adjudication is abstracted behind a single configurable **`attestor`** pubkey
in `ProtocolConfig`. The on-chain state machine is independent of *how* a verdict is reached,
so the attestor role can later be replaced by a voting committee or a dispute game **without
reworking the protocol core**. For the MVP, a single trusted attestor resolves challenges.

## On-chain Accounts (PDAs)

| Account | Seeds | Key fields |
|---|---|---|
| **ProtocolConfig** | `["config"]` | `admin`, `attestor`, `payment_mint`, `treasury`, `protocol_fee_bps:u16`, `min_publisher_stake:u64`, `challenge_window:u64`, `paused:bool`, `bump` |
| **Publisher** | `["publisher", authority]` | `authority`, `stake_vault`, `staked_amount:u64`, `locked_amount:u64`, `metadata:[u8;32]`, `bump` |
| **Campaign** | `["campaign", advertiser, campaign_id]` | `advertiser`, `escrow_vault`, `campaign_id:u64`, `price_per_event:u64`, `budget_remaining:u64`, `locked_budget:u64`, `claims_count:u64`, `status`, `bump` |
| **Claim** | `["claim", campaign, claim_nonce]` | `campaign`, `publisher`, `claim_nonce:u64`, `event_count:u64`, `amount:u64`, `merkle_root:[u8;32]`, `evidence_hash:[u8;32]`, `challenger:Option<Pubkey>`, `challenge_deadline:i64`, `status`, `bump` |

Funds live in **PDA-owned SPL token vaults**: `treasury` (authority = config PDA),
`stake_vault` (authority = publisher PDA), `escrow_vault` (authority = campaign PDA). The
owning state PDA signs CPI transfers out of its vault via seeds.

**Single payment mint.** Both advertiser budgets and publisher collateral use one configurable
SPL mint (USDC-like in production), so a slashed stake compensates the advertiser in the same
currency — no price oracle needed. `claim_nonce` (a per-campaign monotonic counter,
`Campaign.claims_count`) keys each Claim PDA and is stored so claims are self-identifying.

## Claim Lifecycle

```
submit_claim (publisher signs — skin in the game)
  ├─ require campaign Active, event_count > 0
  ├─ amount = event_count * price_per_event
  ├─ require budget_remaining >= amount  → move to locked_budget
  ├─ require staked - locked >= amount    → move to publisher.locked_amount
  └─ challenge_deadline = now + challenge_window;  status = Pending
       │
   ┌───┴───────────────────────────┬──────────────────────────────────┐
   │ challenge_claim (anyone, in    │ window elapses, unchallenged       │
   │ window): records evidence_hash │ settle_claim (permissionless)      │
   ▼  status = Challenged           ▼   pay publisher amount−fee,         │
   resolve_claim (ATTESTOR only):       fee → treasury, release locks,    │
   ├─ valid → settle: pay publisher     status = Settled                  │
   │   amount−fee, fee → treasury,
   │   release locks, status = Settled
   └─ fraud → slash: move `amount` of stake_vault → escrow_vault
       (publisher compensation to advertiser); budget_remaining += amount
       (unlock) + amount (compensation); staked_amount −= amount;
       release locks; status = Slashed
```

**Locking is pure accounting.** At submit, no tokens move — `amount` is split out of
`budget_remaining` into `locked_budget`; the matching stake is marked in `locked_amount`. The
`amount` tokens stay physically in `escrow_vault`. Tokens only move on **settle** (escrow →
publisher/treasury) or **slash** (stake → escrow). This preserves the invariant
`escrow_vault balance == budget_remaining + locked_budget` at rest.

## Token & Economic Model

- **Protocol fee** = `amount * protocol_fee_bps / 10_000` (checked 128-bit math; `fee ≤ amount`).
- **Settle**: publisher receives `amount − fee`; `fee` goes to the treasury; the advertiser's
  budget is consumed (tokens leave escrow); the publisher's stake lock is released.
- **Slash**: the advertiser does **not** pay — the locked budget returns to `budget_remaining`
  **and** the publisher's slashed stake (`amount`) is added on top as compensation; the
  publisher's `staked_amount` drops by `amount`.
- All balance arithmetic is checked; on overflow the instruction errors (`MathOverflow`).

## Instructions (11)

| # | Instruction | Signer | Role |
|---|---|---|---|
| 1 | `initialize_protocol` | admin | Create Config + treasury vault; validate fee bps |
| 2 | `register_publisher` | publisher | Create Publisher + stake vault |
| 3 | `stake` | publisher | Deposit collateral |
| 4 | `unstake` | publisher | Withdraw collateral (≥ locked; full-exit or ≥ minimum) |
| 5 | `create_campaign` | advertiser | Create Campaign + escrow vault; `price_per_event > 0` |
| 6 | `fund_campaign` | advertiser | Deposit budget |
| 7 | `submit_claim` | publisher | Batched claim; lock budget + stake; open challenge window |
| 8 | `challenge_claim` | anyone | Flag fraud with evidence hash (within window) |
| 9 | `resolve_claim` | **attestor** | Adjudicate a challenged claim: settle or slash |
| 10 | `settle_claim` | anyone | Pay out an unchallenged claim after the window |
| 11 | `close_campaign` | advertiser | Refund remaining budget (only when no locked budget) |

### Security constraints (defense-in-depth)

- Vault accounts are bound by `has_one` (config↔treasury, campaign↔escrow_vault,
  publisher↔stake_vault) and validated with explicit PDA `seeds`/`bump` and `token::mint`
  checks where tokens move.
- The single-mint rule is enforced via `has_one = payment_mint` on funded instructions.
- Fund-moving instructions check `!config.paused` (emergency stop). `unstake` and
  `close_campaign` intentionally remain available during a pause — they return a user's own
  funds (exit/withdrawal).
- `merkle_root` is stored but not verified in the MVP — a hook for future per-event proofs.

## Testing

- **Unit / integration (authoritative): LiteSVM, Rust** — `programs/urthr-net/tests/`.
  27 tests run the *real* SPL Token program in-process and cover every instruction plus the
  full lifecycles: fund→stake→submit→settle, submit→challenge→resolve(slash), and
  submit→settle→close. A shared harness (`tests/common/mod.rs`) byte-packs mints/token
  accounts, derives PDAs, and bridges the multi-version `solana-pubkey` crate graph.
- **Surfnet smoke test:** `tests/lifecycle.mjs` (`pnpm test:integration`) checks a live
  surfnet + program deployment when one is running; skips cleanly otherwise.

Run: `NO_DNA=1 anchor build` then `cargo test -p urthr-net`.
(If a stale `target/deploy` keypair makes `anchor build` abort on a program-id mismatch, use
`NO_DNA=1 anchor build --ignore-keys`, which preserves the source `declare_id!`.)

## Repository Layout

```
programs/urthr-net/src/
  lib.rs                 # #[program] entrypoints, declare_id
  constants.rs           # seeds, FEE_DENOMINATOR
  error.rs               # UrthrError
  util.rs                # fee_amount
  state/                 # ProtocolConfig, Publisher, Campaign, Claim
  instructions/          # one file per instruction (11)
programs/urthr-net/tests/
  common/mod.rs          # LiteSVM harness
  *.rs                   # per-domain test suites
tests/lifecycle.mjs      # surfnet integration smoke test
app/                     # wallet-connect frontend (extended in #3)
runbooks/                # txtx / Surfpool deployment
docs/
  ARCHITECTURE.md        # this file
  superpowers/{specs,plans}/
```

`services/` (off-chain #2) and a full codegen `clients/` (#3) are not created yet — they are
added when those sub-projects begin.

## Roadmap

1. ✅ **Protocol Core** (on-chain `urthr_net`) — this milestone.
2. ⬜ **Off-chain services** — Reporter (aggregate + submit claims), Challenger (bot detection
   + submit challenges), Attestor (resolve), Indexer (read state for dashboards).
3. ⬜ **Frontend dashboards** — advertiser (create/fund/monitor campaigns) and publisher
   (register/stake/earnings), extending `app/`.

Future protocol hardening (post-MVP): merkle-proof per-event challenges, decentralized
adjudication (voting / dispute game) replacing the single attestor, multi-mint support, and
Token-2022 / confidential transfers.
