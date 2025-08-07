use anchor_lang::prelude::*;

pub const MAX_WHITE_LIST_LP: usize = 10;

#[account]
#[derive(InitSpace, Copy)]
pub struct Config {
  pub seed: u64,
  pub authority: Pubkey,
  pub mint_x: Pubkey,
  pub mint_y: Pubkey,
  pub fee: u16,
  pub locked: bool,

  pub white_list_lp: Option<[Pubkey; MAX_WHITE_LIST_LP]>,

  pub auth_bump: u8,
  pub config_bump: u8,
  pub lp_bump: u8,
}

pub struct InitConfigParams {
  pub seed: u64,
  pub authority: Pubkey,
  pub mint_x: Pubkey,
  pub mint_y: Pubkey,
  pub fee: u16,
  pub white_list_lp: Option<[Pubkey; MAX_WHITE_LIST_LP]>,
  pub auth_bump: u8,
  pub config_bump: u8,
  pub lp_bump: u8,
}

impl Config {
  pub fn init(&mut self, params: InitConfigParams) {
    let InitConfigParams {
      seed,
      authority,
      mint_x,
      mint_y,
      fee,
      white_list_lp,
      auth_bump,
      config_bump,
      lp_bump,
    } = params;

    self.seed = seed;
    self.authority = authority;
    self.mint_x = mint_x;
    self.mint_y = mint_y;
    self.fee = fee;
    self.locked = false;
    self.white_list_lp = white_list_lp;
    self.auth_bump = auth_bump;
    self.config_bump = config_bump;
    self.lp_bump = lp_bump;

    msg!("Pool initialized with seed: {}, fee: {}", seed, fee);
    msg!("Mint X: {}, Mint Y: {}", mint_x, mint_y);
  }
}

#[account]
#[derive(InitSpace, Copy)]
pub struct PoolState {
  pub config: Pubkey,
  pub vault_x: Pubkey,
  pub vault_y: Pubkey,
  pub lp_mint: Pubkey,
  pub reserve_x: u64,
  pub reserve_y: u64,
  pub lp_supply: u64,
}

pub struct InitPoolStateParams {
  pub config: Pubkey,
  pub vault_x: Pubkey,
  pub vault_y: Pubkey,
  pub lp_mint: Pubkey,
}

impl PoolState {
  pub fn init(&mut self, params: InitPoolStateParams) {
    let InitPoolStateParams {
      config,
      lp_mint,
      vault_x,
      vault_y,
    } = params;

    self.config = config;
    self.lp_mint = lp_mint;
    self.vault_x = vault_x;
    self.vault_y = vault_y;
    self.reserve_x = 0;
    self.reserve_y = 0;
    self.lp_supply = 0;
  }
}
