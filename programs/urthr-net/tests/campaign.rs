mod common;
use common::*;
use urthr_net::state::{Campaign, CampaignStatus};

pub fn init_protocol(env: &mut Env) {
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: env.payer_pk(),
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: env.payer_pk(), config, treasury,
            payment_mint: env.mint, token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();
}

pub fn create_campaign(env: &mut Env, id: u64, price: u64) -> (solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let advertiser = env.payer_pk();
    let (config, _) = config_pda(&env.program_id);
    let (campaign, _) = campaign_pda(&env.program_id, &advertiser, id);
    let (escrow_vault, _) = escrow_vault_pda(&env.program_id, &campaign);
    env.send(
        urthr_net::instruction::CreateCampaign { campaign_id: id, price_per_event: price },
        urthr_net::accounts::CreateCampaign {
            advertiser, config, campaign, escrow_vault,
            payment_mint: env.mint, token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();
    (campaign, escrow_vault)
}

pub fn fund_campaign(env: &mut Env, campaign: solana_pubkey::Pubkey, escrow_vault: solana_pubkey::Pubkey, src: solana_pubkey::Pubkey, amount: u64) {
    let advertiser = env.payer_pk();
    let (config, _) = config_pda(&env.program_id);
    env.send(
        urthr_net::instruction::FundCampaign { amount },
        urthr_net::accounts::FundCampaign {
            advertiser, config, campaign, escrow_vault,
            advertiser_token_account: src,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
}

#[test]
fn create_and_fund_campaign() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let advertiser = env.payer_pk();
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
