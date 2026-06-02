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

    /// Only the program's upgrade authority may initialize the protocol (anti-front-run).
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ UrthrError::Unauthorized)]
    pub program: Program<'info, crate::program::UrthrNet>,
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()) @ UrthrError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeProtocol>,
    attestor: Pubkey,
    protocol_fee_bps: u16,
    min_publisher_stake: u64,
    challenge_window: u64,
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
