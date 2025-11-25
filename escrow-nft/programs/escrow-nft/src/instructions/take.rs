use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::state::EscrowState;
use crate::Errors;
use mpl_core::accounts::BaseAssetV1;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>, 
    
    /// CHECK: El Mint del NFT (Asset) se usa para inicializar la ATA de destino.
    #[account(mut, address = escrow.maker)]
    pub maker: SystemAccount<'info>, 

    // 3. El Token de Pago (Mint)
    #[account(mut, address = escrow.currency_mint)]
    pub mint_sol: Box<InterfaceAccount<'info, Mint>>,
    
    // 4. El NFT/Asset
    /// CHECK: El NFT de Metaplex Core se transfiere cambiando su dueño interno.
    #[account(mut, address = escrow.nft_mint)]
    pub asset: UncheckedAccount<'info>,
    
    // 5. La Cuenta de Estado del Escrow (PDA que contiene la data y es la autoridad del NFT)
    #[account(
        mut,
        close = maker, 
        seeds = [b"escrow",seed.to_le_bytes().as_ref()],
        bump = escrow.escrow_bump,
        constraint = escrow.maker == maker.key(),
    )]
    pub escrow: Box<Account<'info, EscrowState>>,
    
    // 6. Cuenta Vault del Programa (ATA que posee el token de PAGO)
    // No se usa en TAKE, pero se mantiene por coherencia si se usara una fee.
    #[account(
       mut,
       associated_token::mint = mint_sol,
       associated_token::authority = escrow,
       associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    // 7. Cuenta del Vendedor 
    #[account(
        mut,
        associated_token::mint = mint_sol,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_sol: Box<InterfaceAccount<'info, TokenAccount>>,

    // 8. Cuenta del Comprador (ATA que paga el token de PAGO)
    #[account(
        mut,
        associated_token::mint = mint_sol,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_sol: Box<InterfaceAccount<'info, TokenAccount>>,

    // 10. Cuentas de Programas
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    /// CHECK: El ID del programa Metaplex Core se verifica fuera de este struct y se requiere para la CPI.
    pub mpl_core_program: AccountInfo<'info>,
}

impl<'info> Take<'info> {
    pub fn take_nft(&mut self, seed: u64, _bumps: &TakeBumps, price_amount: u64) -> Result<()> {
        
        // 1. Validar el Asset 
        let _base_asset = BaseAssetV1::try_from(&self.asset.to_account_info())
            .map_err(|_| error!(Errors::InvalidAsset))?;
        
        // 2. Transferir el Token de Pago del Taker al Maker
        // 2.1. Cuentas para el CPI de TransferChecked
        let transfer_accounts = TransferChecked {
            from: self.taker_ata_sol.to_account_info(),  
            to: self.maker_ata_sol.to_account_info(),   
            mint: self.mint_sol.to_account_info(),       
            authority: self.taker.to_account_info(),     
        };
        

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);
        
        // 2.3. Ejecutar la transferencia
        transfer_checked(cpi_ctx, price_amount, self.mint_sol.decimals)?;

        // 3. Transferir el NFT del Escrow al Taker (Comprador)
        
        // 3.1. Seeds para firmar con el PDA del Escrow
        let binding = seed.to_le_bytes();
        let seeds: &[&[u8]] = &[
            b"escrow",
            &binding.as_ref(),
            &[self.escrow.escrow_bump], 
        ];
        
        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .authority(Some(&self.escrow.to_account_info())) 
            .payer(&self.taker.to_account_info()) 
            .new_owner(&self.taker.to_account_info()) 
            .invoke_signed(&[seeds])?; 

        msg!("NFT Bought successfully. Escrow closed and fees returned to Maker.");
        Ok(())
    }
}