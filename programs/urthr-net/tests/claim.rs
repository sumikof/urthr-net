mod common;
use common::*;
use solana_signer::Signer;
use urthr_net::state::{Campaign, CampaignStatus, Claim, ClaimStatus, Publisher};

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

pub fn challenge(env: &mut Env, claim: solana_pubkey::Pubkey, challenger: &solana_keypair::Keypair) {
    env.svm.airdrop(&challenger.pubkey(), 1_000_000_000).unwrap();
    env.send(
        urthr_net::instruction::ChallengeClaim { evidence_hash: [3u8; 32] },
        urthr_net::accounts::ChallengeClaim {
            challenger: challenger.pk(),
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
    assert_eq!(c.challenger, Some(challenger.pk()));
}

#[test]
fn challenge_after_window_is_rejected() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    env.warp_unix(4000); // window was 3600
    let challenger = solana_keypair::Keypair::new();
    env.svm.airdrop(&challenger.pubkey(), 1_000_000_000).unwrap();
    let res = env.send(
        urthr_net::instruction::ChallengeClaim { evidence_hash: [3u8; 32] },
        urthr_net::accounts::ChallengeClaim { challenger: challenger.pk(), claim },
        &[&challenger],
    );
    assert!(res.is_err());
}

#[test]
fn settle_unchallenged_pays_publisher_and_fee() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40); // amount 40, fee 0.5%

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
            token_program: spl_token_id(),
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
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn settle_challenged_claim_is_rejected() {
    // A challenged claim must go through resolve_claim (attestor), not settle.
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40);
    let challenger = solana_keypair::Keypair::new();
    challenge(&mut env, claim, &challenger);
    env.warp_unix(4000); // even past the window, a Challenged claim can't be settled
    let res = env.send(
        urthr_net::instruction::SettleClaim {},
        urthr_net::accounts::SettleClaim {
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn resolve_fraud_slashes_stake_and_compensates_advertiser() {
    let mut env = Env::new();
    let s = setup(&mut env); // attestor == payer
    let claim = submit(&mut env, &s, 0, 40); // amount 40
    let challenger = solana_keypair::Keypair::new();
    challenge(&mut env, claim, &challenger);

    let stake_before = env.token_balance(&s.stake_vault);   // 100
    let escrow_before = env.token_balance(&s.escrow_vault); // 200

    env.send(
        urthr_net::instruction::ResolveClaim { fraud: true },
        urthr_net::accounts::ResolveClaim {
            attestor: s.authority,
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();

    assert_eq!(env.token_balance(&s.stake_vault), stake_before - 40 * ONE_TOKEN);
    assert_eq!(env.token_balance(&s.escrow_vault), escrow_before + 40 * ONE_TOKEN);

    let camp: Campaign = env.get(&s.campaign);
    assert_eq!(camp.budget_remaining, 240 * ONE_TOKEN); // 160 + 40 unlock + 40 comp
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
    let challenger = solana_keypair::Keypair::new();
    challenge(&mut env, claim, &challenger);
    let wallet_before = env.token_balance(&s.wallet);

    env.send(
        urthr_net::instruction::ResolveClaim { fraud: false },
        urthr_net::accounts::ResolveClaim {
            attestor: s.authority,
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
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
    let challenger = solana_keypair::Keypair::new();
    challenge(&mut env, claim, &challenger);
    let imposter = solana_keypair::Keypair::new();
    env.svm.airdrop(&imposter.pubkey(), 1_000_000_000).unwrap();
    let res = env.send(
        urthr_net::instruction::ResolveClaim { fraud: true },
        urthr_net::accounts::ResolveClaim {
            attestor: imposter.pk(),
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[&imposter],
    );
    assert!(res.is_err());
}

#[test]
fn resolve_unchallenged_claim_is_rejected() {
    // resolve_claim only adjudicates Challenged claims; a Pending one must error.
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40); // still Pending (never challenged)
    let res = env.send(
        urthr_net::instruction::ResolveClaim { fraud: true },
        urthr_net::accounts::ResolveClaim {
            attestor: s.authority,
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, stake_vault: s.stake_vault,
            publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn close_campaign_refunds_remaining() {
    let mut env = Env::new();
    let s = setup(&mut env); // funded 200, no claims, locked_budget == 0
    let wallet_before = env.token_balance(&s.wallet);
    env.send(
        urthr_net::instruction::CloseCampaign {},
        urthr_net::accounts::CloseCampaign {
            advertiser: s.authority,
            config: s.config,
            campaign: s.campaign,
            escrow_vault: s.escrow_vault,
            advertiser_token_account: s.wallet,
            payment_mint: env.mint,
            token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
    assert_eq!(env.token_balance(&s.wallet), wallet_before + 200 * ONE_TOKEN);
    assert_eq!(env.token_balance(&s.escrow_vault), 0);
    let c: Campaign = env.get(&s.campaign);
    assert!(c.status == CampaignStatus::Closed);
    assert_eq!(c.budget_remaining, 0);
}

#[test]
fn close_campaign_with_locked_budget_is_rejected() {
    let mut env = Env::new();
    let s = setup(&mut env);
    let _claim = submit(&mut env, &s, 0, 40); // locks 40 of budget
    let res = env.send(
        urthr_net::instruction::CloseCampaign {},
        urthr_net::accounts::CloseCampaign {
            advertiser: s.authority,
            config: s.config,
            campaign: s.campaign,
            escrow_vault: s.escrow_vault,
            advertiser_token_account: s.wallet,
            payment_mint: env.mint,
            token_program: spl_token_id(),
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn close_campaign_after_partial_settle_refunds_remainder() {
    // End-to-end: fund 200, submit+settle one 40 claim, then close refunds the 160 left.
    let mut env = Env::new();
    let s = setup(&mut env);
    let claim = submit(&mut env, &s, 0, 40); // budget_remaining 200 -> 160, locked 40
    env.warp_unix(4000); // past the 3600 challenge window
    env.send(
        urthr_net::instruction::SettleClaim {},
        urthr_net::accounts::SettleClaim {
            config: s.config, campaign: s.campaign, claim,
            escrow_vault: s.escrow_vault, treasury: s.treasury,
            publisher: s.publisher, publisher_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
    // settle releases the lock but does NOT restore budget_remaining (tokens left escrow).
    assert_eq!(env.get::<Campaign>(&s.campaign).locked_budget, 0);
    assert_eq!(env.get::<Campaign>(&s.campaign).budget_remaining, 160 * ONE_TOKEN);

    let wallet_before = env.token_balance(&s.wallet);
    env.send(
        urthr_net::instruction::CloseCampaign {},
        urthr_net::accounts::CloseCampaign {
            advertiser: s.authority, config: s.config, campaign: s.campaign,
            escrow_vault: s.escrow_vault, advertiser_token_account: s.wallet,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
    assert_eq!(env.token_balance(&s.wallet), wallet_before + 160 * ONE_TOKEN);
    assert_eq!(env.token_balance(&s.escrow_vault), 0);
    let c: Campaign = env.get(&s.campaign);
    assert!(c.status == CampaignStatus::Closed);
    assert_eq!(c.budget_remaining, 0);
}
