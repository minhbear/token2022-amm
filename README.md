# Token2022 AMM Template

AMM (Automated Market Maker) template for Solana with liquidity pools, token swapping, and full Token2022 compatibility.

## Features

- **Liquidity Pools**: Create and manage token pair pools
- **Token Swapping**: Efficient x*y=k constant product swaps
- **Liquidity Mining**: Add/remove liquidity with LP tokens
- **Universal Token Support**: SPL Token + Token2022 compatibility
- **Fee System**: Configurable trading fees with collection
- **Slippage Protection**: Minimum output guarantees
- **Transfer Fee Handling**: Automatic Token2022 fee calculations

## Quick Start

```bash
# Prerequisites: Install Rust, Solana CLI, Anchor, Node.js
# See: https://solana.com/docs/intro/installation

# Clone and build
git clone https://github.com/minhbear/token2022-amm
cd token2022-amm
yarn install
anchor build

# Test
anchor test

# Deploy
anchor deploy --provider.cluster devnet
```

## Usage

```typescript
// Initialize pool
await program.methods.initializePool(fee, seed)
  .accounts({ poolState, mintX, mintY, /* ... */ })
  .rpc();

// Add liquidity
await program.methods.deposit(amountX, amountY, minLpOut)
  .accounts({ poolState, userTokenX, userTokenY, /* ... */ })
  .rpc();

// Swap tokens
await program.methods.swap(amountIn, minAmountOut, isXToY)
  .accounts({ poolState, userTokenIn, userTokenOut, /* ... */ })
  .rpc();

// Remove liquidity
await program.methods.withdraw(lpAmount, minAmountX, minAmountY)
  .accounts({ poolState, userLpToken, /* ... */ })
  .rpc();
```

## Program Structure

```
programs/token2022-amm/src/
├── instructions/     # Pool operations
├── state/           # Pool state
├── utils/           # Token validation & fees
└── common/          # Errors & events
```

## Security

- Transfer fee calculations
- Token program validation  
- Extension filtering
- Slippage protection
- Comprehensive testing

## Tests

```bash
anchor test
```

Tests cover pool operations, transfer fees, and token compatibility.

## Contributing

Fork, customize, test, and deploy! See examples in `tests/` for usage patterns.

## Resources

- [Solana Docs](https://docs.solana.com/)
- [Anchor Guide](https://anchor-lang.com/docs)
- [Token2022 Guide](https://solana-program.com/docs/token-2022)

## License

MIT License
