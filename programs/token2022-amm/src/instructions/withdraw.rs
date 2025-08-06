use {
  crate::{
    common::error::AMMError,
    state::{Config, PoolState},
  },
  anchor_lang::prelude::*,
  anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
      burn, transfer_checked, Burn, Mint as MintInterface, TokenAccount, TokenInterface,
      TransferChecked,
    },
  },
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

  pub mint_x: InterfaceAccount<'info, MintInterface>,
  pub mint_y: InterfaceAccount<'info, MintInterface>,

  #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program_x,
    )]
  pub vault_x: InterfaceAccount<'info, TokenAccount>,

  #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program_y,
    )]
  pub vault_y: InterfaceAccount<'info, TokenAccount>,

  #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program_x,
    )]
  pub user_token_x: InterfaceAccount<'info, TokenAccount>,

  #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program_y,
    )]
  pub user_token_y: InterfaceAccount<'info, TokenAccount>,

  #[account(
        mut,
        seeds = [b"lp_mint", config.key().as_ref()],
        bump = config.lp_bump
    )]
  pub lp_mint: InterfaceAccount<'info, MintInterface>,

  #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program_lp,
    )]
  pub user_lp_token: InterfaceAccount<'info, TokenAccount>,

  pub token_program_x: Interface<'info, TokenInterface>,
  pub token_program_y: Interface<'info, TokenInterface>,
  pub token_program_lp: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}

pub fn handler(
  ctx: Context<Withdraw>,
  lp_amount: u64,
  min_amount_x: u64,
  min_amount_y: u64,
) -> Result<()> {
  let pool_state = &mut ctx.accounts.pool_state;
  let config = &ctx.accounts.config;

  require!(lp_amount > 0, AMMError::InvalidAmount);
  require!(pool_state.lp_supply > 0, AMMError::InsufficientLiquidity);

  // Calculate proportional withdrawal amounts
  let amount_x = (lp_amount as u128)
    .checked_mul(pool_state.reserve_x as u128)
    .unwrap()
    .checked_div(pool_state.lp_supply as u128)
    .unwrap() as u64;

  let amount_y = (lp_amount as u128)
    .checked_mul(pool_state.reserve_y as u128)
    .unwrap()
    .checked_div(pool_state.lp_supply as u128)
    .unwrap() as u64;

  // Check slippage
  require!(amount_x >= min_amount_x, AMMError::SlippageExceeded);
  require!(amount_y >= min_amount_y, AMMError::SlippageExceeded);

  // Burn LP tokens from user
  let burn_ctx = CpiContext::new(
    ctx.accounts.token_program_lp.to_account_info(),
    Burn {
      mint: ctx.accounts.lp_mint.to_account_info(),
      from: ctx.accounts.user_lp_token.to_account_info(),
      authority: ctx.accounts.user.to_account_info(),
    },
  );
  burn(burn_ctx, lp_amount)?;

  // Transfer tokens from vault to user
  let config_key = config.key();
  let auth_seeds = &[b"auth", config_key.as_ref(), &[config.auth_bump]];
  let signer = &[&auth_seeds[..]];

  let transfer_x_ctx = CpiContext::new_with_signer(
    ctx.accounts.token_program_x.to_account_info(),
    TransferChecked {
      from: ctx.accounts.vault_x.to_account_info(),
      mint: ctx.accounts.mint_x.to_account_info(),
      to: ctx.accounts.user_token_x.to_account_info(),
      authority: ctx.accounts.pool_authority.to_account_info(),
    },
    signer,
  );
  transfer_checked(transfer_x_ctx, amount_x, ctx.accounts.mint_x.decimals)?;

  let transfer_y_ctx = CpiContext::new_with_signer(
    ctx.accounts.token_program_y.to_account_info(),
    TransferChecked {
      from: ctx.accounts.vault_y.to_account_info(),
      mint: ctx.accounts.mint_y.to_account_info(),
      to: ctx.accounts.user_token_y.to_account_info(),
      authority: ctx.accounts.pool_authority.to_account_info(),
    },
    signer,
  );
  transfer_checked(transfer_y_ctx, amount_y, ctx.accounts.mint_y.decimals)?;

  // Update pool state
  pool_state.reserve_x = pool_state.reserve_x.checked_sub(amount_x).unwrap();
  pool_state.reserve_y = pool_state.reserve_y.checked_sub(amount_y).unwrap();
  pool_state.lp_supply = pool_state.lp_supply.checked_sub(lp_amount).unwrap();

  msg!(
    "Withdrew {} token X, {} token Y, burned {} LP tokens",
    amount_x,
    amount_y,
    lp_amount
  );

  Ok(())
}
