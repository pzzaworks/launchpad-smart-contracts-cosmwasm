# Launchpad Smart Contracts (CosmWasm)

This repository contains CosmWasm smart contracts for a token launchpad platform, developed by Berke (pzzaworks), including:

- Token creation and management (CW20)
- Token sale and distribution
- Vesting contracts
- Staking mechanisms
- Token faucet

## Project Structure

The repository is organized into two main sections:

- `cw20/`: Smart contracts related to CW20 token functionality
  - Token creation and management
  - Staking
  - Vesting
  - Faucet

- `denom/`: Smart contracts for native token management
  - Sale
  - Vesting

## Requirements

- Rust 1.69+
- Node.js (see `.nvmrc` for version)
- Bun or Yarn

## Development Setup

1. Clone the repository
2. Set up environment variables (copy `.env-example` to `.env` and fill in values)
3. Install dependencies:

```bash
# For Rust contracts
cargo build

# For scripts
cd cw20 # or denom
bun install # or yarn install
```

## Testing

Each contract contains its own tests that can be run with:

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 