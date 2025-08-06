use anchor_lang::prelude::*;

#[account]
pub struct Config {
  pub seed: u64,
  pub authority: Pubkey,
  pub mint_x: Pubkey,
  pub mint_y: Pubkey,
  pub fee: u16,
  pub locked: bool,

  pub white_list_lp: Option<[Pubkey; 50]>,

  pub auth_bump: u8,
  pub config_bump: u8,
  pub lp_bump: u8,
}

impl Config {
  pub const LEN: usize = 8 + // discriminator
        8 + // seed
        32 + // authority
        32 + // mint_x
        32 + // mint_y
        2 + // fee
        1 + // locked
        1 + (50 * 32) + // white_list_lp (Option<[Pubkey; 50]>)
        1 + // auth_bump
        1 + // config_bump
        1; // lp_bump
}

#[account]
pub struct PoolState {
  pub config: Pubkey,
  pub vault_x: Pubkey,
  pub vault_y: Pubkey,
  pub lp_mint: Pubkey,
  pub reserve_x: u64,
  pub reserve_y: u64,
  pub lp_supply: u64,
}

impl PoolState {
  pub const LEN: usize = 8 + // discriminator
        32 + // config
        32 + // vault_x
        32 + // vault_y
        32 + // lp_mint
        8 + // reserve_x
        8 + // reserve_y
        8; // lp_supply
}
