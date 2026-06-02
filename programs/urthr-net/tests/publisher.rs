mod common;
use common::*;
use urthr_net::state::Publisher;

fn init_protocol(env: &mut Env) {
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
            admin: env.payer_pk(),
            config,
            treasury,
            payment_mint: env.mint,
            program: env.program_id,
            program_data: common::program_data_address(&env.program_id),
            token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();
}

#[test]
fn registers_a_publisher() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let (config, _) = config_pda(&env.program_id);
    let authority = env.payer_pk();
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
            token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
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

fn register(env: &mut Env) -> (solana_pubkey::Pubkey, solana_pubkey::Pubkey) {
    let authority = env.payer_pk();
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);
    let (config, _) = config_pda(&env.program_id);
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
    (publisher, stake_vault)
}

#[test]
fn stake_and_unstake_round_trip() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let authority = env.payer_pk();
    let (publisher, stake_vault) = register(&mut env);
    let user_ata = env.create_token_account(&authority, 100 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);

    env.send(
        urthr_net::instruction::Stake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
    assert_eq!(env.token_balance(&stake_vault), 30 * ONE_TOKEN);
    assert_eq!(env.get::<urthr_net::state::Publisher>(&publisher).staked_amount, 30 * ONE_TOKEN);

    env.send(
        urthr_net::instruction::Unstake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Unstake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token_id(),
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
    let authority = env.payer_pk();
    let (publisher, stake_vault) = register(&mut env);
    let user_ata = env.create_token_account(&authority, 100 * ONE_TOKEN);
    let (config, _) = config_pda(&env.program_id);
    env.send(
        urthr_net::instruction::Stake { amount: 30 * ONE_TOKEN },
        urthr_net::accounts::Stake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    ).unwrap();
    // leaving 5 (< 10 min, nonzero) must fail
    let res = env.send(
        urthr_net::instruction::Unstake { amount: 25 * ONE_TOKEN },
        urthr_net::accounts::Unstake {
            authority, config, publisher, stake_vault,
            authority_token_account: user_ata,
            payment_mint: env.mint, token_program: spl_token_id(),
        },
        &[],
    );
    assert!(res.is_err());
}
