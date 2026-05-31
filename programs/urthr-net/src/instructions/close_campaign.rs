use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, ProtocolConfig};

#[derive(Accounts)]
pub struct CloseCampaign<'info> {
    pub advertiser: Signer<'info>,

    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = payment_mint @ UrthrError::InvalidMint)]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, advertiser.key().as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = advertiser @ UrthrError::Unauthorized,
        has_one = escrow_vault,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
        constraint = campaign.locked_budget == 0 @ UrthrError::HasPendingClaims,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = advertiser)]
    pub advertiser_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

// Intentionally NOT gated on `config.paused`: closing returns the advertiser's
// own unspent budget (an exit/withdrawal), which should remain available even
// during an emergency pause — same rationale as `unstake`.
pub fn handler(ctx: Context<CloseCampaign>) -> Result<()> {
    let refund = ctx.accounts.campaign.budget_remaining;
    if refund > 0 {
        let advertiser = ctx.accounts.campaign.advertiser;
        let campaign_id = ctx.accounts.campaign.campaign_id.to_le_bytes();
        let bump = ctx.accounts.campaign.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            CAMPAIGN_SEED,
            advertiser.as_ref(),
            &campaign_id,
            &[bump],
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                    to: ctx.accounts.advertiser_token_account.to_account_info(),
                    authority: ctx.accounts.campaign.to_account_info(),
                },
                signer_seeds,
            ),
            refund,
            ctx.accounts.payment_mint.decimals,
        )?;
    }
    let campaign = &mut ctx.accounts.campaign;
    campaign.budget_remaining = 0;
    campaign.status = CampaignStatus::Closed;
    Ok(())
}
