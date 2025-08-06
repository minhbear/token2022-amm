# Token2022 AMM Template

A production-ready template for building decentralized Automated Market Makers (AMM) on Solana that supports both legacy SPL tokens and Token2022 with extensions. This template provides a solid foundation for creating liquidity pools with comprehensive transfer fee handling and Token2022 extension compatibility.

## Why This Template?

This template addresses the complexities of building AMMs that work with both traditional SPL tokens and the Token Extensions Program (Token-2022). Most existing AMM examples don't properly handle Token Extensions features like transfer fees, which can lead to incorrect pricing and broken functionality. This template provides:

- **Battle-tested architecture** with proper Token2022 integration
- **Production-ready security** with comprehensive validation
- **Transfer fee support** that many AMMs overlook
- **Extension compatibility** that future-proofs your implementation
- **Clean, documented code** that's easy to understand and modify

## Features

- **Dual Token Support**: Compatible with both legacy SPL tokens and Token Extensions Program
- **Transfer Fee Integration**: Accurate pricing calculations accounting for Token Extensions transfer fees  
- **Extension Support**: Supports safe Token Extensions including transfer fees, interest-bearing tokens, and metadata
- **Security First**: Comprehensive validation and filtering of unsupported extensions
- **Proportional Liquidity**: Standard x*y=k constant product formula with fee adjustments
- **Slippage Protection**: User-defined minimum output amounts and slippage tolerance

## Supported Token Extensions

The template supports the **Token Extensions Program** (also known as Token-2022) alongside legacy SPL tokens.

### Allowed Extensions
- **Transfer Fee Config**: Automatic fee deduction on transfers with proper calculation
- **Interest Bearing Config**: Tokens that accrue interest over time  
- **Metadata & Metadata Pointer**: On-chain metadata storage and pointers
- **Group & Group Member**: Token grouping functionality for collections
- **Immutable Owner**: Enhanced security for token accounts
- **CPI Guard**: Protection against unwanted CPI calls

### Prohibited Extensions (Security & Compatibility)
- **Mint Close Authority**: Prevents pool disruption from mint closure
- **Default Account State**: Incompatible with AMM trading mechanics
- **Non-Transferable**: Conflicts with core trading functionality
- **Permanent Delegate**: Security risk for automated pool operations
- **Transfer Hook**: May interfere with AMM transfer logic

## Installation

### Prerequisites

Install Rust, Solana CLI, Anchor, and Node.js. See the [official installation guide](https://solana.com/vi/docs/intro/installation).

### Build

```bash
# Fork and clone this repository
git clone https://github.com/minhbear/token2022-amm
cd token2022-amm

# Install dependencies
yarn install

# Build the program
anchor build

```

### Test

```bash
# Run tests
anchor test
```

### Deploy

```bash
# Deploy to devnet
anchor deploy --provider.cluster devnet

# Deploy to mainnet-beta
anchor deploy --provider.cluster mainnet-beta
```

## Usage

### Initialize Pool

Create a new liquidity pool between two tokens:

```typescript
const tx = await program.methods
  .initializePool(seed, fee, whitelistLp)
  .accounts({
    authority: wallet.publicKey,
    config: configPda,
    poolState: poolStatePda,
    mintX: tokenXMint,
    mintY: tokenYMint,
    lpMint: lpMintPda,
    poolAuthority: poolAuthorityPda,
    vaultX: vaultXPda,
    vaultY: vaultYPda,
    tokenProgramX: getTokenProgram(tokenXMint),
    tokenProgramY: getTokenProgram(tokenYMint),
    tokenProgramLp: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Deposit Liquidity

Add liquidity to an existing pool:

```typescript
const tx = await program.methods
  .deposit(amountX, amountY, minLpOut)
  .accounts({
    user: wallet.publicKey,
    config: configPda,
    poolState: poolStatePda,
    poolAuthority: poolAuthorityPda,
    mintX: tokenXMint,
    mintY: tokenYMint,
    vaultX: vaultXPda,
    vaultY: vaultYPda,
    userTokenX: userTokenXAccount,
    userTokenY: userTokenYAccount,
    lpMint: lpMintPda,
    userLpToken: userLpTokenAccount,
    tokenProgramX: getTokenProgram(tokenXMint),
    tokenProgramY: getTokenProgram(tokenYMint),
    tokenProgramLp: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Swap Tokens

Exchange one token for another:

```typescript
const tx = await program.methods
  .swap(amountIn, minAmountOut)
  .accounts({
    user: wallet.publicKey,
    config: configPda,
    poolState: poolStatePda,
    poolAuthority: poolAuthorityPda,
    mintIn: inputTokenMint,
    mintOut: outputTokenMint,
    vaultIn: inputVaultPda,
    vaultOut: outputVaultPda,
    userTokenIn: userInputTokenAccount,
    userTokenOut: userOutputTokenAccount,
    tokenProgramX: getTokenProgram(tokenXMint),
    tokenProgramY: getTokenProgram(tokenYMint),
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Withdraw Liquidity

Remove liquidity from a pool:

```typescript
const tx = await program.methods
  .withdraw(lpAmount, minAmountX, minAmountY)
  .accounts({
    user: wallet.publicKey,
    config: configPda,
    poolState: poolStatePda,
    poolAuthority: poolAuthorityPda,
    mintX: tokenXMint,
    mintY: tokenYMint,
    vaultX: vaultXPda,
    vaultY: vaultYPda,
    userTokenX: userTokenXAccount,
    userTokenY: userTokenYAccount,
    lpMint: lpMintPda,
    userLpToken: userLpTokenAccount,
    tokenProgramX: getTokenProgram(tokenXMint),
    tokenProgramY: getTokenProgram(tokenYMint),
    tokenProgramLp: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

## Architecture

### Program Structure

```
programs/token2022-amm/src/
├── lib.rs                 # Program entry point
├── common/
│   ├── constant.rs        # Program constants
│   ├── error.rs          # Custom error types
│   ├── event.rs          # Program events
│   └── mod.rs            # Module exports
├── instructions/
│   ├── init_pool.rs      # Pool initialization
│   ├── deposit.rs        # Liquidity deposits
│   ├── withdraw.rs       # Liquidity withdrawals
│   ├── swap.rs           # Token swapping
│   └── mod.rs            # Instruction exports
├── state/
│   └── mod.rs            # Program state definitions
└── utils/
    ├── token.rs          # Token utility functions
    └── mod.rs            # Utility exports
```

### Key Components

- **Config**: Pool configuration and authority information
- **PoolState**: Current pool reserves and LP token supply
- **Transfer Fee Handling**: Automatic detection and calculation of Token2022 transfer fees
- **Extension Validation**: Safety checks for Token2022 extensions

## Security Considerations

### Transfer Fees

When using tokens with transfer fees, be aware that:
- Actual amounts received may be less than requested due to fees
- Slippage calculations should account for transfer fees
- Pool reserves track post-fee amounts

### Extension Validation

The program automatically validates Token2022 extensions and rejects tokens with:
- Non-transferable restrictions
- Transfer hooks (for security)
- Permanent delegates
- Other potentially unsafe extensions

### Slippage Protection

Always set appropriate slippage tolerances:
- For swaps: Use `minAmountOut` parameter
- For deposits: Use `minLpOut` parameter  
- For withdrawals: Use `minAmountX` and `minAmountY` parameters

## Configuration

### Pool Parameters

- **Fee**: Trading fee in basis points (max 1000 = 10%)
- **Seed**: Unique identifier for pool derivation  
- **Whitelist**: Optional whitelist for LP providers (useful for private/restricted pools)

### Supported Networks

- **Localnet**: For development and testing
- **Devnet**: For staging and integration testing
- **Mainnet Beta**: For production deployment



### Token Program Compatibility

| Token Type | Support Level | Notes |
|------------|---------------|-------|
| Legacy SPL | ✅ Full | All standard operations |
| Token2022 | ✅ Full | With extension filtering |
| Transfer Fees | ✅ Full | Automatic calculation |
| Interest Bearing | ✅ Supported | Current rate used |
| Confidential | ⚠️ Partial | Non-confidential ops only |
| Non-Transferable | ❌ Blocked | Security restriction |
| Transfer Hooks | ❌ Blocked | Security restriction |

## Fork and Customize

This template is designed to be forked and customized for your specific AMM needs:

### Getting Started with Your Fork

1. **Fork this repository** to your GitHub account
2. **Clone your fork** locally
3. **Update identifiers**:
   ```bash
   # Generate new program keypair
   anchor keys list
   
   # Update lib.rs with your program ID
   # Update Anchor.toml with your program ID
   ```
4. **Customize features** as needed for your use case
5. **Deploy** to your preferred network

### Template Structure

- **Core AMM Logic**: Standard x*y=k constant product formula
- **Token2022 Integration**: Comprehensive extension support
- **Transfer Fee Handling**: Automatic fee detection and calculation  
- **Security Features**: Extensive validation and error handling
- **Test Suite**: Complete test coverage for all scenarios

### Customization Ideas

- **Different Curve Types**: Implement stable swaps, concentrated liquidity
- **Advanced Features**: Add flash loans, limit orders, or governance
- **Fee Structures**: Implement dynamic fees or protocol revenue sharing
- **UI Integration**: Build frontend with proper Token2022 support
- **Analytics**: Add detailed event logging for analytics platforms

## Contributing to Template

We welcome contributions to improve this template:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/template-improvement`)
3. Make your changes (focus on template improvements, not specific use cases)
4. Add tests and documentation
5. Submit a pull request

### Development Guidelines

- **Template Focus**: Keep changes generic and useful for all forks
- **Documentation**: Update README for any new template features
- **Testing**: Ensure all tests pass and add tests for new functionality
- **Security**: Any security improvements are especially welcome

## Testing

The test suite covers:
- Pool initialization with various token types
- Liquidity provision and removal
- Token swapping with different fee scenarios
- Transfer fee calculations
- Extension validation

Run specific test suites:

```bash
# Test pool initialization
anchor test --skip-lint --test-suite initialize-pool

# Test trading operations  
anchor test --skip-lint --test-suite trading

# Test with transfer fees
anchor test --skip-lint --test-suite transfer-fees
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This software is provided "as is" and should be used at your own risk. Always conduct thorough testing and audits before deploying to mainnet. The authors are not responsible for any losses incurred through the use of this software.

## Template Resources

### Documentation
- [Official Solana Docs](https://docs.solana.com/)
- [Anchor Framework Guide](https://anchor-lang.com/docs/installation)
- [Token Extensions Complete Guide](https://solana-program.com/docs/token-2022)
- [Token Extensions On-chain Development](https://solana-program.com/docs/token-2022/onchain)
- [SPL Token Documentation](https://solana-program.com/docs/token)

### Development Tools
- [Solana Playground](https://beta.solpg.io/) - Online IDE
- [Anchor by Example](https://examples.anchor-lang.com/) - Code examples
- [Solana Cookbook](https://solanacookbook.com/) - Developer recipes
