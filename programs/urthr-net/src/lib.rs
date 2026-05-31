pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod util;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR");

#[program]
pub mod urthr_net {
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        attestor: Pubkey,
        protocol_fee_bps: u16,
        min_publisher_stake: u64,
        challenge_window: u64,
    ) -> Result<()> {
        initialize_protocol::handler(ctx, attestor, protocol_fee_bps, min_publisher_stake, challenge_window)
    }

    pub fn register_publisher(ctx: Context<RegisterPublisher>, metadata: [u8; 32]) -> Result<()> {
        register_publisher::handler(ctx, metadata)
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        stake::handler(ctx, amount)
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        unstake::handler(ctx, amount)
    }

    pub fn create_campaign(ctx: Context<CreateCampaign>, campaign_id: u64, price_per_event: u64) -> Result<()> {
        create_campaign::handler(ctx, campaign_id, price_per_event)
    }

    pub fn fund_campaign(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
        fund_campaign::handler(ctx, amount)
    }

    pub fn submit_claim(ctx: Context<SubmitClaim>, event_count: u64, merkle_root: [u8; 32]) -> Result<()> {
        submit_claim::handler(ctx, event_count, merkle_root)
    }
}
