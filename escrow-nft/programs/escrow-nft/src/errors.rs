use anchor_lang::prelude::*;

#[error_code]
pub enum Errors {
    #[msg("Invalid Asset")]
    InvalidAsset,
    #[msg("Escrow is not listed")]
    NotListed, // Un error útil para agregar después
}