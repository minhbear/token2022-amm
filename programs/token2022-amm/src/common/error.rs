use anchor_lang::prelude::*;

#[error_code]
pub enum AMMError {
  #[msg(
    "The token mint is not allowed because it had token extensions not allowed by the program"
  )]
  NotAllowedTokenExtension,

  #[msg("Transfer fee calculation error")]
  TransferFeeCalculationError,
}
