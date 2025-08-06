use anchor_spl::token_2022::spl_token_2022::extension::ExtensionType;

// Token extensions that are not allowed in the AMM
pub const NOT_ALLOW_TOKEN_EXTS: [ExtensionType; 3] = [
  ExtensionType::NonTransferable,
  ExtensionType::TransferHook,
  ExtensionType::PermanentDelegate,
];
