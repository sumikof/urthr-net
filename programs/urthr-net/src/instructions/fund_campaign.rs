use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    pub advertiser: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = advertiser @ UrthrError::Unauthorized,
        has_one = escrow_vault,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = advertiser)]
    pub advertiser_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.advertiser_token_account.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.advertiser.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.payment_mint.decimals,
    )?;

    let campaign = &mut ctx.accounts.campaign;
    campaign.budget_remaining = campaign.budget_remaining
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;
    Ok(())
}
