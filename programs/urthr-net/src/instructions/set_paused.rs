use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::ProtocolConfig;

#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ UrthrError::Unauthorized,
    )]
    pub config: Account<'info, ProtocolConfig>,
}

pub fn handler(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.config.paused = paused;
    Ok(())
}
