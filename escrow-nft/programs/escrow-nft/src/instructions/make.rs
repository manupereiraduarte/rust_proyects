use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::state::EscrowState;
use crate::Errors;
use mpl_core::accounts::BaseAssetV1;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
#[instruction(amount: u64, seed: u64)]
pub struct Make<'info> {
    // 1. El Maker (firmante que listará el NFT)
    #[account(mut)]
    pub maker: Signer<'info>, 

    // 2. El NFT/Asset (Metaplex Core Asset)
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    
    // 3. La cuenta de estado del Escrow (PDA)
    #[account(
        mut,
        seeds = [b"escrow",seed.to_le_bytes().as_ref()],
        bump = escrow.escrow_bump,
        // Comprobaciones de seguridad: 
        // 1. Que el NFT en el estado coincida con el asset.
        constraint = escrow.nft_mint == asset.key() @ Errors::InvalidAsset,
        // 2. Que el maker de la cuenta sea el firmante actual.
        constraint = escrow.maker == maker.key() @ Errors::InvalidAsset,
    )]
    pub escrow: Box<Account<'info, EscrowState>>,

    // 4. Cuenta Vault para el token SPL (Propiedad del PDA, ya inicializada en `initialize`)
    // No se usa para el depósito del NFT, pero la mantenemos como referencia.
    #[account(
        mut,
        address = escrow.vault_ata @ Errors::InvalidAsset, 
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // 5. Cuentas de programas
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    ///CHECK: SAFE (Metaplex Core Program ID)
    pub mpl_core_program: AccountInfo<'info>,
}

impl<'info> Make<'info> {
    pub fn make_listing(&mut self, _amount: u64, seed: u64) -> Result<()> {
        // 1. Validación del NFT
        let _base_asset = BaseAssetV1::try_from(&self.asset.to_account_info())
            .map_err(|_| error!(Errors::InvalidAsset))?;
        
        let escrow_state = &self.escrow.to_account_info();
        let mpl_program = &self.mpl_core_program.to_account_info();
        let maker = &self.maker.to_account_info();

        // 2. Derivación de Seeds para la firma del PDA
        let binding = seed.to_le_bytes();
        let seeds: &[&[u8]] = &[b"escrow", &binding.as_ref(), &[self.escrow.escrow_bump]];

        // 3. Transferencia de Propiedad del NFT al PDA del Escrow
        TransferV1CpiBuilder::new(&mpl_program)
            .asset(&self.asset.to_account_info()) 
            .payer(&maker) 
            .new_owner(&escrow_state) 
            .invoke_signed(&[seeds])?; 

        msg!("NFT transferido y listado con éxito. Nuevo dueño: {}", self.escrow.key());
        Ok(())
    }
}