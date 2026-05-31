use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum CampaignStatus {
    Active,
    Paused,
    Closed,
}

#[account]
#[derive(InitSpace)]
pub struct Campaign {
    pub advertiser: Pubkey,
    pub escrow_vault: Pubkey,
    pub campaign_id: u64,
    pub price_per_event: u64,
    pub budget_remaining: u64,
    pub locked_budget: u64,
    pub claims_count: u64,
    pub status: CampaignStatus,
    pub bump: u8,
}
