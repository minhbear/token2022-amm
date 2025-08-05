use anchor_lang::prelude::*;

declare_id!("J4a3aueEwbNTW84q6DnEvVZG2EC182fvnXYSnoE6zDkF");

#[program]
pub mod token2022_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
