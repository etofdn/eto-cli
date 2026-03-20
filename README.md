# eto

Command-line interface for the [ETO Universal VM](https://github.com/etofdn/etovm).

## Install

**Mac (Apple Silicon):**
```bash
curl -sL https://github.com/etofdn/eto-cli/releases/latest/download/eto-macos-arm64.tar.gz | tar xz
sudo mv eto /usr/local/bin/
```

**Mac (Intel):**
```bash
curl -sL https://github.com/etofdn/eto-cli/releases/latest/download/eto-macos-x86_64.tar.gz | tar xz
sudo mv eto /usr/local/bin/
```

**Linux:**
```bash
curl -sL https://github.com/etofdn/eto-cli/releases/latest/download/eto-linux-x86_64.tar.gz | tar xz
sudo mv eto /usr/local/bin/
```

**From source:**
```bash
git clone https://github.com/etofdn/eto-cli.git
cd eto-cli
cargo install --path .
```

## Quick Start

```bash
# Connect to testnet
eto config set --url http://98.93.46.219:8899

# Get testnet tokens
eto airdrop 10

# Check balance
eto balance

# Transfer
eto transfer <ADDRESS> 1.5

# Deploy contracts
eto deploy evm <BYTECODE_HEX>
eto deploy wasm contract.wasm
eto deploy move module.mv

# ZK operations
eto zk add
eto zk mul

# Cluster info
eto cluster-info
```

## Commands

| Command | Description |
|---------|-------------|
| `eto config get` | Show current configuration |
| `eto config set --url <URL>` | Set RPC endpoint |
| `eto balance [ADDRESS]` | Show account balance |
| `eto airdrop <AMOUNT> [ADDRESS]` | Request testnet tokens |
| `eto transfer <TO> <AMOUNT>` | Transfer tokens |
| `eto deploy evm <HEX>` | Deploy Solidity contract |
| `eto deploy wasm <FILE>` | Deploy WASM contract |
| `eto deploy move <FILE>` | Deploy Move module |
| `eto zk add` | BN254 EC point addition |
| `eto zk mul` | BN254 scalar multiplication |
| `eto account <ADDRESS>` | Show account details |
| `eto block-height` | Current block height |
| `eto transaction-count` | Total transactions |
| `eto cluster-info` | Cluster status |
| `eto address` | Show default keypair address |

## Testnet

| | |
|-|-|
| RPC (Zone A) | `http://98.93.46.219:8899` |
| RPC (Zone B) | `http://100.54.242.199:8899` |
| RPC (Zone C) | `http://35.175.111.73:8899` |
| Explorer | http://13.222.157.139 |
| Chain ID | `0x454F` (17743) |

## License

MIT
