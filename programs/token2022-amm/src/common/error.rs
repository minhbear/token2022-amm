use anchor_lang::prelude::*;

#[error_code]
pub enum AMMError {
  #[msg(
    "The token mint is not allowed because it had token extensions not allowed by the program"
  )]
  NotAllowedTokenExtension,

  #[msg("Transfer fee calculation error")]
  TransferFeeCalculationError,

  #[msg("Pool is locked")]
  PoolLocked,

  #[msg("User not whitelisted")]
  NotWhitelisted,

  #[msg("Slippage tolerance exceeded")]
  SlippageExceeded,

  #[msg("Invalid amount")]
  InvalidAmount,

  #[msg("Insufficient liquidity")]
  InsufficientLiquidity,

  #[msg("Invalid mint")]
  InvalidMint,

  #[msg("Insufficient output amount")]
  InsufficientOutputAmount,
}
