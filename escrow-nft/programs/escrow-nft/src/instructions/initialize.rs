use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::state::EscrowState; 

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut)]
    pub mint_sol: Box<InterfaceAccount<'info, Mint>>, 
    #[account(mut)]

    pub asset: UncheckedAccount<'info>,
    
    // 1. Inicialización de la cuenta de estado (PDA)
    #[account(
        init_if_needed,
        space = 8 + EscrowState::INIT_SPACE, 
        payer = maker,
        seeds = [b"escrow",seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Box<Account<'info, EscrowState>>, 
    // 2. Inicialización de la cuenta Vault (ATA que poseerá el token de PAGO)
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_sol,
        associated_token::authority = escrow, // El PDA del escrow es el propietario
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    // 3. Inicialización de la cuenta del Vendedor (ATA que recibirá el token de PAGO)
    #[account(
      init_if_needed,
      payer = maker,
      associated_token::mint = mint_sol,
      associated_token::authority = maker,
      associated_token::token_program = token_program,
    )]
    pub maker_ata_sol: Box<InterfaceAccount<'info, TokenAccount>>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    ///CHECK: SAFE (Metaplex Core Program ID)
    pub mpl_core_program: AccountInfo<'info>,
}

impl<'info> Initialize<'info> {
    pub fn initialize_escrow(
        &mut self,
        seed: u64,
        bumps: &InitializeBumps,
        amount: u64,
    ) -> Result<()> {
        // Guardamos los datos en la nueva EscrowState
        self.escrow.set_inner(EscrowState {
            seed: seed,
            maker: self.maker.key(),
            nft_mint: self.asset.key(), // El asset key es el mint del NFT
            currency_mint: self.mint_sol.key(),
            price: amount,
            vault_ata: self.vault.key(),
            escrow_bump: bumps.escrow,
            fee_percent: 5, // Tarifa fija del 5%
        });
        Ok(())
    }
}