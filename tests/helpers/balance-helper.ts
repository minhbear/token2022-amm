import { Connection, PublicKey } from '@solana/web3.js';
import { 
  getAccount, 
  TOKEN_PROGRAM_ID, 
  TOKEN_2022_PROGRAM_ID,
  getTransferFeeAmount,
  unpackAccount
} from '@solana/spl-token';
import { Program } from '@coral-xyz/anchor';
import { Token2022Amm } from '../../target/types/token2022_amm';

export interface TokenBalance {
  balance: bigint;
  withheldFees?: bigint;
  effectiveBalance: bigint; // balance - withheld fees
}

export interface PoolSnapshot {
  reserveX: number;
  reserveY: number;
  lpSupply: number;
}

export interface UserSnapshot {
  tokenX: TokenBalance;
  tokenY: TokenBalance;
  lpToken: TokenBalance;
}

export interface CompleteSnapshot {
  user: UserSnapshot;
  pool: PoolSnapshot;
  timestamp: number;
}

export interface BalanceChanges {
  userXSpent: number;
  userYSpent: number;
  userXReceived: number;
  userYReceived: number;
  lpMinted: number;
  lpBurned: number;
  poolXChange: number;
  poolYChange: number;
  lpSupplyChange: number;
  feesCollected: {
    tokenX: number;
    tokenY: number;
  };
}

/**
 * Fetches token balance with transfer fee information
 */
export async function getTokenBalance(
  connection: Connection,
  tokenAccount: PublicKey,
  tokenProgram: PublicKey
): Promise<TokenBalance> {
  const account = await getAccount(connection, tokenAccount, undefined, tokenProgram);
  
  let withheldFees = BigInt(0);
  if (tokenProgram.equals(TOKEN_2022_PROGRAM_ID)) {
    try {
      const transferFeeAmount = getTransferFeeAmount(account);
      if (transferFeeAmount) {
        withheldFees = transferFeeAmount.withheldAmount;
      }
    } catch (e) {
      // No transfer fee extension or error reading it
    }
  }

  return {
    balance: account.amount,
    withheldFees,
    effectiveBalance: account.amount - withheldFees,
  };
}

/**
 * Takes a complete snapshot of user and pool state
 */
export async function takeSnapshot(
  connection: Connection,
  program: Program<Token2022Amm>,
  userTokenX: PublicKey,
  userTokenY: PublicKey,
  userLpToken: PublicKey,
  poolState: PublicKey,
  tokenProgramX: PublicKey,
  tokenProgramY: PublicKey,
  tokenProgramLP: PublicKey
): Promise<CompleteSnapshot> {
  // Get user balances
  const [userXBalance, userYBalance, userLpBalance] = await Promise.all([
    getTokenBalance(connection, userTokenX, tokenProgramX),
    getTokenBalance(connection, userTokenY, tokenProgramY),
    getTokenBalance(connection, userLpToken, tokenProgramLP),
  ]);

  // Get pool state
  const poolData = await program.account.poolState.fetch(poolState);

  return {
    user: {
      tokenX: userXBalance,
      tokenY: userYBalance,
      lpToken: userLpBalance,
    },
    pool: {
      reserveX: poolData.reserveX.toNumber(),
      reserveY: poolData.reserveY.toNumber(),
      lpSupply: poolData.lpSupply.toNumber(),
    },
    timestamp: Date.now(),
  };
}

/**
 * Calculates changes between two snapshots
 */
export function calculateChanges(
  before: CompleteSnapshot,
  after: CompleteSnapshot
): BalanceChanges {
  const userXChange = Number(before.user.tokenX.balance) - Number(after.user.tokenX.balance);
  const userYChange = Number(before.user.tokenY.balance) - Number(after.user.tokenY.balance);
  const lpChange = Number(after.user.lpToken.balance) - Number(before.user.lpToken.balance);
  
  const poolXChange = after.pool.reserveX - before.pool.reserveX;
  const poolYChange = after.pool.reserveY - before.pool.reserveY;
  const lpSupplyChange = after.pool.lpSupply - before.pool.lpSupply;

  // Calculate fees collected (increase in withheld amounts)
  const feesXCollected = Number(after.user.tokenX.withheldFees || 0) - Number(before.user.tokenX.withheldFees || 0);
  const feesYCollected = Number(after.user.tokenY.withheldFees || 0) - Number(before.user.tokenY.withheldFees || 0);

  return {
    userXSpent: Math.max(0, userXChange),
    userYSpent: Math.max(0, userYChange),
    userXReceived: Math.max(0, -userXChange),
    userYReceived: Math.max(0, -userYChange),
    lpMinted: Math.max(0, lpChange),
    lpBurned: Math.max(0, -lpChange),
    poolXChange,
    poolYChange,
    lpSupplyChange,
    feesCollected: {
      tokenX: feesXCollected,
      tokenY: feesYCollected,
    },
  };
}

/**
 * Logs a snapshot in a readable format
 */
export function logSnapshot(title: string, snapshot: CompleteSnapshot): void {
  console.log(`\n=== ${title} ===`);
  console.log(`Timestamp: ${new Date(snapshot.timestamp).toISOString()}`);
  
  console.log('\n👤 USER BALANCES:');
  console.log(`  Token X: ${snapshot.user.tokenX.balance.toString()}`);
  if (snapshot.user.tokenX.withheldFees && snapshot.user.tokenX.withheldFees > 0) {
    console.log(`    - Withheld fees: ${snapshot.user.tokenX.withheldFees.toString()}`);
    console.log(`    - Effective balance: ${snapshot.user.tokenX.effectiveBalance.toString()}`);
  }
  
  console.log(`  Token Y: ${snapshot.user.tokenY.balance.toString()}`);
  if (snapshot.user.tokenY.withheldFees && snapshot.user.tokenY.withheldFees > 0) {
    console.log(`    - Withheld fees: ${snapshot.user.tokenY.withheldFees.toString()}`);
    console.log(`    - Effective balance: ${snapshot.user.tokenY.effectiveBalance.toString()}`);
  }
  
  console.log(`  LP Token: ${snapshot.user.lpToken.balance.toString()}`);
  
  console.log('\n🏊 POOL STATE:');
  console.log(`  Reserve X: ${snapshot.pool.reserveX}`);
  console.log(`  Reserve Y: ${snapshot.pool.reserveY}`);
  console.log(`  LP Supply: ${snapshot.pool.lpSupply}`);
}

/**
 * Logs balance changes in a readable format
 */
export function logChanges(changes: BalanceChanges): void {
  console.log('\n📊 BALANCE CHANGES:');
  
  if (changes.userXSpent > 0) {
    console.log(`  User spent ${changes.userXSpent} Token X`);
  }
  if (changes.userXReceived > 0) {
    console.log(`  User received ${changes.userXReceived} Token X`);
  }
  
  if (changes.userYSpent > 0) {
    console.log(`  User spent ${changes.userYSpent} Token Y`);
  }
  if (changes.userYReceived > 0) {
    console.log(`  User received ${changes.userYReceived} Token Y`);
  }
  
  if (changes.lpMinted > 0) {
    console.log(`  LP tokens minted: ${changes.lpMinted}`);
  }
  if (changes.lpBurned > 0) {
    console.log(`  LP tokens burned: ${changes.lpBurned}`);
  }
  
  console.log(`  Pool X change: ${changes.poolXChange > 0 ? '+' : ''}${changes.poolXChange}`);
  console.log(`  Pool Y change: ${changes.poolYChange > 0 ? '+' : ''}${changes.poolYChange}`);
  console.log(`  LP supply change: ${changes.lpSupplyChange > 0 ? '+' : ''}${changes.lpSupplyChange}`);
  
  if (changes.feesCollected.tokenX > 0 || changes.feesCollected.tokenY > 0) {
    console.log('\n💰 TRANSFER FEES COLLECTED:');
    if (changes.feesCollected.tokenX > 0) {
      console.log(`  Token X fees: ${changes.feesCollected.tokenX}`);
    }
    if (changes.feesCollected.tokenY > 0) {
      console.log(`  Token Y fees: ${changes.feesCollected.tokenY}`);
    }
  }
}

/**
 * Validates that a deposit operation worked correctly
 */
export function validateDeposit(
  changes: BalanceChanges,
  expectedAmountX: number,
  expectedAmountY: number,
  allowTransferFees: boolean = true
): void {
  console.log('\n✅ VALIDATING DEPOSIT:');
  
  // User should have spent the expected amounts
  if (changes.userXSpent !== expectedAmountX) {
    throw new Error(`Expected user to spend ${expectedAmountX} X tokens, but spent ${changes.userXSpent}`);
  }
  if (changes.userYSpent !== expectedAmountY) {
    throw new Error(`Expected user to spend ${expectedAmountY} Y tokens, but spent ${changes.userYSpent}`);
  }
  
  // Pool should have received tokens (accounting for potential transfer fees)
  if (allowTransferFees) {
    if (changes.poolXChange < expectedAmountX - changes.feesCollected.tokenX) {
      throw new Error(`Pool X should have increased by at least ${expectedAmountX - changes.feesCollected.tokenX}, but increased by ${changes.poolXChange}`);
    }
    if (changes.poolYChange < expectedAmountY - changes.feesCollected.tokenY) {
      throw new Error(`Pool Y should have increased by at least ${expectedAmountY - changes.feesCollected.tokenY}, but increased by ${changes.poolYChange}`);
    }
  } else {
    if (changes.poolXChange !== expectedAmountX) {
      throw new Error(`Pool X should have increased by ${expectedAmountX}, but increased by ${changes.poolXChange}`);
    }
    if (changes.poolYChange !== expectedAmountY) {
      throw new Error(`Pool Y should have increased by ${expectedAmountY}, but increased by ${changes.poolYChange}`);
    }
  }
  
  // LP tokens should have been minted
  if (changes.lpMinted <= 0) {
    throw new Error(`Expected LP tokens to be minted, but got ${changes.lpMinted}`);
  }
  if (changes.lpSupplyChange !== changes.lpMinted) {
    throw new Error(`LP supply change (${changes.lpSupplyChange}) should match LP minted (${changes.lpMinted})`);
  }
  
  console.log('✅ Deposit validation passed!');
}

/**
 * Validates that a swap operation worked correctly
 */
export function validateSwap(
  changes: BalanceChanges,
  expectedAmountIn: number,
  isXToY: boolean,
  allowTransferFees: boolean = true
): void {
  console.log('\n✅ VALIDATING SWAP:');
  
  if (isXToY) {
    // User should have spent X tokens
    if (changes.userXSpent !== expectedAmountIn) {
      throw new Error(`Expected user to spend ${expectedAmountIn} X tokens, but spent ${changes.userXSpent}`);
    }
    // User should have received Y tokens
    if (changes.userYReceived <= 0) {
      throw new Error(`Expected user to receive Y tokens, but received ${changes.userYReceived}`);
    }
    // Pool should have gained X and lost Y
    if (allowTransferFees) {
      if (changes.poolXChange < expectedAmountIn - changes.feesCollected.tokenX) {
        throw new Error(`Pool X should have increased by at least ${expectedAmountIn - changes.feesCollected.tokenX}, but changed by ${changes.poolXChange}`);
      }
    } else {
      if (changes.poolXChange !== expectedAmountIn) {
        throw new Error(`Pool X should have increased by ${expectedAmountIn}, but changed by ${changes.poolXChange}`);
      }
    }
    if (changes.poolYChange >= 0) {
      throw new Error(`Pool Y should have decreased, but changed by ${changes.poolYChange}`);
    }
  } else {
    // User should have spent Y tokens
    if (changes.userYSpent !== expectedAmountIn) {
      throw new Error(`Expected user to spend ${expectedAmountIn} Y tokens, but spent ${changes.userYSpent}`);
    }
    // User should have received X tokens
    if (changes.userXReceived <= 0) {
      throw new Error(`Expected user to receive X tokens, but received ${changes.userXReceived}`);
    }
    // Pool should have gained Y and lost X
    if (allowTransferFees) {
      if (changes.poolYChange < expectedAmountIn - changes.feesCollected.tokenY) {
        throw new Error(`Pool Y should have increased by at least ${expectedAmountIn - changes.feesCollected.tokenY}, but changed by ${changes.poolYChange}`);
      }
    } else {
      if (changes.poolYChange !== expectedAmountIn) {
        throw new Error(`Pool Y should have increased by ${expectedAmountIn}, but changed by ${changes.poolYChange}`);
      }
    }
    if (changes.poolXChange >= 0) {
      throw new Error(`Pool X should have decreased, but changed by ${changes.poolXChange}`);
    }
  }
  
  console.log('✅ Swap validation passed!');
}

/**
 * Validates that a withdrawal operation worked correctly
 */
export function validateWithdrawal(
  changes: BalanceChanges,
  expectedLpBurn: number,
  allowTransferFees: boolean = true
): void {
  console.log('\n✅ VALIDATING WITHDRAWAL:');
  
  // User should have burned LP tokens
  if (changes.lpBurned !== expectedLpBurn) {
    throw new Error(`Expected user to burn ${expectedLpBurn} LP tokens, but burned ${changes.lpBurned}`);
  }
  if (changes.lpSupplyChange !== -expectedLpBurn) {
    throw new Error(`LP supply should have decreased by ${expectedLpBurn}, but changed by ${changes.lpSupplyChange}`);
  }
  
  // User should have received tokens
  if (changes.userXReceived <= 0) {
    throw new Error(`Expected user to receive X tokens, but received ${changes.userXReceived}`);
  }
  if (changes.userYReceived <= 0) {
    throw new Error(`Expected user to receive Y tokens, but received ${changes.userYReceived}`);
  }
  
  // Pool should have sent tokens
  if (changes.poolXChange >= 0) {
    throw new Error(`Pool X should have decreased, but changed by ${changes.poolXChange}`);
  }
  if (changes.poolYChange >= 0) {
    throw new Error(`Pool Y should have decreased, but changed by ${changes.poolYChange}`);
  }
  
  // If transfer fees are expected, account for them
  if (allowTransferFees) {
    // The amount user receives might be less than what the pool sent due to transfer fees
    if (changes.userXReceived > Math.abs(changes.poolXChange)) {
      throw new Error(`User received more X tokens (${changes.userXReceived}) than pool sent (${Math.abs(changes.poolXChange)})`);
    }
    if (changes.userYReceived > Math.abs(changes.poolYChange)) {
      throw new Error(`User received more Y tokens (${changes.userYReceived}) than pool sent (${Math.abs(changes.poolYChange)})`);
    }
  } else {
    // Without transfer fees, user should receive exactly what pool sent
    if (changes.userXReceived !== Math.abs(changes.poolXChange)) {
      throw new Error(`User should have received ${Math.abs(changes.poolXChange)} X tokens, but received ${changes.userXReceived}`);
    }
    if (changes.userYReceived !== Math.abs(changes.poolYChange)) {
      throw new Error(`User should have received ${Math.abs(changes.poolYChange)} Y tokens, but received ${changes.userYReceived}`);
    }
  }
  
  console.log('✅ Withdrawal validation passed!');
}