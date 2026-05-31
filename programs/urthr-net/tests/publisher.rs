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
