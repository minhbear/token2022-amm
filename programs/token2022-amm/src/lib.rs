use anchor_lang::prelude::*;

mod common;
mod instructions;
mod state;
mod utils;

use instructions::*;

declare_id!("2AXqNb7CQRbS9z7U2NXZXVmzrJ3FxD2ztxiVASfgxUL2");

#[program]
pub mod token2022_amm {
  use super::*;

  pub fn initialize_pool(
    ctx: Context<InitializePool>,
    seed: u64,
    fee: u16,
    white_list_lp: Option<[Pubkey; 10]>,
  ) -> Result<()> {
    init_pool::handler(ctx, seed, fee, white_list_lp)
  }

  pub fn deposit(
    ctx: Context<Deposit>,
    amount_x: u64,
    amount_y: u64,
    min_lp_out: u64,
  ) -> Result<()> {
    deposit::handler(ctx, amount_x, amount_y, min_lp_out)
  }

  pub fn withdraw(
    ctx: Context<Withdraw>,
    lp_amount: u64,
    min_amount_x: u64,
    min_amount_y: u64,
  ) -> Result<()> {
    withdraw::handler(ctx, lp_amount, min_amount_x, min_amount_y)
  }

  pub fn swap(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
    swap::handler(ctx, amount_in, min_amount_out)
  }
}
