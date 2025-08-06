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
  pub config: Box<Account<'info, Config>>,

  #[account(
        mut,
        seeds = [b"pool", config.key().as_ref()],
        bump
    )]
  pub pool_state: Box<Account<'info, PoolState>>,

  /// CHECK: PDA authority for the pool
  #[account(
        seeds = [b"auth", config.key().as_ref()],
        bump = config.auth_bump
    )]
  pub pool_authority: UncheckedAccount<'info>,

  pub mint_in: Box<InterfaceAccount<'info, MintInterface>>,
  pub mint_out: Box<InterfaceAccount<'info, MintInterface>>,

  #[account(
        mut,
        constraint = vault_in.key() == pool_state.vault_x || vault_in.key() == pool_state.vault_y,
        constraint = vault_in.mint == mint_in.key(),
        constraint = vault_in.owner == pool_authority.key(),
    )]
  pub vault_in: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        mut,
        constraint = vault_out.key() == pool_state.vault_x || vault_out.key() == pool_state.vault_y,
        constraint = vault_out.mint == mint_out.key(),
        constraint = vault_out.owner == pool_authority.key(),
        constraint = vault_in.key() != vault_out.key(),
    )]
  pub vault_out: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        mut,
        constraint = user_token_in.mint == mint_in.key(),
        constraint = user_token_in.owner == user.key(),
    )]
  pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        mut,
        constraint = user_token_out.mint == mint_out.key(),
        constraint = user_token_out.owner == user.key(),
    )]
  pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

  pub token_program_x: Interface<'info, TokenInterface>,
  pub token_program_y: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
  let pool_state = &mut ctx.accounts.pool_state;
  let config = &ctx.accounts.config;

  require!(amount_in > 0, AMMError::InvalidAmount);

  // Verify mint constraints
  require!(
    ctx.accounts.mint_in.key() == config.mint_x || ctx.accounts.mint_in.key() == config.mint_y,
    AMMError::InvalidMint
  );
  require!(
    ctx.accounts.mint_out.key() == config.mint_x || ctx.accounts.mint_out.key() == config.mint_y,
    AMMError::InvalidMint
  );
  require!(
    ctx.accounts.mint_in.key() != ctx.accounts.mint_out.key(),
    AMMError::InvalidMint
  );

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

  // For Token2022 tokens with transfer fees, the vault balance might be less than reserves
  // due to fees being collected, so we use a more lenient check
  // We ensure the vault has at least enough for the output amount
  require!(
    ctx.accounts.vault_out.amount > 0,
    AMMError::InsufficientLiquidity
  );

  // Calculate output amount using constant product formula with fee
  // amount_out = (amount_in * (10000 - fee) * reserve_out) / ((reserve_in * 10000) + (amount_in * (10000 - fee)))
  let fee_adjusted_amount_in = (amount_in as u128)
    .checked_mul(
      (10000u128)
        .checked_sub(config.fee as u128)
        .ok_or(AMMError::InvalidAmount)?,
    )
    .ok_or(AMMError::InvalidAmount)?;

  let numerator = fee_adjusted_amount_in
    .checked_mul(reserve_out as u128)
    .ok_or(AMMError::InvalidAmount)?;

  let denominator = (reserve_in as u128)
    .checked_mul(10000u128)
    .ok_or(AMMError::InvalidAmount)?
    .checked_add(fee_adjusted_amount_in)
    .ok_or(AMMError::InvalidAmount)?;

  require!(denominator > 0, AMMError::InvalidAmount);
  let amount_out = numerator
    .checked_div(denominator)
    .ok_or(AMMError::InvalidAmount)? as u64;

  require!(amount_out >= min_amount_out, AMMError::SlippageExceeded);
  require!(amount_out > 0, AMMError::InsufficientOutputAmount);
  require!(amount_out <= reserve_out, AMMError::InsufficientLiquidity);

  // Ensure vault has enough tokens for the swap (accounting for potential transfer fees)
  require!(
    ctx.accounts.vault_out.amount >= amount_out,
    AMMError::InsufficientLiquidity
  );

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
    pool_state.reserve_x = pool_state
      .reserve_x
      .checked_add(amount_in)
      .ok_or(AMMError::InvalidAmount)?;
    pool_state.reserve_y = pool_state
      .reserve_y
      .checked_sub(amount_out)
      .ok_or(AMMError::InvalidAmount)?;
  } else {
    pool_state.reserve_y = pool_state
      .reserve_y
      .checked_add(amount_in)
      .ok_or(AMMError::InvalidAmount)?;
    pool_state.reserve_x = pool_state
      .reserve_x
      .checked_sub(amount_out)
      .ok_or(AMMError::InvalidAmount)?;
  }

  msg!(
    "Swapped {} tokens in for {} tokens out",
    amount_in,
    amount_out
  );

  Ok(())
}
