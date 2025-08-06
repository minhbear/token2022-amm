import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Token2022Amm } from '../target/types/token2022_amm';
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
} from '@solana/spl-token';
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { assert } from 'chai';

// Import our helpers
import {
  takeSnapshot,
  calculateChanges,
  logSnapshot,
  logChanges,
  validateDeposit,
  validateSwap,
  validateWithdrawal,
} from './helpers/balance-helper';
import {
  setupTestTokens,
  fundUsers,
  getTokenAddress,
  TokenInfo,
  UserTokenAccounts,
} from './helpers/token-helper';

describe('Token Pair Tests - All Combinations', () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Token2022Amm as Program<Token2022Amm>;

  // Test accounts
  let authority: Keypair;
  let user1: Keypair;
  let user2: Keypair;

  // Test parameters
  const fee = 300; // 3% AMM fee
  const decimals = 6;
  const initialLiquidityX = 1000 * 10 ** 6; // 1000 tokens
  const initialLiquidityY = 2000 * 10 ** 6; // 2000 tokens

  before(async () => {
    // Initialize test accounts
    authority = Keypair.generate();
    user1 = Keypair.generate();
    user2 = Keypair.generate();

    // Airdrop SOL to test accounts
    const airdropAmount = 10 * LAMPORTS_PER_SOL;
    await Promise.all([
      provider.connection.requestAirdrop(authority.publicKey, airdropAmount),
      provider.connection.requestAirdrop(user1.publicKey, airdropAmount),
      provider.connection.requestAirdrop(user2.publicKey, airdropAmount),
    ]);

    // Wait for airdrops to confirm
    await new Promise((resolve) => setTimeout(resolve, 2000));

    console.log('🚀 Test environment initialized');
    console.log(`Authority: ${authority.publicKey.toString()}`);
    console.log(`User1: ${user1.publicKey.toString()}`);
    console.log(`User2: ${user2.publicKey.toString()}`);
  });

  /**
   * Helper function to ensure LP token account exists
   */
  async function ensureLpTokenAccount(
    user: PublicKey,
    lpMint: PublicKey,
    payer: Keypair
  ): Promise<void> {
    const lpTokenAccount = getTokenAddress(lpMint, user, TOKEN_PROGRAM_ID);

    const instruction = createAssociatedTokenAccountIdempotentInstruction(
      payer.publicKey,
      lpTokenAccount,
      user,
      lpMint,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const transaction = new Transaction().add(instruction);
    await sendAndConfirmTransaction(provider.connection, transaction, [payer], {
      skipPreflight: true,
    });
  }

  /**
   * Helper function to test a complete AMM flow for any token pair
   */
  function testTokenPair(
    pairName: string,
    getTokenX: () => TokenInfo,
    getTokenY: () => TokenInfo,
    getUserAccounts: () => Map<string, UserTokenAccounts>
  ) {
    describe(`${pairName} Pair Tests`, () => {
      let config: PublicKey;
      let poolState: PublicKey;
      let poolAuthority: PublicKey;
      let lpMint: PublicKey;
      let vaultX: PublicKey;
      let vaultY: PublicKey;

      const seed = new anchor.BN(Math.floor(Math.random() * 1000000));

      it('Should initialize pool', async () => {
        const tokenX = getTokenX();
        const tokenY = getTokenY();

        // Derive PDAs
        [config] = PublicKey.findProgramAddressSync(
          [Buffer.from('config'), seed.toArrayLike(Buffer, 'le', 8)],
          program.programId
        );

        [poolState] = PublicKey.findProgramAddressSync(
          [Buffer.from('pool'), config.toBuffer()],
          program.programId
        );

        [poolAuthority] = PublicKey.findProgramAddressSync(
          [Buffer.from('auth'), config.toBuffer()],
          program.programId
        );

        [lpMint] = PublicKey.findProgramAddressSync(
          [Buffer.from('lp_mint'), config.toBuffer()],
          program.programId
        );

        // Get vault addresses using correct token programs
        vaultX = getTokenAddress(
          tokenX.mint,
          poolAuthority,
          tokenX.tokenProgram,
          true
        );
        vaultY = getTokenAddress(
          tokenY.mint,
          poolAuthority,
          tokenY.tokenProgram,
          true
        );

        console.log(`\n🏊 Initializing ${pairName} pool...`);
        console.log(`Seed: ${seed.toString()}`);
        console.log(`Config: ${config.toString()}`);
        console.log(`Pool Authority: ${poolAuthority.toString()}`);
        console.log(`Vault X: ${vaultX.toString()}`);
        console.log(`Vault Y: ${vaultY.toString()}`);

        // Initialize pool with dual token programs
        const tx = await program.methods
          .initializePool(seed, fee, null)
          .accountsPartial({
            authority: authority.publicKey,
            config,
            poolState,
            mintX: tokenX.mint,
            mintY: tokenY.mint,
            lpMint,
            poolAuthority,
            vaultX,
            vaultY,
            tokenProgramX: tokenX.tokenProgram,
            tokenProgramY: tokenY.tokenProgram,
            tokenProgramLp: TOKEN_PROGRAM_ID, // Use legacy token for LP tokens
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc();

        console.log(`✅ Pool initialized: ${tx}`);

        // Verify pool configuration
        const configData = await program.account.config.fetch(config);
        assert.equal(configData.mintX.toString(), tokenX.mint.toString());
        assert.equal(configData.mintY.toString(), tokenY.mint.toString());
        assert.equal(configData.fee, fee);
      });

      it('Should deposit initial liquidity', async () => {
        const tokenX = getTokenX();
        const tokenY = getTokenY();
        const userAccounts = getUserAccounts();
        const user1Accounts = userAccounts.get(user1.publicKey.toString())!;

        // Create LP token account for user1
        const user1LpToken = getTokenAddress(
          lpMint,
          user1.publicKey,
          TOKEN_PROGRAM_ID
        );

        // Ensure LP token account exists before taking snapshot
        await ensureLpTokenAccount(user1.publicKey, lpMint, authority);

        const amountX = new anchor.BN(initialLiquidityX);
        const amountY = new anchor.BN(initialLiquidityY);
        const minLpOut = new anchor.BN(1);

        // Take snapshot before deposit
        const before = await takeSnapshot(
          provider.connection,
          program,
          user1Accounts.tokenX,
          user1Accounts.tokenY,
          user1LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('BEFORE DEPOSIT', before);

        // Execute deposit
        const tx = await program.methods
          .deposit(amountX, amountY, minLpOut)
          .accountsPartial({
            user: user1.publicKey,
            config,
            poolState,
            poolAuthority,
            mintX: tokenX.mint,
            mintY: tokenY.mint,
            vaultX,
            vaultY,
            userTokenX: user1Accounts.tokenX,
            userTokenY: user1Accounts.tokenY,
            lpMint,
            userLpToken: user1LpToken,
            tokenProgramX: tokenX.tokenProgram,
            tokenProgramY: tokenY.tokenProgram,
            tokenProgramLp: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();

        console.log(`✅ Deposit completed: ${tx}`);

        // Take snapshot after deposit
        const after = await takeSnapshot(
          provider.connection,
          program,
          user1Accounts.tokenX,
          user1Accounts.tokenY,
          user1LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('AFTER DEPOSIT', after);

        // Calculate and log changes
        const changes = calculateChanges(before, after);
        logChanges(changes);

        // Validate the deposit
        const hasTransferFees = tokenX.hasTransferFee || tokenY.hasTransferFee;
        validateDeposit(
          changes,
          initialLiquidityX,
          initialLiquidityY,
          hasTransferFees
        );
      });

      it('Should perform swap X -> Y', async () => {
        const tokenX = getTokenX();
        const tokenY = getTokenY();
        const userAccounts = getUserAccounts();
        const user2Accounts = userAccounts.get(user2.publicKey.toString())!;
        const user2LpToken = getTokenAddress(
          lpMint,
          user2.publicKey,
          TOKEN_PROGRAM_ID
        );

        // Ensure LP token account exists for user2 before taking snapshot
        await ensureLpTokenAccount(user2.publicKey, lpMint, authority);

        const amountIn = new anchor.BN(10 * 10 ** 6); // 10 tokens
        const minAmountOut = new anchor.BN(1);

        // Take snapshot before swap
        const before = await takeSnapshot(
          provider.connection,
          program,
          user2Accounts.tokenX,
          user2Accounts.tokenY,
          user2LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('BEFORE SWAP X->Y', before);

        // Execute swap
        const tx = await program.methods
          .swap(amountIn, minAmountOut)
          .accountsPartial({
            user: user2.publicKey,
            config,
            poolState,
            poolAuthority,
            mintIn: tokenX.mint,
            mintOut: tokenY.mint,
            vaultIn: vaultX,
            vaultOut: vaultY,
            userTokenIn: user2Accounts.tokenX,
            userTokenOut: user2Accounts.tokenY,
            tokenProgramX: tokenX.tokenProgram,
            tokenProgramY: tokenY.tokenProgram,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user2])
          .rpc();

        console.log(`✅ Swap X->Y completed: ${tx}`);

        // Take snapshot after swap
        const after = await takeSnapshot(
          provider.connection,
          program,
          user2Accounts.tokenX,
          user2Accounts.tokenY,
          user2LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('AFTER SWAP X->Y', after);

        // Calculate and log changes
        const changes = calculateChanges(before, after);
        logChanges(changes);

        // Validate the swap
        const hasTransferFees = tokenX.hasTransferFee || tokenY.hasTransferFee;
        validateSwap(changes, amountIn.toNumber(), true, hasTransferFees);
      });

      it('Should perform swap Y -> X', async () => {
        const tokenX = getTokenX();
        const tokenY = getTokenY();
        const userAccounts = getUserAccounts();
        const user2Accounts = userAccounts.get(user2.publicKey.toString())!;
        const user2LpToken = getTokenAddress(
          lpMint,
          user2.publicKey,
          TOKEN_PROGRAM_ID
        );

        // Ensure LP token account exists for user2 before taking snapshot
        await ensureLpTokenAccount(user2.publicKey, lpMint, authority);

        const amountIn = new anchor.BN(20 * 10 ** 6); // 20 tokens
        const minAmountOut = new anchor.BN(1);

        // Take snapshot before swap
        const before = await takeSnapshot(
          provider.connection,
          program,
          user2Accounts.tokenX,
          user2Accounts.tokenY,
          user2LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('BEFORE SWAP Y->X', before);

        // Execute swap
        const tx = await program.methods
          .swap(amountIn, minAmountOut)
          .accountsPartial({
            user: user2.publicKey,
            config,
            poolState,
            poolAuthority,
            mintIn: tokenY.mint,
            mintOut: tokenX.mint,
            vaultIn: vaultY,
            vaultOut: vaultX,
            userTokenIn: user2Accounts.tokenY,
            userTokenOut: user2Accounts.tokenX,
            tokenProgramX: tokenX.tokenProgram,
            tokenProgramY: tokenY.tokenProgram,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user2])
          .rpc();

        console.log(`✅ Swap Y->X completed: ${tx}`);

        // Take snapshot after swap
        const after = await takeSnapshot(
          provider.connection,
          program,
          user2Accounts.tokenX,
          user2Accounts.tokenY,
          user2LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('AFTER SWAP Y->X', after);

        // Calculate and log changes
        const changes = calculateChanges(before, after);
        logChanges(changes);

        // Validate the swap
        const hasTransferFees = tokenX.hasTransferFee || tokenY.hasTransferFee;
        validateSwap(changes, amountIn.toNumber(), false, hasTransferFees);
      });

      it('Should withdraw liquidity', async () => {
        const tokenX = getTokenX();
        const tokenY = getTokenY();
        const userAccounts = getUserAccounts();
        const user1Accounts = userAccounts.get(user1.publicKey.toString())!;
        const user1LpToken = getTokenAddress(
          lpMint,
          user1.publicKey,
          TOKEN_PROGRAM_ID
        );

        // Take snapshot before withdrawal to get current LP balance
        const currentSnapshot = await takeSnapshot(
          provider.connection,
          program,
          user1Accounts.tokenX,
          user1Accounts.tokenY,
          user1LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        const lpAmount = new anchor.BN(
          Number(currentSnapshot.user.lpToken.balance) / 4
        ); // Withdraw 25%
        const minAmountX = new anchor.BN(1);
        const minAmountY = new anchor.BN(1);

        logSnapshot('BEFORE WITHDRAWAL', currentSnapshot);

        // Execute withdrawal
        const tx = await program.methods
          .withdraw(lpAmount, minAmountX, minAmountY)
          .accountsPartial({
            user: user1.publicKey,
            config,
            poolState,
            poolAuthority,
            mintX: tokenX.mint,
            mintY: tokenY.mint,
            vaultX,
            vaultY,
            userTokenX: user1Accounts.tokenX,
            userTokenY: user1Accounts.tokenY,
            lpMint,
            userLpToken: user1LpToken,
            tokenProgramX: tokenX.tokenProgram,
            tokenProgramY: tokenY.tokenProgram,
            tokenProgramLp: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();

        console.log(`✅ Withdrawal completed: ${tx}`);

        // Take snapshot after withdrawal
        const after = await takeSnapshot(
          provider.connection,
          program,
          user1Accounts.tokenX,
          user1Accounts.tokenY,
          user1LpToken,
          poolState,
          tokenX.tokenProgram,
          tokenY.tokenProgram,
          TOKEN_PROGRAM_ID
        );

        logSnapshot('AFTER WITHDRAWAL', after);

        // Calculate and log changes
        const changes = calculateChanges(currentSnapshot, after);
        logChanges(changes);

        // Validate the withdrawal
        const hasTransferFees = tokenX.hasTransferFee || tokenY.hasTransferFee;
        validateWithdrawal(changes, lpAmount.toNumber(), hasTransferFees);
      });
    });
  }

  // Test 1: Legacy SPL + Legacy SPL
  describe('🔄 LEGACY + LEGACY PAIR', () => {
    let tokenX: TokenInfo;
    let tokenY: TokenInfo;
    let userAccounts: Map<string, UserTokenAccounts>;

    before(async () => {
      const setup = await setupTestTokens(
        provider.connection,
        authority,
        [user1.publicKey, user2.publicKey],
        'legacy',
        'legacy'
      );

      tokenX = setup.tokenX;
      tokenY = setup.tokenY;
      userAccounts = setup.userAccounts;

      await fundUsers(
        provider.connection,
        authority,
        tokenX,
        tokenY,
        userAccounts
      );
    });

    testTokenPair(
      'Legacy + Legacy',
      () => tokenX,
      () => tokenY,
      () => userAccounts
    );
  });

  // Test 2: Token-2022 + Legacy SPL
  describe('🔄 TOKEN-2022 + LEGACY PAIR', () => {
    let tokenX: TokenInfo;
    let tokenY: TokenInfo;
    let userAccounts: Map<string, UserTokenAccounts>;

    before(async () => {
      const setup = await setupTestTokens(
        provider.connection,
        authority,
        [user1.publicKey, user2.publicKey],
        'token2022',
        'legacy'
      );

      tokenX = setup.tokenX;
      tokenY = setup.tokenY;
      userAccounts = setup.userAccounts;

      await fundUsers(
        provider.connection,
        authority,
        tokenX,
        tokenY,
        userAccounts
      );
    });

    testTokenPair(
      'Token-2022 + Legacy',
      () => tokenX,
      () => tokenY,
      () => userAccounts
    );
  });

  // Test 3: Token-2022 + Token-2022
  describe('🔄 TOKEN-2022 + TOKEN-2022 PAIR', () => {
    let tokenX: TokenInfo;
    let tokenY: TokenInfo;
    let userAccounts: Map<string, UserTokenAccounts>;

    before(async () => {
      const setup = await setupTestTokens(
        provider.connection,
        authority,
        [user1.publicKey, user2.publicKey],
        'token2022',
        'token2022'
      );

      tokenX = setup.tokenX;
      tokenY = setup.tokenY;
      userAccounts = setup.userAccounts;

      await fundUsers(
        provider.connection,
        authority,
        tokenX,
        tokenY,
        userAccounts
      );
    });

    testTokenPair(
      'Token-2022 + Token-2022',
      () => tokenX,
      () => tokenY,
      () => userAccounts
    );
  });

  after(() => {
    console.log('\n🎉 ALL TOKEN PAIR TESTS COMPLETED!');
    console.log('✅ Legacy + Legacy pair: TESTED');
    console.log('✅ Token-2022 + Legacy pair: TESTED');
    console.log('✅ Token-2022 + Token-2022 pair: TESTED');
    console.log('\n🔍 Key Features Validated:');
    console.log('  • Dual token program support');
    console.log('  • Transfer fee handling');
    console.log('  • Complete AMM functionality');
    console.log('  • Balance tracking and validation');
  });
});
