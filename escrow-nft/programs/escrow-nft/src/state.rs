use anchor_lang::prelude::*;
#[account]
#[derive(InitSpace)]
pub struct EscrowState {
    pub seed: u64,
    pub maker: Pubkey,
    pub nft_mint: Pubkey,
    pub currency_mint: Pubkey,
    pub price: u64,
    pub vault_ata: Pubkey,
    pub escrow_bump: u8,
    pub fee_percent: u8,
}