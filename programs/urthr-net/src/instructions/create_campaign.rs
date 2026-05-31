use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub advertiser: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = advertiser,
        space = 8 + Campaign::INIT_SPACE,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign_id.to_le_bytes()],
        bump,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        init,
        payer = advertiser,
        seeds = [ESCROW_VAULT_SEED, campaign.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = campaign,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateCampaign>, campaign_id: u64, price_per_event: u64) -> Result<()> {
    require!(price_per_event > 0, UrthrError::InvalidPrice);

    let campaign = &mut ctx.accounts.campaign;
    campaign.advertiser = ctx.accounts.advertiser.key();
    campaign.escrow_vault = ctx.accounts.escrow_vault.key();
    campaign.campaign_id = campaign_id;
    campaign.price_per_event = price_per_event;
    campaign.budget_remaining = 0;
    campaign.locked_budget = 0;
    campaign.claims_count = 0;
    campaign.status = CampaignStatus::Active;
    campaign.bump = ctx.bumps.campaign;
    Ok(())
}
