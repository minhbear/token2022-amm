use {
  crate::{
    common::error::AMMError,
    state::{Config, PoolState},
  },
  anchor_lang::prelude::*,
  anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
      mint_to, transfer_checked, Mint as MintInterface, MintTo, TokenAccount, TokenInterface,
      TransferChecked,
    },
  },
};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        init_if_needed,
        payer = user,
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

pub fn handler(ctx: Context<Deposit>, amount_x: u64, amount_y: u64, min_lp_out: u64) -> Result<()> {
  let pool_state = &mut ctx.accounts.pool_state;
  let config = &ctx.accounts.config;

  // Check whitelist if enabled
  if let Some(whitelist) = &config.white_list_lp {
    let user_key = ctx.accounts.user.key();
    require!(whitelist.contains(&user_key), AMMError::NotWhitelisted);
  }

  let lp_tokens_to_mint = if pool_state.lp_supply == 0 {
    // Initial deposit - use geometric mean
    let initial_lp = (amount_x as u128)
      .checked_mul(amount_y as u128)
      .unwrap()
      .integer_sqrt() as u64;

    require!(initial_lp >= min_lp_out, AMMError::SlippageExceeded);
    initial_lp
  } else {
    // Proportional deposit
    let lp_from_x = (amount_x as u128)
      .checked_mul(pool_state.lp_supply as u128)
      .unwrap()
      .checked_div(pool_state.reserve_x as u128)
      .unwrap() as u64;

    let lp_from_y = (amount_y as u128)
      .checked_mul(pool_state.lp_supply as u128)
      .unwrap()
      .checked_div(pool_state.reserve_y as u128)
      .unwrap() as u64;

    let lp_tokens = lp_from_x.min(lp_from_y);
    require!(lp_tokens >= min_lp_out, AMMError::SlippageExceeded);
    lp_tokens
  };

  // Transfer tokens from user to vault
  let transfer_x_ctx = CpiContext::new(
    ctx.accounts.token_program_x.to_account_info(),
    TransferChecked {
      from: ctx.accounts.user_token_x.to_account_info(),
      mint: ctx.accounts.mint_x.to_account_info(),
      to: ctx.accounts.vault_x.to_account_info(),
      authority: ctx.accounts.user.to_account_info(),
    },
  );
  transfer_checked(transfer_x_ctx, amount_x, ctx.accounts.mint_x.decimals)?;

  let transfer_y_ctx = CpiContext::new(
    ctx.accounts.token_program_y.to_account_info(),
    TransferChecked {
      from: ctx.accounts.user_token_y.to_account_info(),
      mint: ctx.accounts.mint_y.to_account_info(),
      to: ctx.accounts.vault_y.to_account_info(),
      authority: ctx.accounts.user.to_account_info(),
    },
  );
  transfer_checked(transfer_y_ctx, amount_y, ctx.accounts.mint_y.decimals)?;

  // Mint LP tokens to user
  let config_key = config.key();
  let auth_seeds = &[b"auth", config_key.as_ref(), &[config.auth_bump]];
  let signer = &[&auth_seeds[..]];

  let mint_ctx = CpiContext::new_with_signer(
    ctx.accounts.token_program_lp.to_account_info(),
    MintTo {
      mint: ctx.accounts.lp_mint.to_account_info(),
      to: ctx.accounts.user_lp_token.to_account_info(),
      authority: ctx.accounts.pool_authority.to_account_info(),
    },
    signer,
  );
  mint_to(mint_ctx, lp_tokens_to_mint)?;

  // Update pool state
  pool_state.reserve_x = pool_state.reserve_x.checked_add(amount_x).unwrap();
  pool_state.reserve_y = pool_state.reserve_y.checked_add(amount_y).unwrap();
  pool_state.lp_supply = pool_state.lp_supply.checked_add(lp_tokens_to_mint).unwrap();

  msg!(
    "Deposited {} token X, {} token Y, minted {} LP tokens",
    amount_x,
    amount_y,
    lp_tokens_to_mint
  );

  Ok(())
}

// Helper trait for integer square root
trait IntegerSquareRoot {
  fn integer_sqrt(self) -> Self;
}

impl IntegerSquareRoot for u128 {
  fn integer_sqrt(self) -> Self {
    if self < 2 {
      return self;
    }

    let mut x = self;
    let mut y = (self + 1) / 2;

    while y < x {
      x = y;
      y = (x + self / x) / 2;
    }

    x
  }
}
