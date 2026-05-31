# UrthrNet Protocol Core (MVP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the on-chain UrthrNet protocol core — a single Anchor program with campaign escrow, publisher staking, and an attestor-adjudicated claim/challenge/slash/settle state machine.

**Architecture:** One Anchor program `urthr_net`. Four account domains (ProtocolConfig, Publisher, Campaign, Claim) namespaced by PDA seeds. Funds live in PDA-owned SPL token vaults; the owning state PDA signs CPI transfers via seeds. Off-chain fraud adjudication is abstracted behind a single configurable `attestor` pubkey in `ProtocolConfig`. Money movement uses `anchor_spl::token::transfer_checked`. "Locking" of budget/stake against a pending claim is pure accounting on the same vault balance — tokens only move on settle (escrow→publisher/treasury) or slash (stake→escrow).

**Tech Stack:** Anchor 1.0.2, anchor-spl 1.0.2, Rust 1.89, LiteSVM 0.10 (Rust unit tests loading `target/deploy/urthr_net.so`), Surfpool + `@solana/kit` (TS integration test). Build/test via `NO_DNA=1 anchor build` and `cargo test`.

**Spec:** `docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md`

---

## Conventions for the implementer

- **Build before LiteSVM tests.** LiteSVM tests `include_bytes!("../../../target/deploy/urthr_net.so")`. Always run `NO_DNA=1 anchor build` (or `cargo build-sbf`) **before** `cargo test`, and after every program change. A stale `.so` produces confusing failures.
- **anchor-spl API.** Use `anchor_spl::token::{transfer_checked, TransferChecked, Token, TokenAccount, Mint}` and `anchor_spl::associated_token::AssociatedToken`. If a method signature differs slightly in 1.0.2, follow the compiler — the logic in each handler is authoritative, the exact call shape may need a tweak.
- **Vault authority = owning state PDA.** `treasury` authority = ProtocolConfig PDA; `stake_vault` authority = Publisher PDA; `escrow_vault` authority = Campaign PDA. Transfers **out** of a vault are signed with that PDA's seeds via `CpiContext::new_with_signer`.
- **Checked math everywhere.** Use `checked_add`/`checked_sub`/`checked_mul` and map `None` to `UrthrError::MathOverflow`. Never use bare `+ - *` on balances.
- **Account space.** Derive `#[derive(InitSpace)]` on every account struct and allocate `8 + T::INIT_SPACE`.
- After each task: `git add` the listed files and commit with the given message.

---

## File Structure

| File | Responsibility |
|---|---|
| `programs/urthr-net/Cargo.toml` | Add `anchor-spl`; dev-deps `spl-token`, `spl-associated-token-account` |
| `programs/urthr-net/src/lib.rs` | `#[program]` entrypoints, module wiring, `declare_id!` |
| `programs/urthr-net/src/constants.rs` | Seeds, `FEE_DENOMINATOR` |
| `programs/urthr-net/src/error.rs` | `UrthrError` enum |
| `programs/urthr-net/src/util.rs` | `fee_amount` pure helper |
| `programs/urthr-net/src/state/mod.rs` | re-exports |
| `programs/urthr-net/src/state/protocol_config.rs` | `ProtocolConfig` |
| `programs/urthr-net/src/state/publisher.rs` | `Publisher` |
| `programs/urthr-net/src/state/campaign.rs` | `Campaign`, `CampaignStatus` |
| `programs/urthr-net/src/state/claim.rs` | `Claim`, `ClaimStatus` |
| `programs/urthr-net/src/instructions/mod.rs` | re-exports |
| `programs/urthr-net/src/instructions/<ix>.rs` | one file per instruction (11 total) |
| `programs/urthr-net/tests/common/mod.rs` | LiteSVM harness (svm, mint, token accts, PDA derivation) |
| `programs/urthr-net/tests/<ix>.rs` | LiteSVM unit tests per instruction group |
| `tests/lifecycle.test.ts` | Surfpool TS integration test |
| `docs/ARCHITECTURE.md` | system overview + roadmap |
| `README.md` | updated overview + local run/test instructions |

The existing `state.rs` (single placeholder) is deleted in Task 2 in favor of the `state/` directory. The existing `instructions/initialize.rs` is replaced by `initialize_protocol.rs` in Task 4.

---

## Task 1: Add dependencies and pure helpers (constants, errors, util)

**Files:**
- Modify: `programs/urthr-net/Cargo.toml`
- Modify: `programs/urthr-net/src/constants.rs`
- Modify: `programs/urthr-net/src/error.rs`
- Create: `programs/urthr-net/src/util.rs`
- Modify: `programs/urthr-net/src/lib.rs`

This task adds no instructions (the program still exposes only the existing `initialize`), so it stays compiling and the existing `test_initialize` keeps passing. We TDD the one piece of pure logic available now: the protocol fee calculation.

- [ ] **Step 1: Add dependencies to `Cargo.toml`**

Under `[dependencies]` add `anchor-spl`; **update the `idl-build` feature to also include `anchor-spl/idl-build`** (without this, `anchor build` IDL generation fails once anchor-spl is a dependency); under `[dev-dependencies]` add the SPL crates used to construct token state in LiteSVM tests:

```toml
[dependencies]
anchor-lang = "1.0.2"
anchor-spl = "1.0.2"
```

Change the existing `idl-build` feature line under `[features]` from
`idl-build = ["anchor-lang/idl-build"]` to:

```toml
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
```

Add the dev-dependencies:

```toml
[dev-dependencies]
litesvm = "0.10.0"
solana-message = "3.0.1"
solana-transaction = "3.0.2"
solana-signer = "3.0.0"
solana-keypair = "3.0.1"
solana-pubkey = "3.0.0"
solana-account = "3.0.0"
spl-token = "8.0.0"
spl-associated-token-account = "7.0.0"
```

(If `cargo` reports a version that does not resolve against anchor-spl 1.0.2's pinned SPL, accept the version cargo selects — these are only used to byte-pack token accounts in tests.)

- [ ] **Step 2: Write the failing test for `fee_amount`**

Create `programs/urthr-net/src/util.rs`:

```rust
use anchor_lang::prelude::*;
use crate::error::UrthrError;

/// Protocol fee = amount * fee_bps / 10_000, with checked math.
pub fn fee_amount(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee = (amount as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(UrthrError::MathOverflow)?
        .checked_div(crate::constants::FEE_DENOMINATOR as u128)
        .ok_or(UrthrError::MathOverflow)?;
    u64::try_from(fee).map_err(|_| UrthrError::MathOverflow.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_is_bps_of_amount() {
        // 0.5% of 1_000_000 = 5_000
        assert_eq!(fee_amount(1_000_000, 50).unwrap(), 5_000);
    }

    #[test]
    fn zero_bps_is_zero_fee() {
        assert_eq!(fee_amount(1_000_000, 0).unwrap(), 0);
    }

    #[test]
    fn rounds_down() {
        // 1 bps of 12_345 = 1.2345 -> 1
        assert_eq!(fee_amount(12_345, 1).unwrap(), 1);
    }
}
```

- [ ] **Step 3: Run the test to verify it fails to compile**

Run: `cargo test -p urthr-net fee_ -- --nocapture`
Expected: FAIL — `constants::FEE_DENOMINATOR` and `error::UrthrError` do not exist yet.

- [ ] **Step 4: Fill in `constants.rs`**

Replace `programs/urthr-net/src/constants.rs`:

```rust
use anchor_lang::prelude::*;

/// Fee basis-point denominator (100% = 10_000 bps).
#[constant]
pub const FEE_DENOMINATOR: u16 = 10_000;

// PDA seed prefixes.
pub const CONFIG_SEED: &[u8] = b"config";
pub const TREASURY_SEED: &[u8] = b"treasury";
pub const PUBLISHER_SEED: &[u8] = b"publisher";
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";
pub const CAMPAIGN_SEED: &[u8] = b"campaign";
pub const ESCROW_VAULT_SEED: &[u8] = b"escrow_vault";
pub const CLAIM_SEED: &[u8] = b"claim";
```

- [ ] **Step 5: Fill in `error.rs`**

Replace `programs/urthr-net/src/error.rs`:

```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum UrthrError {
    #[msg("Signer is not authorized for this action")]
    Unauthorized,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Token mint does not match the protocol payment mint")]
    InvalidMint,
    #[msg("Fee basis points exceed the denominator")]
    InvalidFeeBps,
    #[msg("Stake is below the minimum publisher stake")]
    InsufficientStake,
    #[msg("Requested unstake would drop below locked stake or minimum")]
    StakeLocked,
    #[msg("Campaign budget is insufficient for this claim")]
    InsufficientBudget,
    #[msg("Campaign is not active")]
    CampaignNotActive,
    #[msg("Event count must be greater than zero")]
    InvalidEventCount,
    #[msg("Claim is not in the Pending state")]
    ClaimNotPending,
    #[msg("Claim is not in the Challenged state")]
    ClaimNotChallenged,
    #[msg("Challenge window is still open")]
    ChallengeWindowOpen,
    #[msg("Challenge window has closed")]
    ChallengeWindowClosed,
    #[msg("Campaign still has pending claims")]
    HasPendingClaims,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
```

- [ ] **Step 6: Wire modules in `lib.rs`**

Replace `programs/urthr-net/src/lib.rs` (keeps the existing `initialize` so the program still builds; adds the new modules):

```rust
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod util;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR");

#[program]
pub mod urthr_net {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
```

> Note: `state.rs` still exists as the placeholder at this point; `state::` resolves to it. `instructions::` still resolves to the existing `initialize`. This compiles.

- [ ] **Step 7: Run the unit tests to verify they pass**

Run: `cargo test -p urthr-net fee_ -- --nocapture`
Expected: PASS (3 tests). `util` is unused by the program yet — a dead-code warning is acceptable.

- [ ] **Step 8: Build and run existing LiteSVM test**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net test_initialize`
Expected: PASS (existing initialize test still green).

- [ ] **Step 9: Commit**

```bash
git add programs/urthr-net/Cargo.toml programs/urthr-net/src/constants.rs programs/urthr-net/src/error.rs programs/urthr-net/src/util.rs programs/urthr-net/src/lib.rs Cargo.lock
git commit -m "feat: add anchor-spl dep, protocol constants, errors, and fee helper"
```

---

## Task 2: Account state structs

**Files:**
- Create: `programs/urthr-net/src/state/mod.rs`
- Create: `programs/urthr-net/src/state/protocol_config.rs`
- Create: `programs/urthr-net/src/state/publisher.rs`
- Create: `programs/urthr-net/src/state/campaign.rs`
- Create: `programs/urthr-net/src/state/claim.rs`
- Delete: `programs/urthr-net/src/state.rs`

Pure data definitions — verified by a compile + a `INIT_SPACE` sanity test. No instructions change.

- [ ] **Step 1: Write the failing test (space sanity)**

Create `programs/urthr-net/tests/state_space.rs`:

```rust
use anchor_lang::Space;
use urthr_net::state::{Campaign, Claim, ProtocolConfig, Publisher};

#[test]
fn account_space_is_nonzero_and_bounded() {
    // InitSpace excludes the 8-byte discriminator; all four must be small, fixed accounts.
    assert!(ProtocolConfig::INIT_SPACE > 0 && ProtocolConfig::INIT_SPACE < 512);
    assert!(Publisher::INIT_SPACE > 0 && Publisher::INIT_SPACE < 256);
    assert!(Campaign::INIT_SPACE > 0 && Campaign::INIT_SPACE < 256);
    assert!(Claim::INIT_SPACE > 0 && Claim::INIT_SPACE < 256);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p urthr-net account_space_is_nonzero_and_bounded`
Expected: FAIL — `urthr_net::state::ProtocolConfig` etc. do not exist.

- [ ] **Step 3: Create `state/protocol_config.rs`**

```rust
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub attestor: Pubkey,
    pub payment_mint: Pubkey,
    pub treasury: Pubkey,
    pub protocol_fee_bps: u16,
    pub min_publisher_stake: u64,
    pub challenge_window: i64,
    pub paused: bool,
    pub bump: u8,
}
```

- [ ] **Step 4: Create `state/publisher.rs`**

```rust
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Publisher {
    pub authority: Pubkey,
    pub stake_vault: Pubkey,
    pub staked_amount: u64,
    pub locked_amount: u64,
    pub metadata: [u8; 32],
    pub bump: u8,
}
```

- [ ] **Step 5: Create `state/campaign.rs`**

```rust
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum CampaignStatus {
    Active,
    Paused,
    Closed,
}

#[account]
#[derive(InitSpace)]
pub struct Campaign {
    pub advertiser: Pubkey,
    pub escrow_vault: Pubkey,
    pub campaign_id: u64,
    pub price_per_event: u64,
    pub budget_remaining: u64,
    pub locked_budget: u64,
    pub claims_count: u64,
    pub status: CampaignStatus,
    pub bump: u8,
}
```

- [ ] **Step 6: Create `state/claim.rs`**

```rust
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ClaimStatus {
    Pending,
    Challenged,
    Settled,
    Slashed,
}

#[account]
#[derive(InitSpace)]
pub struct Claim {
    pub campaign: Pubkey,
    pub publisher: Pubkey,
    pub event_count: u64,
    pub amount: u64,
    pub merkle_root: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub challenger: Option<Pubkey>,
    pub challenge_deadline: i64,
    pub status: ClaimStatus,
    pub bump: u8,
}
```

- [ ] **Step 7: Create `state/mod.rs`**

```rust
pub mod campaign;
pub mod claim;
pub mod protocol_config;
pub mod publisher;

pub use campaign::*;
pub use claim::*;
pub use protocol_config::*;
pub use publisher::*;
```

- [ ] **Step 8: Delete the old placeholder**

```bash
git rm programs/urthr-net/src/state.rs
```

(`lib.rs` already declares `pub mod state;` — it now resolves to the directory.)

- [ ] **Step 9: Build and run the space test**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net account_space_is_nonzero_and_bounded`
Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add programs/urthr-net/src/state programs/urthr-net/tests/state_space.rs
git commit -m "feat: add ProtocolConfig, Publisher, Campaign, Claim account state"
```

---

## Task 3: LiteSVM test harness

**Files:**
- Create: `programs/urthr-net/tests/common/mod.rs`

A shared harness so each instruction test stays small. It builds a `LiteSVM`, loads the program, funds a payer, creates an SPL mint and token accounts by byte-packing state directly (no need to invoke the token program for setup), and derives all PDAs. LiteSVM `new()` includes the SPL Token program, so the program-under-test's CPI transfers succeed.

- [ ] **Step 1: Create the harness**

```rust
#![allow(dead_code)]
use {
    anchor_lang::{
        solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_account::Account,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_token::state::{Account as SplAccount, AccountState, Mint},
};

pub const ONE_TOKEN: u64 = 1_000_000; // 6-decimals, USDC-like

pub struct Env {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub mint: Pubkey,
}

impl Env {
    pub fn new() -> Self {
        let program_id = urthr_net::id();
        let mut svm = LiteSVM::new();
        let bytes = include_bytes!("../../../target/deploy/urthr_net.so");
        svm.add_program(program_id, bytes).unwrap();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();

        let mut env = Self { svm, program_id, payer, mint: Pubkey::default() };
        env.mint = env.create_mint(6);
        env
    }

    /// Pack an SPL mint directly into an account.
    pub fn create_mint(&mut self, decimals: u8) -> Pubkey {
        let mint = Pubkey::new_unique();
        let mut data = vec![0u8; Mint::LEN];
        let state = Mint {
            mint_authority: solana_program_option_some(self.payer.pubkey()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: solana_program_option_none(),
        };
        Mint::pack(state, &mut data).unwrap();
        self.svm.set_account(mint, Account {
            lamports: self.svm.minimum_balance_for_rent_exemption(Mint::LEN),
            data,
            owner: spl_token::ID,
            executable: false,
            rent_epoch: 0,
        }).unwrap();
        mint
    }

    /// Pack an SPL token account holding `amount` of `self.mint`, owned by `owner`.
    pub fn create_token_account(&mut self, owner: &Pubkey, amount: u64) -> Pubkey {
        let ata = Pubkey::new_unique();
        self.set_token_account(ata, owner, amount);
        ata
    }

    /// Set token-account state at an explicit address (use for PDA vaults if pre-seeding).
    pub fn set_token_account(&mut self, address: Pubkey, owner: &Pubkey, amount: u64) {
        let mut data = vec![0u8; SplAccount::LEN];
        let state = SplAccount {
            mint: self.mint,
            owner: *owner,
            amount,
            delegate: solana_program_option_none(),
            state: AccountState::Initialized,
            is_native: solana_program_option_none(),
            delegated_amount: 0,
            close_authority: solana_program_option_none(),
        };
        SplAccount::pack(state, &mut data).unwrap();
        self.svm.set_account(address, Account {
            lamports: self.svm.minimum_balance_for_rent_exemption(SplAccount::LEN),
            data,
            owner: spl_token::ID,
            executable: false,
            rent_epoch: 0,
        }).unwrap();
    }

    pub fn token_balance(&self, address: &Pubkey) -> u64 {
        let acct = self.svm.get_account(address).unwrap();
        SplAccount::unpack(&acct.data).unwrap().amount
    }

    pub fn get<T: anchor_lang::AccountDeserialize>(&self, address: &Pubkey) -> T {
        let acct = self.svm.get_account(address).unwrap();
        T::try_deserialize(&mut acct.data.as_slice()).unwrap()
    }

    /// Send a single instruction signed by `signers` (payer auto-included as fee payer).
    pub fn send(
        &mut self,
        data: impl InstructionData,
        accounts: impl ToAccountMetas,
        extra_signers: &[&Keypair],
    ) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let ix = Instruction::new_with_bytes(
            self.program_id,
            &data.data(),
            accounts.to_account_metas(None),
        );
        let blockhash = self.svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[ix], Some(&self.payer.pubkey()), &blockhash);
        let mut signers = vec![&self.payer];
        signers.extend_from_slice(extra_signers);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
        self.svm.send_transaction(tx).map(|_| ())
    }

    pub fn warp_unix(&mut self, seconds: i64) {
        let mut clock = self.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
        clock.unix_timestamp += seconds;
        self.svm.set_sysvar(&clock);
    }
}

// COption helpers (spl-token re-exports solana_program::program_option::COption).
fn solana_program_option_some(p: Pubkey) -> spl_token::solana_program::program_option::COption<spl_token::solana_program::pubkey::Pubkey> {
    spl_token::solana_program::program_option::COption::Some(to_spl_pubkey(p))
}
fn solana_program_option_none() -> spl_token::solana_program::program_option::COption<spl_token::solana_program::pubkey::Pubkey> {
    spl_token::solana_program::program_option::COption::None
}
fn to_spl_pubkey(p: Pubkey) -> spl_token::solana_program::pubkey::Pubkey {
    spl_token::solana_program::pubkey::Pubkey::new_from_array(p.to_bytes())
}

// PDA derivation helpers shared by tests.
pub fn config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::CONFIG_SEED], program_id)
}
pub fn treasury_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::TREASURY_SEED], program_id)
}
pub fn publisher_pda(program_id: &Pubkey, authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::PUBLISHER_SEED, authority.as_ref()], program_id)
}
pub fn stake_vault_pda(program_id: &Pubkey, authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::STAKE_VAULT_SEED, authority.as_ref()], program_id)
}
pub fn campaign_pda(program_id: &Pubkey, advertiser: &Pubkey, id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::CAMPAIGN_SEED, advertiser.as_ref(), &id.to_le_bytes()], program_id)
}
pub fn escrow_vault_pda(program_id: &Pubkey, campaign: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::ESCROW_VAULT_SEED, campaign.as_ref()], program_id)
}
pub fn claim_pda(program_id: &Pubkey, campaign: &Pubkey, nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[urthr_net::constants::CLAIM_SEED, campaign.as_ref(), &nonce.to_le_bytes()], program_id)
}
```

> Implementer note: the `COption`/pubkey-bridging helpers above work around spl-token using its own re-exported `solana_program` types. If `spl-token` 8.x exposes `solana_pubkey::Pubkey` directly, simplify by removing `to_spl_pubkey`. Follow the compiler; the goal is a packed mint/token account owned by `spl_token::ID`.

- [ ] **Step 2: Verify it compiles (no tests yet reference it)**

Run: `cargo test -p urthr-net --no-run`
Expected: Compiles. `common` is only built when a test references it; to force a check, proceed to Task 4 which uses it. If you want an immediate check, temporarily add `mod common;` to `state_space.rs`, confirm compile, then remove.

- [ ] **Step 3: Commit**

```bash
git add programs/urthr-net/tests/common
git commit -m "test: add LiteSVM harness for SPL mint, token accounts, and PDAs"
```

---

## Task 4: `initialize_protocol`

**Files:**
- Create: `programs/urthr-net/src/instructions/initialize_protocol.rs`
- Delete: `programs/urthr-net/src/instructions/initialize.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Delete: `programs/urthr-net/tests/test_initialize.rs`
- Create: `programs/urthr-net/tests/initialize_protocol.rs`

Creates the singleton `ProtocolConfig` PDA and the `treasury` token vault (authority = config PDA). Validates `protocol_fee_bps <= FEE_DENOMINATOR`.

- [ ] **Step 1: Write the failing test**

Create `programs/urthr-net/tests/initialize_protocol.rs`:

```rust
mod common;
use common::*;
use urthr_net::state::ProtocolConfig;

#[test]
fn initializes_protocol_config() {
    let mut env = Env::new();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    let attestor = solana_keypair::Keypair::new().pubkey_();

    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor,
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: env.payer.pubkey(),
            config,
            treasury,
            payment_mint: env.mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();

    let cfg: ProtocolConfig = env.get(&config);
    assert_eq!(cfg.admin, env.payer.pubkey());
    assert_eq!(cfg.attestor, attestor);
    assert_eq!(cfg.payment_mint, env.mint);
    assert_eq!(cfg.protocol_fee_bps, 50);
    assert!(!cfg.paused);
    assert_eq!(env.token_balance(&treasury), 0);
}

#[test]
fn rejects_fee_bps_above_denominator() {
    let mut env = Env::new();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    let res = env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: env.payer.pubkey(),
            protocol_fee_bps: 10_001,
            min_publisher_stake: 0,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: env.payer.pubkey(),
            config,
            treasury,
            payment_mint: env.mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    );
    assert!(res.is_err());
}

// Small helper to read a fresh pubkey in tests.
trait PubkeyExt { fn pubkey_(&self) -> solana_pubkey::Pubkey; }
impl PubkeyExt for solana_keypair::Keypair {
    fn pubkey_(&self) -> solana_pubkey::Pubkey { solana_signer::Signer::pubkey(self) }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p urthr-net --test initialize_protocol`
Expected: FAIL — `InitializeProtocol` instruction/accounts do not exist.

- [ ] **Step 3: Implement the instruction**

Create `programs/urthr-net/src/instructions/initialize_protocol.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::ProtocolConfig;

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = admin,
        seeds = [TREASURY_SEED],
        bump,
        token::mint = payment_mint,
        token::authority = config,
    )]
    pub treasury: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeProtocol>,
    attestor: Pubkey,
    protocol_fee_bps: u16,
    min_publisher_stake: u64,
    challenge_window: i64,
) -> Result<()> {
    require!(protocol_fee_bps <= FEE_DENOMINATOR, UrthrError::InvalidFeeBps);

    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.attestor = attestor;
    config.payment_mint = ctx.accounts.payment_mint.key();
    config.treasury = ctx.accounts.treasury.key();
    config.protocol_fee_bps = protocol_fee_bps;
    config.min_publisher_stake = min_publisher_stake;
    config.challenge_window = challenge_window;
    config.paused = false;
    config.bump = ctx.bumps.config;
    Ok(())
}
```

- [ ] **Step 4: Replace the old `initialize` module and wire `lib.rs`**

Delete the old instruction and test:

```bash
git rm programs/urthr-net/src/instructions/initialize.rs programs/urthr-net/tests/test_initialize.rs
```

Replace `programs/urthr-net/src/instructions/mod.rs`:

```rust
pub mod initialize_protocol;

pub use initialize_protocol::*;
```

Replace the `#[program]` block in `programs/urthr-net/src/lib.rs`:

```rust
#[program]
pub mod urthr_net {
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        attestor: Pubkey,
        protocol_fee_bps: u16,
        min_publisher_stake: u64,
        challenge_window: i64,
    ) -> Result<()> {
        initialize_protocol::handler(ctx, attestor, protocol_fee_bps, min_publisher_stake, challenge_window)
    }
}
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test initialize_protocol`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/initialize_protocol.rs
git commit -m "feat: add initialize_protocol instruction with config and treasury vault"
```

---

## Task 5: `register_publisher`

**Files:**
- Create: `programs/urthr-net/src/instructions/register_publisher.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Create: `programs/urthr-net/tests/publisher.rs`

Creates a `Publisher` PDA and its `stake_vault` (authority = publisher PDA). `metadata` is a caller-supplied 32-byte identifier.

- [ ] **Step 1: Write the failing test**

Create `programs/urthr-net/tests/publisher.rs`:

```rust
mod common;
use common::*;
use urthr_net::state::Publisher;

fn init_protocol(env: &mut Env) {
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: env.payer.pubkey(),
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: env.payer.pubkey(),
            config,
            treasury,
            payment_mint: env.mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();
}

#[test]
fn registers_a_publisher() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let (config, _) = config_pda(&env.program_id);
    let authority = env.payer.pubkey();
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);

    env.send(
        urthr_net::instruction::RegisterPublisher { metadata: [7u8; 32] },
        urthr_net::accounts::RegisterPublisher {
            authority,
            config,
            publisher,
            stake_vault,
            payment_mint: env.mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();

    let p: Publisher = env.get(&publisher);
    assert_eq!(p.authority, authority);
    assert_eq!(p.staked_amount, 0);
    assert_eq!(p.locked_amount, 0);
    assert_eq!(p.metadata, [7u8; 32]);
    assert_eq!(env.token_balance(&stake_vault), 0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p urthr-net --test publisher`
Expected: FAIL — `RegisterPublisher` does not exist.

- [ ] **Step 3: Implement the instruction**

Create `programs/urthr-net/src/instructions/register_publisher.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Publisher, ProtocolConfig};

#[derive(Accounts)]
pub struct RegisterPublisher<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + Publisher::INIT_SPACE,
        seeds = [PUBLISHER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(
        init,
        payer = authority,
        seeds = [STAKE_VAULT_SEED, authority.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = publisher,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<RegisterPublisher>, metadata: [u8; 32]) -> Result<()> {
    let publisher = &mut ctx.accounts.publisher;
    publisher.authority = ctx.accounts.authority.key();
    publisher.stake_vault = ctx.accounts.stake_vault.key();
    publisher.staked_amount = 0;
    publisher.locked_amount = 0;
    publisher.metadata = metadata;
    publisher.bump = ctx.bumps.publisher;
    Ok(())
}
```

> Note: `has_one = payment_mint` requires the field name on `ProtocolConfig` to be `payment_mint` (it is) and the passed account to match — this enforces the single-mint rule on every funded instruction.

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `programs/urthr-net/src/instructions/mod.rs`:

```rust
pub mod register_publisher;
pub use register_publisher::*;
```

Add inside the `#[program]` module in `lib.rs`:

```rust
    pub fn register_publisher(ctx: Context<RegisterPublisher>, metadata: [u8; 32]) -> Result<()> {
        register_publisher::handler(ctx, metadata)
    }
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test publisher`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src/instructions programs/urthr-net/src/lib.rs programs/urthr-net/tests/publisher.rs
git commit -m "feat: add register_publisher instruction with stake vault"
```

---

## Task 6: `stake` and `unstake`

**Files:**
- Create: `programs/urthr-net/src/instructions/stake.rs`
- Create: `programs/urthr-net/src/instructions/unstake.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Modify: `programs/urthr-net/tests/publisher.rs`

`stake`: transfer `amount` from the publisher's token account into `stake_vault` (user signs), increment `staked_amount`. `unstake`: transfer `amount` out (publisher PDA signs), requiring `staked_amount - locked_amount >= amount` **and** the remaining stake stays `>= min_publisher_stake` (unless withdrawing to exactly zero — see rule below).

**Unstake rule (explicit to avoid ambiguity):** allow if `staked_amount - amount >= locked_amount` AND (`staked_amount - amount == 0` OR `staked_amount - amount >= config.min_publisher_stake`). I.e. a publisher may withdraw everything (full exit) or leave at least the minimum, but cannot leave a nonzero sub-minimum balance, and never below locked.

- [ ] **Step 1: Add the failing tests**

Append to `programs/urthr-net/tests/publisher.rs`:

```rust
fn register(env: &mut Env) -> (solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let authority = env.payer.pubkey();
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);
    let (config, _) = config_pda(&env.program_id);
    env.send(
        urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
        urthr_net::accounts::RegisterPublisher {
            authority, config, publisher, stake_vault,
            payment_mint: env.mint, token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();
    (publisher, stake_vault)
}

#[test]
fn stake_and_unstake_round_trip() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let authority = env.payer.pubkey();
    let (publisher, stake_vault) = register(&mut env);
    let user_ata = env.create_token_account(&authority, 100 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);

    // stake 30
    env.send(
        urthr_net::instruction::Stake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();
    assert_eq!(env.token_balance(&stake_vault), 30 * ONE_TOKEN);
    assert_eq!(env.get::<urthr_net::state::Publisher>(&publisher).staked_amount, 30 * ONE_TOKEN);

    // unstake 30 (full exit allowed)
    env.send(
        urthr_net::instruction::Unstake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Unstake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();
    assert_eq!(env.token_balance(&stake_vault), 0);
    assert_eq!(env.token_balance(&user_ata), 100 * ONE_TOKEN);
}

#[test]
fn unstake_below_minimum_but_nonzero_is_rejected() {
    let mut env = Env::new();
    init_protocol(&mut env); // min stake = 10 tokens
    let authority = env.payer.pubkey();
    let (publisher, stake_vault) = register(&mut env);
    let user_ata = env.create_token_account(&authority, 100 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);
    env.send(
        urthr_net::instruction::Stake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();
    // leaving 5 (< 10 min, nonzero) must fail
    let res = env.send(
        urthr_net::instruction::Unstake { amount: 25 * ONE_TOKEN },
        urthr_net::accounts::Unstake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    );
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test publisher`
Expected: FAIL — `Stake`/`Unstake` do not exist.

- [ ] **Step 3: Implement `stake.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{ProtocolConfig, Publisher};

#[derive(Accounts)]
pub struct Stake<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, authority.key().as_ref()],
        bump = publisher.bump,
        has_one = authority @ UrthrError::Unauthorized,
        has_one = stake_vault,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = authority)]
    pub authority_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Stake>, amount: u64) -> Result<()> {
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.authority_token_account.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.payment_mint.decimals,
    )?;

    let publisher = &mut ctx.accounts.publisher;
    publisher.staked_amount = publisher.staked_amount
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;
    Ok(())
}
```

- [ ] **Step 4: Implement `unstake.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{ProtocolConfig, Publisher};

#[derive(Accounts)]
pub struct Unstake<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, authority.key().as_ref()],
        bump = publisher.bump,
        has_one = authority @ UrthrError::Unauthorized,
        has_one = stake_vault,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = authority)]
    pub authority_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    let publisher = &ctx.accounts.publisher;
    let remaining = publisher.staked_amount
        .checked_sub(amount).ok_or(UrthrError::InsufficientStake)?;
    require!(remaining >= publisher.locked_amount, UrthrError::StakeLocked);
    require!(
        remaining == 0 || remaining >= ctx.accounts.config.min_publisher_stake,
        UrthrError::StakeLocked
    );

    let authority_key = ctx.accounts.authority.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        PUBLISHER_SEED,
        authority_key.as_ref(),
        &[publisher.bump],
    ]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.stake_vault.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.authority_token_account.to_account_info(),
                authority: ctx.accounts.publisher.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        ctx.accounts.payment_mint.decimals,
    )?;

    let publisher = &mut ctx.accounts.publisher;
    publisher.staked_amount = remaining;
    Ok(())
}
```

- [ ] **Step 5: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod stake;
pub mod unstake;
pub use stake::*;
pub use unstake::*;
```

Add to `#[program]`:

```rust
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        stake::handler(ctx, amount)
    }
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        unstake::handler(ctx, amount)
    }
```

- [ ] **Step 6: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test publisher`
Expected: PASS (4 tests in the file).

- [ ] **Step 7: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/publisher.rs
git commit -m "feat: add stake and unstake with locked/minimum guards"
```

---

## Task 7: `create_campaign` and `fund_campaign`

**Files:**
- Create: `programs/urthr-net/src/instructions/create_campaign.rs`
- Create: `programs/urthr-net/src/instructions/fund_campaign.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Create: `programs/urthr-net/tests/campaign.rs`

`create_campaign`: create `Campaign` PDA (seed includes `campaign_id`) + `escrow_vault` (authority = campaign PDA), set `price_per_event`, `status = Active`. `fund_campaign`: transfer `amount` from advertiser → `escrow_vault`, `budget_remaining += amount`.

- [ ] **Step 1: Write the failing test**

Create `programs/urthr-net/tests/campaign.rs`:

```rust
mod common;
use common::*;
use urthr_net::state::{Campaign, CampaignStatus};

pub fn init_protocol(env: &mut Env) {
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: env.payer.pubkey(),
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: env.payer.pubkey(), config, treasury,
            payment_mint: env.mint, token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();
}

pub fn create_campaign(env: &mut Env, id: u64, price: u64) -> (solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let advertiser = env.payer.pubkey();
    let (config, _) = config_pda(&env.program_id);
    let (campaign, _) = campaign_pda(&env.program_id, &advertiser, id);
    let (escrow_vault, _) = escrow_vault_pda(&env.program_id, &campaign);
    env.send(
        urthr_net::instruction::CreateCampaign { campaign_id: id, price_per_event: price },
        urthr_net::accounts::CreateCampaign {
            advertiser, config, campaign, escrow_vault,
            payment_mint: env.mint, token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();
    (campaign, escrow_vault)
}

pub fn fund_campaign(env: &mut Env, campaign: solana_pubkey::Pubkey, escrow_vault: solana_pubkey::Pubkey, src: solana_pubkey::Pubkey, amount: u64) {
    let advertiser = env.payer.pubkey();
    let (config, _) = config_pda(&env.program_id);
    env.send(
        urthr_net::instruction::FundCampaign { amount },
        urthr_net::accounts::FundCampaign {
            advertiser, config, campaign, escrow_vault,
            advertiser_token_account: src,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();
}

#[test]
fn create_and_fund_campaign() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let advertiser = env.payer.pubkey();
    let src = env.create_token_account(&advertiser, 1_000 * ONE_TOKEN);
    let (campaign, escrow_vault) = create_campaign(&mut env, 1, 2 * ONE_TOKEN);

    let c: Campaign = env.get(&campaign);
    assert_eq!(c.advertiser, advertiser);
    assert_eq!(c.price_per_event, 2 * ONE_TOKEN);
    assert_eq!(c.budget_remaining, 0);
    assert!(c.status == CampaignStatus::Active);

    fund_campaign(&mut env, campaign, escrow_vault, src, 500 * ONE_TOKEN);
    assert_eq!(env.token_balance(&escrow_vault), 500 * ONE_TOKEN);
    assert_eq!(env.get::<Campaign>(&campaign).budget_remaining, 500 * ONE_TOKEN);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test campaign`
Expected: FAIL — instructions missing.

- [ ] **Step 3: Implement `create_campaign.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub advertiser: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = advertiser,
        space = 8 + Campaign::INIT_SPACE,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign_id.to_le_bytes()],
        bump,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        init,
        payer = advertiser,
        seeds = [ESCROW_VAULT_SEED, campaign.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = campaign,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateCampaign>, campaign_id: u64, price_per_event: u64) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    campaign.advertiser = ctx.accounts.advertiser.key();
    campaign.escrow_vault = ctx.accounts.escrow_vault.key();
    campaign.campaign_id = campaign_id;
    campaign.price_per_event = price_per_event;
    campaign.budget_remaining = 0;
    campaign.locked_budget = 0;
    campaign.claims_count = 0;
    campaign.status = CampaignStatus::Active;
    campaign.bump = ctx.bumps.campaign;
    Ok(())
}
```

- [ ] **Step 4: Implement `fund_campaign.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    pub advertiser: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = advertiser @ UrthrError::Unauthorized,
        has_one = escrow_vault,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = advertiser)]
    pub advertiser_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.advertiser_token_account.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.advertiser.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.payment_mint.decimals,
    )?;

    let campaign = &mut ctx.accounts.campaign;
    campaign.budget_remaining = campaign.budget_remaining
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;
    Ok(())
}
```

- [ ] **Step 5: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod create_campaign;
pub mod fund_campaign;
pub use create_campaign::*;
pub use fund_campaign::*;
```

Add to `#[program]`:

```rust
    pub fn create_campaign(ctx: Context<CreateCampaign>, campaign_id: u64, price_per_event: u64) -> Result<()> {
        create_campaign::handler(ctx, campaign_id, price_per_event)
    }
    pub fn fund_campaign(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
        fund_campaign::handler(ctx, amount)
    }
```

- [ ] **Step 6: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test campaign`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/campaign.rs
git commit -m "feat: add create_campaign and fund_campaign with escrow vault"
```

---

## Task 8: `submit_claim`

**Files:**
- Create: `programs/urthr-net/src/instructions/submit_claim.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Create: `programs/urthr-net/tests/claim.rs`

Publisher submits a batched claim. Validates: campaign Active, `event_count > 0`, `amount = event_count * price_per_event`, `budget_remaining >= amount`, and `staked_amount - locked_amount >= amount`. Moves `amount` from `budget_remaining → locked_budget` and into publisher `locked_amount`, sets `challenge_deadline = now + challenge_window`, creates `Claim` at nonce `campaign.claims_count`, then increments `claims_count`.

- [ ] **Step 1: Write the failing test**

Create `programs/urthr-net/tests/claim.rs`:

```rust
mod common;
use common::*;
use urthr_net::state::{Campaign, Claim, ClaimStatus, Publisher};

// Reuse helpers by duplicating minimal setup (tests are independent crates).
fn setup(env: &mut Env) -> Setup {
    let admin = env.payer.pubkey();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: admin, protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN, challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin, config, treasury, payment_mint: env.mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();

    let authority = admin; // payer acts as both publisher and advertiser in unit tests
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);
    env.send(
        urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
        urthr_net::accounts::RegisterPublisher {
            authority, config, publisher, stake_vault,
            payment_mint: env.mint, token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        &[],
    ).unwrap();

    let wallet = env.create_token_account(&authority, 1_000 * ONE_TOKEN);
    // stake 100
    env.send(
        urthr_net::instruction::Stake { amount: 100 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    let (campaign, escrow_vault) = {
        let (c, _) = campaign_pda(&env.program_id, &authority, 1);
        let (e, _) = escrow_vault_pda(&env.program_id, &c);
        env.send(
            urthr_net::instruction::CreateCampaign { campaign_id: 1, price_per_event: 1 * ONE_TOKEN },
            urthr_net::accounts::CreateCampaign {
                advertiser: authority, config, campaign: c, escrow_vault: e,
                payment_mint: env.mint, token_program: spl_token::ID,
                system_program: anchor_lang::system_program::ID,
                rent: anchor_lang::solana_program::sysvar::rent::ID,
            },
            &[],
        ).unwrap();
        (c, e)
    };
    // fund 200
    env.send(
        urthr_net::instruction::FundCampaign { amount: 200 * ONE_TOKEN },
        urthr_net::accounts::FundCampaign {
            advertiser: authority, config, campaign, escrow_vault,
            advertiser_token_account: wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    Setup { authority, config, publisher, stake_vault, campaign, escrow_vault, treasury, wallet }
}

struct Setup {
    authority: solana_pubkey::Pubkey,
    config: solana_pubkey::Pubkey,
    publisher: solana_pubkey::Pubkey,
    stake_vault: solana_pubkey::Pubkey,
    campaign: solana_pubkey::Pubkey,
    escrow_vault: solana_pubkey::Pubkey,
    treasury: solana_pubkey::Pubkey,
    wallet: solana_pubkey::Pubkey,
}

fn submit(env: &mut Env, s: &Setup, nonce: u64, events: u64) -> solana_pubkey::Pubkey {
    let (claim, _) = claim_pda(&env.program_id, &s.campaign, nonce);
    env.send(
        urthr_net::instruction::SubmitClaim { event_count: events, merkle_root: [9u8; 32] },
        urthr_net::accounts::SubmitClaim {
            publisher_authority: s.authority,
            config: s.config, publisher: s.publisher, campaign: s.campaign, claim,
            system_program: anchor_lang::system_program::ID,
        },
        &[],
    ).unwrap();
    claim
}

#[test]
fn submit_claim_locks_budget_and_stake() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40); // 40 events * 1 token = 40

    let c: Claim = env.get(&claim);
    assert_eq!(c.amount, 40 * ONE_TOKEN);
    assert_eq!(c.event_count, 40);
    assert!(c.status == ClaimStatus::Pending);
    assert_eq!(c.merkle_root, [9u8; 32]);

    let camp: Campaign = env.get(&s.campaign);
    assert_eq!(camp.budget_remaining, 160 * ONE_TOKEN);
    assert_eq!(camp.locked_budget, 40 * ONE_TOKEN);
    assert_eq!(camp.claims_count, 1);

    let pubr: Publisher = env.get(&s.publisher);
    assert_eq!(pubr.locked_amount, 40 * ONE_TOKEN);
}

#[test]
fn submit_claim_rejects_zero_events() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let (claim, _) = claim_pda(&env.program_id, &s.campaign, 0);
    let res = env.send(
        urthr_net::instruction::SubmitClaim { event_count: 0, merkle_root: [0u8; 32] },
        urthr_net::accounts::SubmitClaim {
            publisher_authority: s.authority,
            config: s.config, publisher: s.publisher, campaign: s.campaign, claim,
            system_program: anchor_lang::system_program::ID,
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn submit_claim_rejects_over_budget() {
    let mut env = Env::new();
    let s = setup(&mut env);
    // 300 events > 200 budget
    let (claim, _) = claim_pda(&env.program_id, &s.campaign, 0);
    let res = env.send(
        urthr_net::instruction::SubmitClaim { event_count: 300, merkle_root: [0u8; 32] },
        urthr_net::accounts::SubmitClaim {
            publisher_authority: s.authority,
            config: s.config, publisher: s.publisher, campaign: s.campaign, claim,
            system_program: anchor_lang::system_program::ID,
        },
        &[],
    );
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test claim`
Expected: FAIL — `SubmitClaim` missing.

- [ ] **Step 3: Implement `submit_claim.rs`**

```rust
use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, Claim, ClaimStatus, ProtocolConfig, Publisher};

#[derive(Accounts)]
pub struct SubmitClaim<'info> {
    #[account(mut)]
    pub publisher_authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, publisher_authority.key().as_ref()],
        bump = publisher.bump,
        constraint = publisher.authority == publisher_authority.key() @ UrthrError::Unauthorized,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.advertiser.as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        init,
        payer = publisher_authority,
        space = 8 + Claim::INIT_SPACE,
        seeds = [CLAIM_SEED, campaign.key().as_ref(), &campaign.claims_count.to_le_bytes()],
        bump,
    )]
    pub claim: Account<'info, Claim>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SubmitClaim>, event_count: u64, merkle_root: [u8; 32]) -> Result<()> {
    require!(event_count > 0, UrthrError::InvalidEventCount);

    let campaign = &mut ctx.accounts.campaign;
    let publisher = &mut ctx.accounts.publisher;

    let amount = event_count
        .checked_mul(campaign.price_per_event).ok_or(UrthrError::MathOverflow)?;

    require!(campaign.budget_remaining >= amount, UrthrError::InsufficientBudget);
    let available_stake = publisher.staked_amount
        .checked_sub(publisher.locked_amount).ok_or(UrthrError::MathOverflow)?;
    require!(available_stake >= amount, UrthrError::InsufficientStake);

    // Lock budget (pure accounting on the escrow balance).
    campaign.budget_remaining = campaign.budget_remaining
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    campaign.locked_budget = campaign.locked_budget
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;
    // Lock stake.
    publisher.locked_amount = publisher.locked_amount
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;

    let now = Clock::get()?.unix_timestamp;
    let claim = &mut ctx.accounts.claim;
    claim.campaign = campaign.key();
    claim.publisher = publisher.key();
    claim.event_count = event_count;
    claim.amount = amount;
    claim.merkle_root = merkle_root;
    claim.evidence_hash = [0u8; 32];
    claim.challenger = None;
    claim.challenge_deadline = now
        .checked_add(ctx.accounts.config.challenge_window).ok_or(UrthrError::MathOverflow)?;
    claim.status = ClaimStatus::Pending;
    claim.bump = ctx.bumps.claim;

    campaign.claims_count = campaign.claims_count
        .checked_add(1).ok_or(UrthrError::MathOverflow)?;
    Ok(())
}
```

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod submit_claim;
pub use submit_claim::*;
```

Add to `#[program]`:

```rust
    pub fn submit_claim(ctx: Context<SubmitClaim>, event_count: u64, merkle_root: [u8; 32]) -> Result<()> {
        submit_claim::handler(ctx, event_count, merkle_root)
    }
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test claim`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/claim.rs
git commit -m "feat: add submit_claim with budget and stake locking"
```

---

## Task 9: `challenge_claim`

**Files:**
- Create: `programs/urthr-net/src/instructions/challenge_claim.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Modify: `programs/urthr-net/tests/claim.rs`

Permissionless within the window. Requires `status == Pending` and `now <= challenge_deadline`. Records `challenger` and `evidence_hash`, sets `status = Challenged`.

- [ ] **Step 1: Add the failing test**

Append to `programs/urthr-net/tests/claim.rs`:

```rust
fn challenge(env: &mut Env, claim: solana_pubkey::Pubkey, challenger: &solana_keypair::Keypair) {
    use solana_signer::Signer;
    env.svm.airdrop(&challenger.pubkey(), 1_000_000_000).unwrap();
    env.send(
        urthr_net::instruction::ChallengeClaim { evidence_hash: [3u8; 32] },
        urthr_net::accounts::ChallengeClaim {
            challenger: challenger.pubkey(),
            claim,
        },
        &[challenger],
    ).unwrap();
}

#[test]
fn challenge_moves_claim_to_challenged() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    let challenger = solana_keypair::Keypair::new();
    challenge(&mut env, claim, &challenger);

    let c: Claim = env.get(&claim);
    assert!(c.status == ClaimStatus::Challenged);
    assert_eq!(c.evidence_hash, [3u8; 32]);
    assert_eq!(c.challenger, Some(challenger.pubkey_()));
}

#[test]
fn challenge_after_window_is_rejected() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    env.warp_unix(4000); // window was 3600
    let challenger = solana_keypair::Keypair::new();
    use solana_signer::Signer;
    env.svm.airdrop(&challenger.pubkey(), 1_000_000_000).unwrap();
    let res = env.send(
        urthr_net::instruction::ChallengeClaim { evidence_hash: [3u8; 32] },
        urthr_net::accounts::ChallengeClaim { challenger: challenger.pubkey(), claim },
        &[&challenger],
    );
    assert!(res.is_err());
}

// reuse the pubkey helper
trait PubkeyExt2 { fn pubkey_(&self) -> solana_pubkey::Pubkey; }
impl PubkeyExt2 for solana_keypair::Keypair {
    fn pubkey_(&self) -> solana_pubkey::Pubkey { solana_signer::Signer::pubkey(self) }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test claim`
Expected: FAIL — `ChallengeClaim` missing.

- [ ] **Step 3: Implement `challenge_claim.rs`**

```rust
use anchor_lang::prelude::*;
use crate::error::UrthrError;
use crate::state::{Claim, ClaimStatus};

#[derive(Accounts)]
pub struct ChallengeClaim<'info> {
    #[account(mut)]
    pub challenger: Signer<'info>,

    #[account(mut)]
    pub claim: Account<'info, Claim>,
}

pub fn handler(ctx: Context<ChallengeClaim>, evidence_hash: [u8; 32]) -> Result<()> {
    let claim = &mut ctx.accounts.claim;
    require!(claim.status == ClaimStatus::Pending, UrthrError::ClaimNotPending);
    let now = Clock::get()?.unix_timestamp;
    require!(now <= claim.challenge_deadline, UrthrError::ChallengeWindowClosed);

    claim.status = ClaimStatus::Challenged;
    claim.challenger = Some(ctx.accounts.challenger.key());
    claim.evidence_hash = evidence_hash;
    Ok(())
}
```

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod challenge_claim;
pub use challenge_claim::*;
```

Add to `#[program]`:

```rust
    pub fn challenge_claim(ctx: Context<ChallengeClaim>, evidence_hash: [u8; 32]) -> Result<()> {
        challenge_claim::handler(ctx, evidence_hash)
    }
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test claim`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/claim.rs
git commit -m "feat: add challenge_claim within challenge window"
```

---

## Task 10: `settle_claim` (permissionless, unchallenged)

**Files:**
- Create: `programs/urthr-net/src/instructions/settle_claim.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Modify: `programs/urthr-net/tests/claim.rs`

Permissionless after the window. Requires `status == Pending` and `now > challenge_deadline`. Pays `amount - fee` from escrow → publisher wallet, `fee` → treasury (both signed by Campaign PDA), consumes `locked_budget`, releases publisher `locked_amount`, sets `status = Settled`.

- [ ] **Step 1: Add the failing test**

Append to `programs/urthr-net/tests/claim.rs`:

```rust
#[test]
fn settle_unchallenged_pays_publisher_and_fee() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40); // amount 40, fee 0.5% = 0.2 token

    env.warp_unix(4000); // past 3600 window
    let wallet_before = env.token_balance(&s.wallet);

    env.send(
        urthr_net::instruction::SettleClaim {},
        urthr_net::accounts::SettleClaim {
            config: s.config,
            campaign: s.campaign,
            claim,
            escrow_vault: s.escrow_vault,
            treasury: s.treasury,
            publisher: s.publisher,
            publisher_token_account: s.wallet,
            payment_mint: env.mint,
            token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    let fee = 40 * ONE_TOKEN / 10_000 * 50; // 0.5%
    let payout = 40 * ONE_TOKEN - fee;
    assert_eq!(env.token_balance(&s.wallet), wallet_before + payout);
    assert_eq!(env.token_balance(&s.treasury), fee);

    let c: Claim = env.get(&claim);
    assert!(c.status == ClaimStatus::Settled);
    let camp: Campaign = env.get(&s.campaign);
    assert_eq!(camp.locked_budget, 0);
    assert_eq!(env.get::<Publisher>(&s.publisher).locked_amount, 0);
}

#[test]
fn settle_before_window_is_rejected() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    let res = env.send(
        urthr_net::instruction::SettleClaim {},
        urthr_net::accounts::SettleClaim {
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    );
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test claim`
Expected: FAIL — `SettleClaim` missing.

- [ ] **Step 3: Implement `settle_claim.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, Claim, ClaimStatus, ProtocolConfig, Publisher};
use crate::util::fee_amount;

#[derive(Accounts)]
pub struct SettleClaim<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = treasury)]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.advertiser.as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = escrow_vault,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        mut,
        constraint = claim.campaign == campaign.key() @ UrthrError::Unauthorized,
        constraint = claim.publisher == publisher.key() @ UrthrError::Unauthorized,
    )]
    pub claim: Account<'info, Claim>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,

    #[account(mut)]
    pub publisher: Account<'info, Publisher>,

    #[account(mut, token::mint = payment_mint, token::authority = publisher.authority)]
    pub publisher_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SettleClaim>) -> Result<()> {
    let claim = &ctx.accounts.claim;
    require!(claim.status == ClaimStatus::Pending, UrthrError::ClaimNotPending);
    let now = Clock::get()?.unix_timestamp;
    require!(now > claim.challenge_deadline, UrthrError::ChallengeWindowOpen);

    let amount = claim.amount;
    let fee = fee_amount(amount, ctx.accounts.config.protocol_fee_bps)?;
    let payout = amount.checked_sub(fee).ok_or(UrthrError::MathOverflow)?;

    settle_payout(
        &ctx.accounts.token_program,
        &ctx.accounts.escrow_vault,
        &ctx.accounts.treasury,
        &ctx.accounts.publisher_token_account,
        &ctx.accounts.payment_mint,
        &ctx.accounts.campaign,
        payout,
        fee,
    )?;

    // Accounting: consume locked budget, release locked stake.
    let campaign = &mut ctx.accounts.campaign;
    campaign.locked_budget = campaign.locked_budget
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    let publisher = &mut ctx.accounts.publisher;
    publisher.locked_amount = publisher.locked_amount
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    ctx.accounts.claim.status = ClaimStatus::Settled;
    Ok(())
}

/// Shared escrow->publisher/treasury transfer, signed by the Campaign PDA.
/// Reused by resolve_claim (settle branch) in Task 11.
#[allow(clippy::too_many_arguments)]
pub fn settle_payout<'info>(
    token_program: &Program<'info, Token>,
    escrow_vault: &Account<'info, TokenAccount>,
    treasury: &Account<'info, TokenAccount>,
    publisher_token_account: &Account<'info, TokenAccount>,
    payment_mint: &Account<'info, Mint>,
    campaign: &Account<'info, Campaign>,
    payout: u64,
    fee: u64,
) -> Result<()> {
    let advertiser = campaign.advertiser;
    let campaign_id = campaign.campaign_id.to_le_bytes();
    let signer_seeds: &[&[&[u8]]] = &[&[
        CAMPAIGN_SEED,
        advertiser.as_ref(),
        &campaign_id,
        &[campaign.bump],
    ]];
    let decimals = payment_mint.decimals;

    transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            TransferChecked {
                from: escrow_vault.to_account_info(),
                mint: payment_mint.to_account_info(),
                to: publisher_token_account.to_account_info(),
                authority: campaign.to_account_info(),
            },
            signer_seeds,
        ),
        payout,
        decimals,
    )?;

    if fee > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                TransferChecked {
                    from: escrow_vault.to_account_info(),
                    mint: payment_mint.to_account_info(),
                    to: treasury.to_account_info(),
                    authority: campaign.to_account_info(),
                },
                signer_seeds,
            ),
            fee,
            decimals,
        )?;
    }
    Ok(())
}
```

> Note: `settle_payout` is `pub` so Task 11's `resolve_claim` reuses it (DRY). Keep its signature stable.

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod settle_claim;
pub use settle_claim::*;
```

Add to `#[program]`:

```rust
    pub fn settle_claim(ctx: Context<SettleClaim>) -> Result<()> {
        settle_claim::handler(ctx)
    }
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test claim`
Expected: PASS (7 tests).

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/claim.rs
git commit -m "feat: add settle_claim for unchallenged claims past the window"
```

---

## Task 11: `resolve_claim` (attestor: settle or slash)

**Files:**
- Create: `programs/urthr-net/src/instructions/resolve_claim.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Modify: `programs/urthr-net/tests/claim.rs`

Attestor-only. Requires `status == Challenged`. A boolean `fraud` chooses the branch:
- **valid (`fraud = false`)** → reuse `settle_payout` (escrow → publisher/treasury), consume `locked_budget`, release `locked_amount`, `status = Settled`.
- **fraud (`fraud = true`)** → release `locked_budget` back to `budget_remaining` (advertiser keeps budget), transfer `amount` from `stake_vault` → `escrow_vault` (signed by Publisher PDA) as compensation and add it to `budget_remaining`, decrement publisher `staked_amount` and `locked_amount`, `status = Slashed`.

- [ ] **Step 1: Add the failing tests**

Append to `programs/urthr-net/tests/claim.rs`:

```rust
fn challenge_with(env: &mut Env, claim: solana_pubkey::Pubkey) {
    use solana_signer::Signer;
    let ch = solana_keypair::Keypair::new();
    env.svm.airdrop(&ch.pubkey(), 1_000_000_000).unwrap();
    env.send(
        urthr_net::instruction::ChallengeClaim { evidence_hash: [3u8; 32] },
        urthr_net::accounts::ChallengeClaim { challenger: ch.pubkey(), claim },
        &[&ch],
    ).unwrap();
}

#[test]
fn resolve_fraud_slashes_stake_and_compensates_advertiser() {
    let mut env = Env::new();
    let s = setup(&mut env); // attestor == payer
    let claim = submit(&mut env, &s, 0, 40); // amount 40
    challenge_with(&mut env, claim);

    let stake_before = env.token_balance(&s.stake_vault);     // 100
    let escrow_before = env.token_balance(&s.escrow_vault);   // 200

    env.send(
        urthr_net::instruction::ResolveClaim { fraud: true },
        urthr_net::accounts::ResolveClaim {
            attestor: s.authority,
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    // stake_vault -40, escrow +40
    assert_eq!(env.token_balance(&s.stake_vault), stake_before - 40 * ONE_TOKEN);
    assert_eq!(env.token_balance(&s.escrow_vault), escrow_before + 40 * ONE_TOKEN);

    let camp: Campaign = env.get(&s.campaign);
    // 160 unlocked + 40 returned + 40 compensation = 240
    assert_eq!(camp.budget_remaining, 240 * ONE_TOKEN);
    assert_eq!(camp.locked_budget, 0);

    let pubr: Publisher = env.get(&s.publisher);
    assert_eq!(pubr.staked_amount, 60 * ONE_TOKEN);
    assert_eq!(pubr.locked_amount, 0);
    assert!(env.get::<Claim>(&claim).status == ClaimStatus::Slashed);
}

#[test]
fn resolve_valid_settles_like_unchallenged() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    challenge_with(&mut env, claim);
    let wallet_before = env.token_balance(&s.wallet);

    env.send(
        urthr_net::instruction::ResolveClaim { fraud: false },
        urthr_net::accounts::ResolveClaim {
            attestor: s.authority,
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    let fee = 40 * ONE_TOKEN / 10_000 * 50;
    assert_eq!(env.token_balance(&s.wallet), wallet_before + (40 * ONE_TOKEN - fee));
    assert_eq!(env.token_balance(&s.treasury), fee);
    assert!(env.get::<Claim>(&claim).status == ClaimStatus::Settled);
}

#[test]
fn resolve_by_non_attestor_is_rejected() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    challenge_with(&mut env, claim);
    use solana_signer::Signer;
    let imposter = solana_keypair::Keypair::new();
    env.svm.airdrop(&imposter.pubkey(), 1_000_000_000).unwrap();
    let res = env.send(
        urthr_net::instruction::ResolveClaim { fraud: true },
        urthr_net::accounts::ResolveClaim {
            attestor: imposter.pubkey(),
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[&imposter],
    );
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test claim`
Expected: FAIL — `ResolveClaim` missing.

- [ ] **Step 3: Implement `resolve_claim.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::instructions::settle_claim::settle_payout;
use crate::state::{Campaign, Claim, ClaimStatus, ProtocolConfig, Publisher};
use crate::util::fee_amount;

#[derive(Accounts)]
pub struct ResolveClaim<'info> {
    pub attestor: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = treasury,
        constraint = config.attestor == attestor.key() @ UrthrError::Unauthorized,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.advertiser.as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = escrow_vault,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        mut,
        constraint = claim.campaign == campaign.key() @ UrthrError::Unauthorized,
        constraint = claim.publisher == publisher.key() @ UrthrError::Unauthorized,
    )]
    pub claim: Account<'info, Claim>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,

    #[account(mut, has_one = stake_vault)]
    pub publisher: Account<'info, Publisher>,

    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = publisher.authority)]
    pub publisher_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ResolveClaim>, fraud: bool) -> Result<()> {
    require!(ctx.accounts.claim.status == ClaimStatus::Challenged, UrthrError::ClaimNotChallenged);
    let amount = ctx.accounts.claim.amount;

    if fraud {
        // Compensation: stake_vault -> escrow_vault, signed by Publisher PDA.
        let publisher_authority = ctx.accounts.publisher.authority;
        let bump = ctx.accounts.publisher.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            PUBLISHER_SEED,
            publisher_authority.as_ref(),
            &[bump],
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.stake_vault.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                    to: ctx.accounts.escrow_vault.to_account_info(),
                    authority: ctx.accounts.publisher.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            ctx.accounts.payment_mint.decimals,
        )?;

        let campaign = &mut ctx.accounts.campaign;
        // Return locked budget AND add compensation to remaining.
        campaign.locked_budget = campaign.locked_budget
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        campaign.budget_remaining = campaign.budget_remaining
            .checked_add(amount).ok_or(UrthrError::MathOverflow)? // unlock
            .checked_add(amount).ok_or(UrthrError::MathOverflow)?; // compensation

        let publisher = &mut ctx.accounts.publisher;
        publisher.staked_amount = publisher.staked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        publisher.locked_amount = publisher.locked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;

        ctx.accounts.claim.status = ClaimStatus::Slashed;
    } else {
        let fee = fee_amount(amount, ctx.accounts.config.protocol_fee_bps)?;
        let payout = amount.checked_sub(fee).ok_or(UrthrError::MathOverflow)?;
        settle_payout(
            &ctx.accounts.token_program,
            &ctx.accounts.escrow_vault,
            &ctx.accounts.treasury,
            &ctx.accounts.publisher_token_account,
            &ctx.accounts.payment_mint,
            &ctx.accounts.campaign,
            payout,
            fee,
        )?;
        let campaign = &mut ctx.accounts.campaign;
        campaign.locked_budget = campaign.locked_budget
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        let publisher = &mut ctx.accounts.publisher;
        publisher.locked_amount = publisher.locked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        ctx.accounts.claim.status = ClaimStatus::Settled;
    }
    Ok(())
}
```

> Note: the `has_one = treasury` / `has_one = escrow_vault` / `has_one = stake_vault` constraints validate the vault accounts against the stored pubkeys, preventing a caller from substituting their own token accounts.

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod resolve_claim;
pub use resolve_claim::*;
```

Add to `#[program]`:

```rust
    pub fn resolve_claim(ctx: Context<ResolveClaim>, fraud: bool) -> Result<()> {
        resolve_claim::handler(ctx, fraud)
    }
```

- [ ] **Step 5: Build and run**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net --test claim`
Expected: PASS (10 tests).

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/claim.rs
git commit -m "feat: add resolve_claim with attestor settle/slash branches"
```

---

## Task 12: `close_campaign`

**Files:**
- Create: `programs/urthr-net/src/instructions/close_campaign.rs`
- Modify: `programs/urthr-net/src/instructions/mod.rs`
- Modify: `programs/urthr-net/src/lib.rs`
- Modify: `programs/urthr-net/tests/campaign.rs`

Advertiser-only. Requires `locked_budget == 0` (no pending/challenged claims tying up funds). Refunds `budget_remaining` from escrow → advertiser (signed by Campaign PDA), zeroes `budget_remaining`, sets `status = Closed`.

- [ ] **Step 1: Add the failing test**

Append to `programs/urthr-net/tests/campaign.rs`:

```rust
#[test]
fn close_campaign_refunds_remaining() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let advertiser = env.payer.pubkey();
    let src = env.create_token_account(&advertiser, 1_000 * ONE_TOKEN);
    let (campaign, escrow_vault) = create_campaign(&mut env, 1, 1 * ONE_TOKEN);
    fund_campaign(&mut env, campaign, escrow_vault, src, 300 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);

    let src_before = env.token_balance(&src);
    env.send(
        urthr_net::instruction::CloseCampaign {},
        urthr_net::accounts::CloseCampaign {
            advertiser, config, campaign, escrow_vault,
            advertiser_token_account: src,
            payment_mint: env.mint, token_program: spl_token::ID,
        },
        &[],
    ).unwrap();

    assert_eq!(env.token_balance(&src), src_before + 300 * ONE_TOKEN);
    assert_eq!(env.token_balance(&escrow_vault), 0);
    let c: Campaign = env.get(&campaign);
    assert!(c.status == CampaignStatus::Closed);
    assert_eq!(c.budget_remaining, 0);
}

#[test]
fn close_campaign_with_locked_budget_is_rejected() {
    use urthr_net::state::Publisher;
    let mut env = Env::new();
    init_protocol(&mut env);
    let authority = env.payer.pubkey();
    let src = env.create_token_account(&authority, 1_000 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);

    // register + stake so a claim can be submitted
    let (publisher, stake_vault) = {
        let (p, _) = publisher_pda(&env.program_id, &authority);
        let (sv, _) = stake_vault_pda(&env.program_id, &authority);
        env.send(
            urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
            urthr_net::accounts::RegisterPublisher {
                authority, config, publisher: p, stake_vault: sv,
                payment_mint: env.mint, token_program: spl_token::ID,
                system_program: anchor_lang::system_program::ID,
                rent: anchor_lang::solana_program::sysvar::rent::ID,
            }, &[],
        ).unwrap();
        env.send(
            urthr_net::instruction::Stake { amount: 100 * ONE_TOKEN },
            urthr_net::accounts::Stake {
                authority, config, publisher: p, stake_vault: sv,
                authority_token_account: src,
                payment_mint: env.mint, token_program: spl_token::ID,
            }, &[],
        ).unwrap();
        (p, sv)
    };
    let _ = stake_vault;

    let (campaign, escrow_vault) = create_campaign(&mut env, 1, 1 * ONE_TOKEN);
    fund_campaign(&mut env, campaign, escrow_vault, src, 300 * ONE_TOKEN);

    // submit a claim -> locks budget
    let (claim, _) = claim_pda(&env.program_id, &campaign, 0);
    env.send(
        urthr_net::instruction::SubmitClaim { event_count: 10, merkle_root: [0u8; 32] },
        urthr_net::accounts::SubmitClaim {
            publisher_authority: authority, config, publisher, campaign, claim,
            system_program: anchor_lang::system_program::ID,
        }, &[],
    ).unwrap();

    let res = env.send(
        urthr_net::instruction::CloseCampaign {},
        urthr_net::accounts::CloseCampaign {
            advertiser: authority, config, campaign, escrow_vault,
            advertiser_token_account: src,
            payment_mint: env.mint, token_program: spl_token::ID,
        }, &[],
    );
    assert!(res.is_err());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p urthr-net --test campaign`
Expected: FAIL — `CloseCampaign` missing.

- [ ] **Step 3: Implement `close_campaign.rs`**

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
pub struct CloseCampaign<'info> {
    pub advertiser: Signer<'info>,

    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = payment_mint @ UrthrError::InvalidMint)]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = advertiser @ UrthrError::Unauthorized,
        has_one = escrow_vault,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
        constraint = campaign.locked_budget == 0 @ UrthrError::HasPendingClaims,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = advertiser)]
    pub advertiser_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<CloseCampaign>) -> Result<()> {
    let refund = ctx.accounts.campaign.budget_remaining;
    if refund > 0 {
        let advertiser = ctx.accounts.campaign.advertiser;
        let campaign_id = ctx.accounts.campaign.campaign_id.to_le_bytes();
        let bump = ctx.accounts.campaign.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            CAMPAIGN_SEED,
            advertiser.as_ref(),
            &campaign_id,
            &[bump],
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                    to: ctx.accounts.advertiser_token_account.to_account_info(),
                    authority: ctx.accounts.campaign.to_account_info(),
                },
                signer_seeds,
            ),
            refund,
            ctx.accounts.payment_mint.decimals,
        )?;
    }
    let campaign = &mut ctx.accounts.campaign;
    campaign.budget_remaining = 0;
    campaign.status = CampaignStatus::Closed;
    Ok(())
}
```

- [ ] **Step 4: Wire `mod.rs` and `lib.rs`**

Append to `mod.rs`:

```rust
pub mod close_campaign;
pub use close_campaign::*;
```

Add to `#[program]`:

```rust
    pub fn close_campaign(ctx: Context<CloseCampaign>) -> Result<()> {
        close_campaign::handler(ctx)
    }
```

- [ ] **Step 5: Build and run the whole Rust suite**

Run: `NO_DNA=1 anchor build && cargo test -p urthr-net`
Expected: PASS — all unit/integration Rust tests across `util`, `state_space`, `initialize_protocol`, `publisher`, `campaign`, `claim`.

- [ ] **Step 6: Commit**

```bash
git add programs/urthr-net/src programs/urthr-net/tests/campaign.rs
git commit -m "feat: add close_campaign with refund and pending-claim guard"
```

---

## Task 13: Surfpool TypeScript integration test (full lifecycle)

**Files:**
- Create: `tests/lifecycle.test.ts`
- Modify: `package.json` (add a `test:integration` script)
- Create/Modify: `tests/tsconfig.json` if needed for ts-node/vitest

Runs the full happy-path and slash-path against a Surfpool surfnet using the Codama-generated client conventions already used by `app/` (or raw `@solana/kit` instructions). This complements the Rust unit tests with realistic cluster state and the actual SPL Token program.

> Implementer guidance: the repo already generates a TS client for `app/` via `app/scripts/codegen.mjs`. For the integration test, generate (or reuse) a client for the full instruction set and drive it with `@solana/kit`. Use a Surfpool surfnet started with `NO_DNA=1 surfpool start` (see `.claude/skills/solana-dev/references/surfpool/overview.md`) and fund a USDC-like mint via a cheatcode. If wiring full codegen here is heavy, the Rust LiteSVM suite is the authoritative correctness gate; this test is the integration smoke test.

- [ ] **Step 1: Write the lifecycle test**

Create `tests/lifecycle.test.ts`. It must cover both paths end to end:

```ts
import { describe, it, expect, beforeAll } from "vitest";
import { createSolanaRpc, address } from "@solana/kit";

// Connects to a running surfnet on localhost. Start it with `NO_DNA=1 surfpool start`
// (and `anchor deploy`/txtx runbook) before running this test.
const RPC_URL = process.env.RPC_URL ?? "http://127.0.0.1:8899";

describe("UrthrNet lifecycle", () => {
  const rpc = createSolanaRpc(RPC_URL);

  beforeAll(async () => {
    const health = await rpc.getHealth().send();
    expect(health).toBe("ok");
  });

  it("happy path: fund -> stake -> submit -> settle pays publisher minus fee", async () => {
    // 1. initialize_protocol (mint = surfpool USDC cheatcode mint, fee 50 bps, window short)
    // 2. register_publisher + stake
    // 3. create_campaign + fund_campaign
    // 4. submit_claim
    // 5. warp past challenge window (surfpool time cheatcode) and settle_claim
    // 6. assert publisher token balance increased by amount - fee, treasury == fee
    expect(true).toBe(true); // replace with real assertions during implementation
  });

  it("slash path: submit -> challenge -> resolve(fraud) slashes stake to advertiser", async () => {
    // 1..4 as above
    // 5. challenge_claim
    // 6. resolve_claim(fraud = true) as attestor
    // 7. assert stake_vault decreased by amount, escrow increased by amount, claim Slashed
    expect(true).toBe(true); // replace with real assertions during implementation
  });
});
```

> The two `expect(true)` lines are scaffolding placeholders the implementer **must** replace with real RPC-driven assertions (balances before/after, account `status`). Do not leave them in the committed test if the surfnet is available in the environment. If a surfnet cannot run in the environment, mark these `it.skip` with a comment explaining why, and rely on the Rust suite as the correctness gate — log this decision in the commit message.

- [ ] **Step 2: Add the script to `package.json`**

Add under `"scripts"`:

```json
"test:integration": "vitest run tests/lifecycle.test.ts"
```

- [ ] **Step 3: Run the integration test (if surfnet available)**

Run: `NO_DNA=1 surfpool start` (separate shell), then `pnpm test:integration`
Expected: PASS, or explicitly `skip`ped with a logged reason if no surfnet is available.

- [ ] **Step 4: Commit**

```bash
git add tests/lifecycle.test.ts package.json
git commit -m "test: add Surfpool full-lifecycle integration test"
```

---

## Task 14: Documentation — ARCHITECTURE.md and README

**Files:**
- Create: `docs/ARCHITECTURE.md`
- Modify: `README.md`

- [ ] **Step 1: Write `docs/ARCHITECTURE.md`**

Create `docs/ARCHITECTURE.md` with these sections (use the spec as the source of truth; this is the living architecture doc):

```markdown
# UrthrNet Architecture

## Vision
Solana-based decentralized advertising network. Solves ad fraud by requiring publishers
to stake collateral ("skin in the game"): fraudulent traffic results in the publisher's
stake being slashed and used to compensate the advertiser.

## System Map
- **On-chain program `urthr_net`** (this repo, implemented): ProtocolConfig/Treasury,
  Publisher+Stake, Campaign Escrow, Claim challenge/slash/settle state machine.
- **Off-chain services** (sub-project #2, planned): Reporter (aggregates events, submits
  claims), Challenger (bot detection, submits challenges), Attestor (resolves challenges),
  Indexer (reads on-chain state for dashboards).
- **Frontend** (sub-project #3, planned): advertiser and publisher dashboards extending `app/`.

## Trust Model
Off-chain fraud adjudication is abstracted behind a single configurable `attestor` pubkey in
`ProtocolConfig`. The on-chain state machine is independent of how the verdict is reached, so the
attestor role can later be replaced by a voting committee or dispute game without reworking the core.

## On-chain Accounts
(Describe ProtocolConfig, Publisher, Campaign, Claim — mirror the table in the design spec.)

## Claim Lifecycle
(Embed the state-machine diagram from the design spec: submit -> Pending ->
challenge -> Challenged -> resolve(settle|slash); or unchallenged -> settle.)

## Token & Economic Model
Single payment mint (USDC-like) for both budget and collateral. Fee = amount * fee_bps / 10_000.
Settle pays publisher amount - fee (fee -> treasury). Slash returns budget to advertiser and adds
the slashed stake as compensation.

## Repository Layout
(Mirror the file structure table from the implementation plan.)

## Roadmap
1. ✅ Protocol Core (this sub-project)
2. ⬜ Off-chain services (Reporter / Challenger / Attestor / Indexer)
3. ⬜ Frontend dashboards
```

Fill the parenthesised sections with the actual content from `docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md` (do not leave parentheses/placeholders in the committed file).

- [ ] **Step 2: Update `README.md`**

Add an UrthrNet project overview near the top and a "Protocol Core" section documenting:
- One-paragraph project description (decentralized ad network, skin-in-the-game anti-fraud).
- Build: `NO_DNA=1 anchor build`
- Test (Rust unit): `cargo test -p urthr-net`
- Test (integration): `NO_DNA=1 surfpool start` then `pnpm test:integration`
- A link to `docs/ARCHITECTURE.md` and `docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md`.

Keep the existing localnet/wallet instructions for `app/` intact.

- [ ] **Step 3: Verify docs reference real paths**

Run: `ls docs/ARCHITECTURE.md docs/superpowers/specs/2026-05-31-urthr-net-protocol-core-design.md`
Expected: both exist. Re-read both docs and confirm no `TODO`/placeholder/parenthetical instructions remain.

- [ ] **Step 4: Commit**

```bash
git add docs/ARCHITECTURE.md README.md
git commit -m "docs: add ARCHITECTURE and update README for protocol core"
```

---

## Final Verification

- [ ] **Build the program**

Run: `NO_DNA=1 anchor build`
Expected: success, `target/deploy/urthr_net.so` and updated IDL produced.

- [ ] **Run the full Rust test suite**

Run: `cargo test -p urthr-net`
Expected: all green — `fee_*` (3), `account_space_*` (1), `initialize_protocol` (2), `publisher` (4), `campaign` (4), `claim` (10).

- [ ] **Run the integration test (if surfnet available)**

Run: `NO_DNA=1 surfpool start` then `pnpm test:integration`
Expected: PASS or logged `skip`.

- [ ] **Confirm clean tree**

Run: `git status`
Expected: clean working tree, all work committed.
```
