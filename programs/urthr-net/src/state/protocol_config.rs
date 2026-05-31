use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub attestor: Pubkey,
    pub payment_mint: Pubkey,
    pub treasury: Pubkey,
    pub protocol_fee_bps: u16,
    pub min_publisher_stake: u64,
    pub challenge_window: i64,
    pub paused: bool,
    pub bump: u8,
}
