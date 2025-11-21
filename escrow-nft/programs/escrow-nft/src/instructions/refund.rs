use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::TokenInterface};

use crate::state::EscrowState;
use crate::Errors;
use mpl_core::accounts::BaseAssetV1;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Refund<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut, address = escrow.nft_mint)] 
    pub asset: UncheckedAccount<'info>,
    
    #[account(
        mut,
        close = maker, 
        seeds = [b"escrow",seed.to_le_bytes().as_ref()],
        bump = escrow.escrow_bump,
        constraint = escrow.maker == maker.key() @ Errors::InvalidAsset,
    )]
    pub escrow: Box<Account<'info, EscrowState>>,

    // 4. Cuentas de Programas
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub mpl_core_program: AccountInfo<'info>,

    // 5. Cuenta ATA del Maker que recibirá el NFT
   
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = asset.key(),
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
}

impl<'info> Refund<'info> {
    pub fn refund_nft(&mut self, seed: u64, _bumps: &RefundBumps) -> Result<()> {
        
        let _base_asset = BaseAssetV1::try_from(&self.asset.to_account_info())
            .map_err(|_| error!(Errors::InvalidAsset))?;
        
        let escrow_state = &self.escrow.to_account_info();
        let mpl_program = &self.mpl_core_program.to_account_info();
        let maker = &self.maker.to_account_info();
        
        let binding = seed.to_le_bytes();
        let seeds: &[&[u8]] = &[
            b"escrow",
            &binding.as_ref(),
            &[self.escrow.escrow_bump], 
        ];

        // 3. Transferir el NFT del Escrow de vuelta al Maker
        TransferV1CpiBuilder::new(&mpl_program)
            .asset(&self.asset.to_account_info())
            .authority(Some(escrow_state)) 
            .payer(maker) 
            .new_owner(maker) 
            
            .invoke_signed(&[seeds])?;
        
        msg!("NFT Unlisted/Refunded successfully. Escrow closed.");

        Ok(())
    }
}