#![allow(deprecated)]
use anchor_lang::prelude::*;
declare_id!("8Ht8LLi2j5igFeWfmLkfXvRMkUxspwA2sEHiPctPsrXL");
pub mod instructions; 
pub mod state;      


pub use instructions::*; 

pub mod errors;
pub use errors::*;

#[program]
pub mod escrow_nft {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, seed: u64, amount: u64) -> Result<()> {
        ctx.accounts.initialize_escrow(seed, &ctx.bumps, amount)?;
        msg!("Program initialized: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn make(ctx: Context<Make>, amount: u64, seed: u64) -> Result<()> {
        ctx.accounts.make_listing(amount, seed,)?;
        msg!("NFT listing created");
        Ok(())
    }

    pub fn take(ctx: Context<Take>, seed: u64) -> Result<()> {
        msg!("NFT listing taken");
        ctx.accounts.take_nft(seed, &ctx.bumps, ctx.accounts.escrow.price)?;
        Ok(())
    }
    pub fn refund(ctx: Context<Refund>, seed: u64) -> Result<()> {
        ctx.accounts.refund_nft(seed, &ctx.bumps)?;
        msg!("NFT listing refunded");
        Ok(())
    }


}

