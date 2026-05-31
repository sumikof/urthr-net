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
    spl_token::{
        solana_program::program_pack::Pack,
        state::{Account as SplAccount, AccountState, Mint},
    },
};

/// spl-token 8 packs/unpacks with its own `Pubkey` type (a distinct
/// `solana-pubkey` version from anchor's), so bridge by raw bytes.
type SplPubkey = spl_token::solana_program::pubkey::Pubkey;

fn to_spl(p: Pubkey) -> SplPubkey {
    SplPubkey::new_from_array(p.to_bytes())
}

/// The litesvm/solana-account `owner` field for SPL-owned accounts.
fn spl_owner() -> Pubkey {
    Pubkey::new_from_array(spl_token::ID.to_bytes())
}

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
        let bytes = include_bytes!("../../../../target/deploy/urthr_net.so");
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
            mint_authority: coption_some(self.payer.pubkey()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: coption_none(),
        };
        Mint::pack(state, &mut data).unwrap();
        self.svm.set_account(mint, Account {
            lamports: self.svm.minimum_balance_for_rent_exemption(Mint::LEN),
            data,
            owner: spl_owner(),
            executable: false,
            rent_epoch: 0,
        }).unwrap();
        mint
    }

    /// Pack an SPL token account holding `amount` of `self.mint`, owned by `owner`.
    pub fn create_token_account(&mut self, owner: &Pubkey, amount: u64) -> Pubkey {
        let addr = Pubkey::new_unique();
        self.set_token_account(addr, owner, amount);
        addr
    }

    /// Set token-account state at an explicit address (use for PDA vaults if pre-seeding).
    pub fn set_token_account(&mut self, address: Pubkey, owner: &Pubkey, amount: u64) {
        let mut data = vec![0u8; SplAccount::LEN];
        let state = SplAccount {
            mint: to_spl(self.mint),
            owner: to_spl(*owner),
            amount,
            delegate: coption_none(),
            state: AccountState::Initialized,
            is_native: coption_none_u64(),
            delegated_amount: 0,
            close_authority: coption_none(),
        };
        SplAccount::pack(state, &mut data).unwrap();
        self.svm.set_account(address, Account {
            lamports: self.svm.minimum_balance_for_rent_exemption(SplAccount::LEN),
            data,
            owner: spl_owner(),
            executable: false,
            rent_epoch: 0,
        }).unwrap();
    }

    pub fn token_balance(&self, address: &Pubkey) -> u64 {
        let acct = self
            .svm
            .get_account(address)
            .unwrap_or_else(|| panic!("token account not found: {address}"));
        SplAccount::unpack(&acct.data).unwrap().amount
    }

    pub fn get<T: anchor_lang::AccountDeserialize>(&self, address: &Pubkey) -> T {
        let acct = self
            .svm
            .get_account(address)
            .unwrap_or_else(|| panic!("account not found: {address}"));
        T::try_deserialize(&mut acct.data.as_slice()).unwrap()
    }

    /// Send a single instruction signed by `extra_signers` (payer auto-included as fee payer).
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

    /// Advance the simulated clock by `seconds` (relative delta, not an absolute timestamp).
    pub fn warp_unix(&mut self, seconds: i64) {
        let mut clock = self.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
        clock.unix_timestamp += seconds;
        self.svm.set_sysvar(&clock);
    }
}

// COption helpers (spl-token re-exports solana_program::program_option::COption).
use spl_token::solana_program::program_option::COption;

fn coption_some(p: Pubkey) -> COption<SplPubkey> {
    COption::Some(to_spl(p))
}
fn coption_none() -> COption<SplPubkey> {
    COption::None
}
fn coption_none_u64() -> COption<u64> {
    COption::None
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
