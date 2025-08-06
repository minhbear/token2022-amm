use {
  crate::common::{constant::NOT_ALLOW_TOKEN_EXTS, error::AMMError},
  anchor_lang::prelude::*,
  anchor_spl::{
    token::Token,
    token_2022::spl_token_2022::{
      self,
      extension::{
        self,
        transfer_fee::{TransferFee, MAX_FEE_BASIS_POINTS},
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
      },
    },
    token_interface::Mint,
  },
};

pub fn verify_supported_token_mint(token_mint: &InterfaceAccount<'_, Mint>) -> Result<bool> {
  let token_mint_info = token_mint.to_account_info();

  // if mint is owned by Token Program, it is supported (compatible to initialize_pool / initialize_reward)
  if *token_mint_info.owner == Token::id() {
    return Ok(true);
  }

  // now mint is owned by Token-2022 Program

  // reject native mint of Token-2022 Program to avoid SOL liquidity fragmentation
  if spl_token_2022::native_mint::check_id(&token_mint.key()) {
    return Ok(false);
  }

  // reject if mint has freeze_authority
  if token_mint.freeze_authority.is_some() {
    return Ok(false);
  }

  let token_mint_data = token_mint_info.try_borrow_data()?;
  let token_mint_unpacked =
    StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;

  let tlv_data = token_mint_unpacked.get_tlv_data();
  let extensions = get_token_extension_types(tlv_data)?;

  // Check if any extension is in the NOT_ALLOW_TOKEN_EXTS list
  for extension in extensions {
    if NOT_ALLOW_TOKEN_EXTS.contains(&extension) {
      return Err(AMMError::NotAllowedTokenExtension.into());
    }

    match extension {
      // supported extensions
      ExtensionType::TransferFeeConfig => {}
      ExtensionType::InterestBearingConfig => {}
      ExtensionType::TokenMetadata => {}
      ExtensionType::MetadataPointer => {}
      // partially supported
      ExtensionType::ConfidentialTransferMint => {
        // Supported, but non-confidential transfer only
      }
      ExtensionType::ConfidentialTransferFeeConfig => {
        // Supported, but non-confidential transfer only
      }
      // explicitly not allowed (handled above by NOT_ALLOW_TOKEN_EXTS check)
      ExtensionType::PermanentDelegate
      | ExtensionType::TransferHook
      | ExtensionType::NonTransferable => {
        // This should not be reached due to the check above, but kept for safety
        return Err(AMMError::NotAllowedTokenExtension.into());
      }
      // other unsupported extensions
      ExtensionType::MintCloseAuthority | ExtensionType::DefaultAccountState => {
        return Err(AMMError::NotAllowedTokenExtension.into());
      }
      // Catch-all for unknown/future extensions - be conservative
      _ => {
        return Err(AMMError::NotAllowedTokenExtension.into());
      }
    }
  }

  Ok(true)
}

// reference implementation: get_tlv_data_info
// https://github.com/solana-program/token-2022/blob/1c1a20cfa930058a853e15821112571b383c3e70/program/src/extension/mod.rs#L203
fn get_token_extension_types(tlv_data: &[u8]) -> Result<Vec<ExtensionType>> {
  const TLV_TYPE_LENGTH: usize = 2;
  const TLV_LENGTH_LENGTH: usize = 2;

  let mut extension_types = Vec::new();
  let mut cursor = 0;

  while cursor < tlv_data.len() {
    let tlv_type_start = cursor;
    let tlv_length_start = tlv_type_start + TLV_TYPE_LENGTH;
    let tlv_value_start = tlv_length_start + TLV_LENGTH_LENGTH;

    if tlv_data.len() < tlv_length_start {
      // There aren't enough bytes to store the next type, which means we
      // got to the end. The last byte could be used during a realloc!
      return Ok(extension_types);
    }

    let extension_type_num = read_u16_le_from_slice(&tlv_data[tlv_type_start..tlv_length_start])?;
    let extension_type =
      ExtensionType::try_from(extension_type_num).map_err(|_| ProgramError::InvalidAccountData)?;

    if extension_type == ExtensionType::Uninitialized {
      return Ok(extension_types);
    } else {
      if tlv_data.len() < tlv_value_start {
        // not enough bytes to store the length, malformed
        return Err(ProgramError::InvalidAccountData.into());
      }
      extension_types.push(extension_type);
      let length = read_u16_le_from_slice(&tlv_data[tlv_length_start..tlv_value_start])?;

      let value_end_index = tlv_value_start.saturating_add(usize::from(length));
      if value_end_index > tlv_data.len() {
        // value blows past the size of the slice, malformed
        return Err(ProgramError::InvalidAccountData.into());
      }
      cursor = value_end_index;
    }
  }

  Ok(extension_types)
}

fn read_u16_le_from_slice(slice: &[u8]) -> Result<u16> {
  if slice.len() < 2 {
    return Err(ProgramError::InvalidAccountData.into());
  }
  Ok(u16::from_le_bytes(
    slice[0..2]
      .try_into()
      .map_err(|_| ProgramError::InvalidAccountData)?,
  ))
}

#[derive(Debug)]
pub struct TransferFeeIncludedAmount {
  pub amount: u64,
  pub transfer_fee: u64,
}

#[derive(Debug)]
pub struct TransferFeeExcludedAmount {
  pub amount: u64,
  pub transfer_fee: u64,
}

pub fn calculate_transfer_fee_excluded_amount(
  token_mint: &InterfaceAccount<'_, Mint>,
  transfer_fee_included_amount: u64,
) -> Result<TransferFeeExcludedAmount> {
  if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
    let transfer_fee = epoch_transfer_fee
      .calculate_fee(transfer_fee_included_amount)
      .unwrap();
    let transfer_fee_excluded_amount = transfer_fee_included_amount
      .checked_sub(transfer_fee)
      .unwrap();
    return Ok(TransferFeeExcludedAmount {
      amount: transfer_fee_excluded_amount,
      transfer_fee,
    });
  }

  Ok(TransferFeeExcludedAmount {
    amount: transfer_fee_included_amount,
    transfer_fee: 0,
  })
}

pub fn calculate_transfer_fee_included_amount(
  token_mint: &InterfaceAccount<'_, Mint>,
  transfer_fee_excluded_amount: u64,
) -> Result<TransferFeeIncludedAmount> {
  if transfer_fee_excluded_amount == 0 {
    return Ok(TransferFeeIncludedAmount {
      amount: 0,
      transfer_fee: 0,
    });
  }

  // now transfer_fee_excluded_amount > 0

  if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
    let transfer_fee: u64 =
      if u16::from(epoch_transfer_fee.transfer_fee_basis_points) == MAX_FEE_BASIS_POINTS {
        // edge-case: if transfer fee rate is 100%, current SPL implementation returns 0 as inverse fee.
        // https://github.com/solana-labs/solana-program-library/blob/fe1ac9a2c4e5d85962b78c3fc6aaf028461e9026/token/program-2022/src/extension/transfer_fee/mod.rs#L95

        // But even if transfer fee is 100%, we can use maximum_fee as transfer fee.
        // if transfer_fee_excluded_amount + maximum_fee > u64 max, the following checked_add should fail.
        u64::from(epoch_transfer_fee.maximum_fee)
      } else {
        epoch_transfer_fee
          .calculate_inverse_fee(transfer_fee_excluded_amount)
          .ok_or(AMMError::TransferFeeCalculationError)?
      };

    let transfer_fee_included_amount = transfer_fee_excluded_amount
      .checked_add(transfer_fee)
      .ok_or(AMMError::TransferFeeCalculationError)?;

    // verify transfer fee calculation for safety
    let transfer_fee_verification = epoch_transfer_fee
      .calculate_fee(transfer_fee_included_amount)
      .unwrap();
    if transfer_fee != transfer_fee_verification {
      // We believe this should never happen
      return Err(AMMError::TransferFeeCalculationError.into());
    }

    return Ok(TransferFeeIncludedAmount {
      amount: transfer_fee_included_amount,
      transfer_fee,
    });
  }

  Ok(TransferFeeIncludedAmount {
    amount: transfer_fee_excluded_amount,
    transfer_fee: 0,
  })
}

pub fn get_epoch_transfer_fee(
  token_mint: &InterfaceAccount<'_, Mint>,
) -> Result<Option<TransferFee>> {
  let token_mint_info = token_mint.to_account_info();
  if *token_mint_info.owner == Token::id() {
    return Ok(None);
  }

  let token_mint_data = token_mint_info.try_borrow_data()?;
  let token_mint_unpacked =
    StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;
  if let Ok(transfer_fee_config) =
    token_mint_unpacked.get_extension::<extension::transfer_fee::TransferFeeConfig>()
  {
    let epoch = Clock::get()?.epoch;
    return Ok(Some(*transfer_fee_config.get_epoch_fee(epoch)));
  }

  Ok(None)
}
