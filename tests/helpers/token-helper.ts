import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getMintLen,
  ExtensionType,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  getAssociatedTokenAddressSync,
} from '@solana/spl-token';

export interface TokenInfo {
  mint: PublicKey;
  decimals: number;
  tokenProgram: PublicKey;
  hasTransferFee: boolean;
  transferFeeBasisPoints?: number;
  maxTransferFee?: bigint;
}

export interface UserTokenAccounts {
  tokenX: PublicKey;
  tokenY: PublicKey;
  lpToken?: PublicKey;
}

/**
 * Creates a legacy SPL token
 */
export async function createLegacyToken(
  connection: Connection,
  payer: Keypair,
  mintAuthority: PublicKey,
  decimals: number = 6
): Promise<TokenInfo> {
  const mint = await createMint(
    connection,
    payer,
    mintAuthority,
    null,
    decimals,
    undefined,
    undefined,
    TOKEN_PROGRAM_ID
  );

  return {
    mint,
    decimals,
    tokenProgram: TOKEN_PROGRAM_ID,
    hasTransferFee: false,
  };
}

/**
 * Creates a Token-2022 with TransferFee extension
 */
export async function createToken2022WithTransferFee(
  connection: Connection,
  payer: Keypair,
  mintAuthority: PublicKey,
  transferFeeConfigAuthority: PublicKey,
  withdrawWithheldAuthority: PublicKey,
  transferFeeBasisPoints: number = 100, // 1%
  maxTransferFee: bigint = BigInt(100 * 10 ** 6), // 100 tokens
  decimals: number = 6
): Promise<TokenInfo> {
  const mintKeypair = Keypair.generate();
  const mint = mintKeypair.publicKey;

  // Calculate space needed for mint with TransferFee extension
  const extensions = [ExtensionType.TransferFeeConfig];
  const mintLen = getMintLen(extensions);
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

  // Create account instruction
  const createAccountInstruction = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: mint,
    space: mintLen,
    lamports,
    programId: TOKEN_2022_PROGRAM_ID,
  });

  // Initialize TransferFee extension
  const initializeTransferFeeInstruction =
    createInitializeTransferFeeConfigInstruction(
      mint,
      transferFeeConfigAuthority,
      withdrawWithheldAuthority,
      transferFeeBasisPoints,
      maxTransferFee,
      TOKEN_2022_PROGRAM_ID
    );

  // Initialize mint
  const initializeMintInstruction = createInitializeMintInstruction(
    mint,
    decimals,
    mintAuthority,
    null,
    TOKEN_2022_PROGRAM_ID
  );

  // Send transaction
  const transaction = new Transaction().add(
    createAccountInstruction,
    initializeTransferFeeInstruction,
    initializeMintInstruction
  );

  await sendAndConfirmTransaction(connection, transaction, [
    payer,
    mintKeypair,
  ]);

  return {
    mint,
    decimals,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    hasTransferFee: true,
    transferFeeBasisPoints,
    maxTransferFee,
  };
}

/**
 * Creates user token accounts for a given mint
 */
export async function createUserTokenAccount(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey,
  tokenProgram: PublicKey
): Promise<PublicKey> {
  return await createAssociatedTokenAccount(
    connection,
    payer,
    mint,
    owner,
    undefined,
    tokenProgram,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

/**
 * Gets the associated token address for a mint and owner
 */
export function getTokenAddress(
  mint: PublicKey,
  owner: PublicKey,
  tokenProgram: PublicKey,
  allowOwnerOffCurve: boolean = false
): PublicKey {
  return getAssociatedTokenAddressSync(
    mint,
    owner,
    allowOwnerOffCurve,
    tokenProgram,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

/**
 * Creates token accounts for a user for both tokens in a pair
 */
export async function createUserTokenAccounts(
  connection: Connection,
  payer: Keypair,
  user: PublicKey,
  tokenX: TokenInfo,
  tokenY: TokenInfo,
  lpMint?: PublicKey,
  lpTokenProgram?: PublicKey
): Promise<UserTokenAccounts> {
  const [tokenXAccount, tokenYAccount] = await Promise.all([
    createUserTokenAccount(
      connection,
      payer,
      tokenX.mint,
      user,
      tokenX.tokenProgram
    ),
    createUserTokenAccount(
      connection,
      payer,
      tokenY.mint,
      user,
      tokenY.tokenProgram
    ),
  ]);

  let lpTokenAccount: PublicKey | undefined;
  if (lpMint && lpTokenProgram) {
    lpTokenAccount = await createUserTokenAccount(
      connection,
      payer,
      lpMint,
      user,
      lpTokenProgram
    );
  }

  return {
    tokenX: tokenXAccount,
    tokenY: tokenYAccount,
    lpToken: lpTokenAccount,
  };
}

/**
 * Mints tokens to a user account
 */
export async function mintTokensToUser(
  connection: Connection,
  payer: Keypair,
  tokenInfo: TokenInfo,
  userTokenAccount: PublicKey,
  mintAuthority: Keypair,
  amount: number
): Promise<void> {
  await mintTo(
    connection,
    payer,
    tokenInfo.mint,
    userTokenAccount,
    mintAuthority,
    amount,
    undefined,
    undefined,
    tokenInfo.tokenProgram
  );
}

/**
 * Setup function to create all tokens and accounts for testing
 */
export async function setupTestTokens(
  connection: Connection,
  authority: Keypair,
  users: PublicKey[],
  tokenXType: 'legacy' | 'token2022',
  tokenYType: 'legacy' | 'token2022'
): Promise<{
  tokenX: TokenInfo;
  tokenY: TokenInfo;
  userAccounts: Map<string, UserTokenAccounts>;
}> {
  console.log(`\n🔧 Setting up tokens: ${tokenXType} + ${tokenYType}`);

  // Create tokens based on type
  const tokenX =
    tokenXType === 'legacy'
      ? await createLegacyToken(connection, authority, authority.publicKey)
      : await createToken2022WithTransferFee(
          connection,
          authority,
          authority.publicKey,
          authority.publicKey,
          authority.publicKey
        );

  const tokenY =
    tokenYType === 'legacy'
      ? await createLegacyToken(connection, authority, authority.publicKey)
      : await createToken2022WithTransferFee(
          connection,
          authority,
          authority.publicKey,
          authority.publicKey,
          authority.publicKey
        );

  console.log(`✅ Token X (${tokenXType}): ${tokenX.mint.toString()}`);
  console.log(`✅ Token Y (${tokenYType}): ${tokenY.mint.toString()}`);

  // Create user accounts
  const userAccounts = new Map<string, UserTokenAccounts>();
  for (const user of users) {
    const accounts = await createUserTokenAccounts(
      connection,
      authority,
      user,
      tokenX,
      tokenY
    );
    userAccounts.set(user.toString(), accounts);
  }

  console.log(`✅ Created token accounts for ${users.length} users`);

  return {
    tokenX,
    tokenY,
    userAccounts,
  };
}

/**
 * Fund users with tokens for testing
 */
export async function fundUsers(
  connection: Connection,
  authority: Keypair,
  tokenX: TokenInfo,
  tokenY: TokenInfo,
  userAccounts: Map<string, UserTokenAccounts>,
  amountX: number = 10000 * 10 ** 6, // 10,000 tokens
  amountY: number = 10000 * 10 ** 6 // 10,000 tokens
): Promise<void> {
  console.log('\n💰 Funding users with tokens...');

  for (const [userKey, accounts] of userAccounts) {
    await Promise.all([
      mintTokensToUser(
        connection,
        authority,
        tokenX,
        accounts.tokenX,
        authority,
        amountX
      ),
      mintTokensToUser(
        connection,
        authority,
        tokenY,
        accounts.tokenY,
        authority,
        amountY
      ),
    ]);
  }

  console.log(
    `✅ Funded ${userAccounts.size} users with ${
      amountX / 10 ** 6
    } X tokens and ${amountY / 10 ** 6} Y tokens each`
  );
}
