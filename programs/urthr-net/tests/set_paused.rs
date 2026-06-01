mod common;
use common::*;
use urthr_net::state::ProtocolConfig;

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
    )
    .unwrap();
}

#[test]
fn admin_can_pause_and_unpause() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let (config, _) = config_pda(&env.program_id);

    env.send(
        urthr_net::instruction::SetPaused { paused: true },
        urthr_net::accounts::SetPaused { admin: env.payer_pk(), config },
        &[],
    )
    .unwrap();
    assert!(env.get::<ProtocolConfig>(&config).paused);

    env.send(
        urthr_net::instruction::SetPaused { paused: false },
        urthr_net::accounts::SetPaused { admin: env.payer_pk(), config },
        &[],
    )
    .unwrap();
    assert!(!env.get::<ProtocolConfig>(&config).paused);
}

#[test]
fn paused_blocks_gated_instruction() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let (config, _) = config_pda(&env.program_id);
    let authority = env.payer_pk();
    let (publisher, _) = publisher_pda(&env.program_id, &authority);
    let (stake_vault, _) = stake_vault_pda(&env.program_id, &authority);

    // Sanity: while unpaused, register_publisher would succeed, so any failure
    // after pausing must be attributable to the `!config.paused` gate.
    {
        let mut probe = Env::new();
        init_protocol(&mut probe);
        let (c, _) = config_pda(&probe.program_id);
        let auth = probe.payer_pk();
        let (pub_pda, _) = publisher_pda(&probe.program_id, &auth);
        let (sv, _) = stake_vault_pda(&probe.program_id, &auth);
        probe
            .send(
                urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
                urthr_net::accounts::RegisterPublisher {
                    authority: auth,
                    config: c,
                    publisher: pub_pda,
                    stake_vault: sv,
                    payment_mint: probe.mint,
                    token_program: spl_token_id(),
                    system_program: anchor_lang::system_program::ID,
                    rent: anchor_lang::prelude::rent::ID,
                },
                &[],
            )
            .expect("register_publisher succeeds when not paused");
    }

    // Now pause and confirm the same instruction is rejected by the gate.
    env.send(
        urthr_net::instruction::SetPaused { paused: true },
        urthr_net::accounts::SetPaused { admin: authority, config },
        &[],
    )
    .unwrap();

    let res = env.send(
        urthr_net::instruction::RegisterPublisher { metadata: [0u8; 32] },
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
    );
    assert!(res.is_err(), "register_publisher must fail while paused (ProtocolPaused gate)");
}

#[test]
fn non_admin_cannot_set_paused() {
    let mut env = Env::new();
    init_protocol(&mut env);
    let (config, _) = config_pda(&env.program_id);

    let attacker = solana_keypair::Keypair::new();
    env.svm.airdrop(&solana_signer::Signer::pubkey(&attacker), 1_000_000_000).unwrap();

    let res = env.send(
        urthr_net::instruction::SetPaused { paused: true },
        urthr_net::accounts::SetPaused { admin: attacker.pk(), config },
        &[&attacker],
    );
    assert!(res.is_err(), "non-admin set_paused must be rejected by has_one = admin");
}
