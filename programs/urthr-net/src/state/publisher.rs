use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Publisher {
    pub authority: Pubkey,
    pub stake_vault: Pubkey,
    pub staked_amount: u64,
    pub locked_amount: u64,
    pub metadata: [u8; 32],
    pub bump: u8,
}
