mod common;
use common::*;
use urthr_net::state::ProtocolConfig;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn initializes_protocol_config() {
    let mut env = Env::new();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    let attestor = Keypair::new().pk();

    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor,
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

    let cfg: ProtocolConfig = env.get(&config);
    assert_eq!(cfg.admin, env.payer_pk());
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
            attestor: env.payer_pk(),
            protocol_fee_bps: 10_001,
            min_publisher_stake: 0,
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
    );
    assert!(res.is_err());
}

#[test]
fn rejects_initialize_by_non_upgrade_authority() {
    let mut env = Env::new();
    let attacker = Keypair::new();
    env.svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);

    let res = env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: attacker.pk(),
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: attacker.pk(),
            config,
            treasury,
            payment_mint: env.mint,
            program: env.program_id,
            program_data: common::program_data_address(&env.program_id),
            token_program: spl_token_id(),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[&attacker],
    );
    assert!(res.is_err(), "non-upgrade-authority admin must be rejected");
}
