mod common;
use common::*;
use urthr_net::state::ProtocolConfig;

// Convert a v3 solana_pubkey::Pubkey to the anchor/v4 Pubkey used in account structs.
fn to_apk(p: solana_pubkey::Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes())
}

// Convert spl_token's internal Pubkey to anchor/v4 Pubkey.
fn spl_id_to_apk(p: spl_token::solana_program::pubkey::Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes())
}

#[test]
fn initializes_protocol_config() {
    let mut env = Env::new();
    let (config, _) = config_pda(&env.program_id);
    let (treasury, _) = treasury_pda(&env.program_id);
    let attestor = solana_keypair::Keypair::new().pubkey_();

    env.send(
        urthr_net::instruction::InitializeProtocol {
            attestor: to_apk(attestor),
            protocol_fee_bps: 50,
            min_publisher_stake: 10 * ONE_TOKEN,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: to_apk(env.payer.pubkey_()),
            config: to_apk(config),
            treasury: to_apk(treasury),
            payment_mint: to_apk(env.mint),
            token_program: spl_id_to_apk(spl_token::ID),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
        },
        &[],
    ).unwrap();

    let cfg: ProtocolConfig = env.get(&config);
    assert_eq!(cfg.admin.to_bytes(), env.payer.pubkey_().to_bytes());
    assert_eq!(cfg.attestor.to_bytes(), attestor.to_bytes());
    assert_eq!(cfg.payment_mint.to_bytes(), env.mint.to_bytes());
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
            attestor: to_apk(env.payer.pubkey_()),
            protocol_fee_bps: 10_001,
            min_publisher_stake: 0,
            challenge_window: 3600,
        },
        urthr_net::accounts::InitializeProtocol {
            admin: to_apk(env.payer.pubkey_()),
            config: to_apk(config),
            treasury: to_apk(treasury),
            payment_mint: to_apk(env.mint),
            token_program: spl_id_to_apk(spl_token::ID),
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::prelude::rent::ID,
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
