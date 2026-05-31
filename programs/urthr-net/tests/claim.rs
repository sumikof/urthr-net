mod common;
use common::*;
use urthr_net::state::{Campaign, Claim, ClaimStatus, Publisher};

// Sets up protocol + a publisher (staked 100) + a funded campaign (200 budget, price 1/event).
// The payer acts as publisher authority AND advertiser AND attestor (fine for unit tests).
pub fn setup(env: &mut Env) -> Setup {
    let admin = env.payer_pk();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: admin, protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN, challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin, config, treasury, payment_mint: env.mint,
            token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();

    let authority = admin;
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);
    env.send(
        urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
        urthr_net::accounts::RegisterPublisher {
            authority, config, publisher, stake_vault,
            payment_mint: env.mint, token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();

    let wallet = env.create_token_account(&authority, 1_000 * ONE_TOKEN);
    env.send(
        urthr_net::instruction::Stake { amount: 100 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();

    let (campaign, _) = campaign_pda(&env.program_id, &authority, 1);
    let (escrow_vault, _) = escrow_vault_pda(&env.program_id, &campaign);
    env.send(
        urthr_net::instruction::CreateCampaign { campaign_id: 1, price_per_event: 1 * ONE_TOKEN },
        urthr_net::accounts::CreateCampaign {
            advertiser: authority, config, campaign, escrow_vault,
            payment_mint: env.mint, token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();
    env.send(
        urthr_net::instruction::FundCampaign { amount: 200 * ONE_TOKEN },
        urthr_net::accounts::FundCampaign {
            advertiser: authority, config, campaign, escrow_vault,
            advertiser_token_account: wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();

    Setup { authority, config, publisher, stake_vault, campaign, escrow_vault, treasury, wallet }
}

pub struct Setup {
    pub authority: solana_pubkey::Pubkey,
    pub config: solana_pubkey::Pubkey,
    pub publisher: solana_pubkey::Pubkey,
    pub stake_vault: solana_pubkey::Pubkey,
    pub campaign: solana_pubkey::Pubkey,
    pub escrow_vault: solana_pubkey::Pubkey,
    pub treasury: solana_pubkey::Pubkey,
    pub wallet: solana_pubkey::Pubkey,
}

pub fn submit(env: &mut Env, s: &Setup, nonce: u64, events: u64) -> solana_pubkey::Pubkey {
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
    assert_eq!(c.claim_nonce, 0);
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
