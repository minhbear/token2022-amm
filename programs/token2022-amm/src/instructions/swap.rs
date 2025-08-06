use {
  crate::{
    common::error::AMMError,
    state::{Config, PoolState},
  },
  anchor_lang::prelude::*,
  anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
      transfer_checked, Mint as MintInterface, TokenAccount, TokenInterface, TransferChecked,
    },
  },
};

#[derive(Accounts)]
pub struct Swap<'info> {
  #[account(mut)]
  pub user: Signer<'info>,

  #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
        constraint = !config.locked @ AMMError::PoolLocked
    )]
  pub config: Account<'info, Config>,

  #[account(
        mut,
        seeds = [b"pool", config.key().as_ref()],
        bump
    )]
  pub pool_state: Account<'info, PoolState>,

  /// CHECK: PDA authority for the pool
  #[account(
        seeds = [b"auth", config.key().as_ref()],
        bump = config.auth_bump
    )]
  pub pool_authority: UncheckedAccount<'info>,

  pub mint_in: InterfaceAccount<'info, MintInterface>,
  pub mint_out: InterfaceAccount<'info, MintInterface>,

  /// CHECK: This will be validated as either vault_x or vault_y
  #[account(mut)]
  pub vault_in: InterfaceAccount<'info, TokenAccount>,

  /// CHECK: This will be validated as either vault_x or vault_y  
  #[account(mut)]
  pub vault_out: InterfaceAccount<'info, TokenAccount>,

  /// CHECK: This will be validated as user's token account for mint_in
  #[account(mut)]
  pub user_token_in: InterfaceAccount<'info, TokenAccount>,

  /// CHECK: This will be validated as user's token account for mint_out
  #[account(mut)]
  pub user_token_out: InterfaceAccount<'info, TokenAccount>,

  pub token_program_x: Interface<'info, TokenInterface>,
  pub token_program_y: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
  let pool_state = &mut ctx.accounts.pool_state;
  let config = &ctx.accounts.config;

  require!(amount_in > 0, AMMError::InvalidAmount);

  // Determine which direction we're swapping
  let (reserve_in, reserve_out, is_x_to_y) = if ctx.accounts.mint_in.key() == config.mint_x {
    require!(
      ctx.accounts.mint_out.key() == config.mint_y,
      AMMError::InvalidMint
    );
    (pool_state.reserve_x, pool_state.reserve_y, true)
  } else if ctx.accounts.mint_in.key() == config.mint_y {
    require!(
      ctx.accounts.mint_out.key() == config.mint_x,
      AMMError::InvalidMint
    );
    (pool_state.reserve_y, pool_state.reserve_x, false)
  } else {
    return Err(AMMError::InvalidMint.into());
  };

  require!(
    reserve_in > 0 && reserve_out > 0,
    AMMError::InsufficientLiquidity
  );

  // Calculate output amount using constant product formula with fee
  // amount_out = (amount_in * (10000 - fee) * reserve_out) / ((reserve_in * 10000) + (amount_in * (10000 - fee)))
  let fee_adjusted_amount_in = (amount_in as u128)
    .checked_mul((10000u128).checked_sub(config.fee as u128).unwrap())
    .unwrap();

  let numerator = fee_adjusted_amount_in
    .checked_mul(reserve_out as u128)
    .unwrap();

  let denominator = (reserve_in as u128)
    .checked_mul(10000u128)
    .unwrap()
    .checked_add(fee_adjusted_amount_in)
    .unwrap();

  let amount_out = numerator.checked_div(denominator).unwrap() as u64;

  require!(amount_out >= min_amount_out, AMMError::SlippageExceeded);
  require!(amount_out > 0, AMMError::InsufficientOutputAmount);

  // Determine which token programs to use based on swap direction
  let (token_program_in, token_program_out) = if is_x_to_y {
    (&ctx.accounts.token_program_x, &ctx.accounts.token_program_y)
  } else {
    (&ctx.accounts.token_program_y, &ctx.accounts.token_program_x)
  };

  // Transfer input tokens from user to vault
  let transfer_in_ctx = CpiContext::new(
    token_program_in.to_account_info(),
    TransferChecked {
      from: ctx.accounts.user_token_in.to_account_info(),
      mint: ctx.accounts.mint_in.to_account_info(),
      to: ctx.accounts.vault_in.to_account_info(),
      authority: ctx.accounts.user.to_account_info(),
    },
  );
  transfer_checked(transfer_in_ctx, amount_in, ctx.accounts.mint_in.decimals)?;

  // Transfer output tokens from vault to user
  let config_key = config.key();
  let auth_seeds = &[b"auth", config_key.as_ref(), &[config.auth_bump]];
  let signer = &[&auth_seeds[..]];

  let transfer_out_ctx = CpiContext::new_with_signer(
    token_program_out.to_account_info(),
    TransferChecked {
      from: ctx.accounts.vault_out.to_account_info(),
      mint: ctx.accounts.mint_out.to_account_info(),
      to: ctx.accounts.user_token_out.to_account_info(),
      authority: ctx.accounts.pool_authority.to_account_info(),
    },
    signer,
  );
  transfer_checked(transfer_out_ctx, amount_out, ctx.accounts.mint_out.decimals)?;

  // Update pool reserves
  if is_x_to_y {
    pool_state.reserve_x = pool_state.reserve_x.checked_add(amount_in).unwrap();
    pool_state.reserve_y = pool_state.reserve_y.checked_sub(amount_out).unwrap();
  } else {
    pool_state.reserve_y = pool_state.reserve_y.checked_add(amount_in).unwrap();
    pool_state.reserve_x = pool_state.reserve_x.checked_sub(amount_out).unwrap();
  }

  msg!(
    "Swapped {} tokens in for {} tokens out",
    amount_in,
    amount_out
  );

  Ok(())
}
