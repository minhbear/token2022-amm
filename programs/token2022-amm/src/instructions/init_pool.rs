use {
  crate::{
    common::error::AMMError,
    state::{Config, PoolState},
    utils::token::verify_supported_token_mint,
  },
  anchor_lang::prelude::*,
  anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint as MintInterface, TokenAccount, TokenInterface},
  },
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct InitializePool<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,

  #[account(
        init,
        payer = authority,
        space = Config::LEN,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump
    )]
  pub config: Box<Account<'info, Config>>,

  #[account(
        init,
        payer = authority,
        space = PoolState::LEN,
        seeds = [b"pool", config.key().as_ref()],
        bump
    )]
  pub pool_state: Box<Account<'info, PoolState>>,

  pub mint_x: Box<InterfaceAccount<'info, MintInterface>>,
  pub mint_y: Box<InterfaceAccount<'info, MintInterface>>,

  #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = pool_authority,
        mint::token_program = token_program_lp,
        seeds = [b"lp_mint", config.key().as_ref()],
        bump
    )]
  pub lp_mint: Box<InterfaceAccount<'info, MintInterface>>,

  /// CHECK: PDA authority for the pool
  #[account(
        seeds = [b"auth", config.key().as_ref()],
        bump
    )]
  pub pool_authority: UncheckedAccount<'info>,

  #[account(
        init,
        payer = authority,
        associated_token::mint = mint_x,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program_x,
    )]
  pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,

  #[account(
        init,
        payer = authority,
        associated_token::mint = mint_y,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program_y,
    )]
  pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,

  pub token_program_x: Interface<'info, TokenInterface>,
  pub token_program_y: Interface<'info, TokenInterface>,
  pub token_program_lp: Interface<'info, TokenInterface>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}

pub fn handler(
  ctx: Context<InitializePool>,
  seed: u64,
  fee: u16,
  white_list_lp: Option<[Pubkey; 10]>,
) -> Result<()> {
  let config = &mut ctx.accounts.config;
  let pool_state = &mut ctx.accounts.pool_state;

  // Validate fee is within reasonable bounds (max 10% = 1000 basis points)
  require!(fee <= 1000, AMMError::InvalidAmount);

  // Verify both tokens are supported (legacy SPL or Token-2022 with allowed extensions)
  let mint_x_supported = verify_supported_token_mint(&ctx.accounts.mint_x)?;
  let mint_y_supported = verify_supported_token_mint(&ctx.accounts.mint_y)?;

  require!(
    mint_x_supported && mint_y_supported,
    AMMError::NotAllowedTokenExtension
  );

  // Ensure mint_x and mint_y are different
  require!(
    ctx.accounts.mint_x.key() != ctx.accounts.mint_y.key(),
    AMMError::InvalidMint
  );

  // Additional security: check for consistent token programs
  let mint_x_info = ctx.accounts.mint_x.to_account_info();
  let mint_y_info = ctx.accounts.mint_y.to_account_info();

  // Verify token programs match the mint owners
  if *mint_x_info.owner == anchor_spl::token::Token::id() {
    require!(
      ctx.accounts.token_program_x.key() == anchor_spl::token::Token::id(),
      AMMError::InvalidMint
    );
  } else {
    require!(
      ctx.accounts.token_program_x.key() == anchor_spl::token_2022::Token2022::id(),
      AMMError::InvalidMint
    );
  }

  if *mint_y_info.owner == anchor_spl::token::Token::id() {
    require!(
      ctx.accounts.token_program_y.key() == anchor_spl::token::Token::id(),
      AMMError::InvalidMint
    );
  } else {
    require!(
      ctx.accounts.token_program_y.key() == anchor_spl::token_2022::Token2022::id(),
      AMMError::InvalidMint
    );
  }

  // Initialize config
  config.seed = seed;
  config.authority = ctx.accounts.authority.key();
  config.mint_x = ctx.accounts.mint_x.key();
  config.mint_y = ctx.accounts.mint_y.key();
  config.fee = fee;
  config.locked = false;
  config.white_list_lp = white_list_lp;
  config.auth_bump = ctx.bumps.pool_authority;
  config.config_bump = ctx.bumps.config;
  config.lp_bump = ctx.bumps.lp_mint;

  // Initialize pool state
  pool_state.config = config.key();
  pool_state.vault_x = ctx.accounts.vault_x.key();
  pool_state.vault_y = ctx.accounts.vault_y.key();
  pool_state.lp_mint = ctx.accounts.lp_mint.key();
  pool_state.reserve_x = 0;
  pool_state.reserve_y = 0;
  pool_state.lp_supply = 0;

  msg!("Pool initialized with seed: {}, fee: {}", seed, fee);
  msg!("Mint X: {}, Mint Y: {}", config.mint_x, config.mint_y);

  Ok(())
}
