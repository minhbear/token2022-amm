pub const DISCRIMINATOR: usize = 8;

pub mod seed_prefix {
  pub const CONFIG: &[u8] = b"config";
  pub const POOL: &[u8] = b"pool";
  pub const LP_MINT: &[u8] = b"lp_mint";
  pub const AUTH: &[u8] = b"auth";
}
