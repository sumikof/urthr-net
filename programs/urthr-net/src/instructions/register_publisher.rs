use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Publisher, ProtocolConfig};

#[derive(Accounts)]
pub struct RegisterPublisher<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = authority,
        space = 8 + Publisher::INIT_SPACE,
        seeds = [PUBLISHER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(
        init,
        payer = authority,
        seeds = [STAKE_VAULT_SEED, authority.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = publisher,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<RegisterPublisher>, metadata: [u8; 32]) -> Result<()> {
    let publisher = &mut ctx.accounts.publisher;
    publisher.authority = ctx.accounts.authority.key();
    publisher.stake_vault = ctx.accounts.stake_vault.key();
    publisher.staked_amount = 0;
    publisher.locked_amount = 0;
    publisher.metadata = metadata;
    publisher.bump = ctx.bumps.publisher;
    Ok(())
}
