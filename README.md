# eto

Command-line interface for the [ETO Universal VM](https://github.com/etofdn/etovm) — a 5-VM blockchain doing 3.6M TPS across 3 consensus zones with GPU acceleration.

One chain. One state. Five VMs: **SVM · EVM · WASM · Move · ZK**

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
# 1. Connect to testnet
eto config set --url http://98.93.46.219:8899

# 2. Get testnet tokens
eto airdrop 1000000000

# 3. Check balance
eto balance

# 4. Deploy a Solidity contract
eto deploy evm 608060405234801561001057600080fd5b50...

# 5. Inspect it on-chain
eto inspect account <CONTRACT_ADDRESS>
```

---

## Configuration

ETO CLI stores config at `~/.config/eto/config.json`. Set it once, never pass `--url` again.

```bash
# Set RPC endpoint
eto config set --url http://98.93.46.219:8899

# View current config
eto config get
# Config File: /home/user/.config/eto/config.json
# RPC URL: http://98.93.46.219:8899
```

**Override per-command:**
```bash
eto balance -u http://100.54.242.199:8899   # Use Zone B
eto balance --url http://35.175.111.73:8899  # Use Zone C
```

**Environment variable:**
```bash
export ETO_RPC_URL=http://98.93.46.219:8899
eto balance  # uses env var
```

**Priority:** `-u` flag > `ETO_RPC_URL` env > config file > default (127.0.0.1:8899)

### Testnet RPC Endpoints

| Zone | RPC | Location |
|------|-----|----------|
| Zone A | `http://98.93.46.219:8899` | us-east-1a |
| Zone B | `http://100.54.242.199:8899` | us-east-1b |
| Zone C | `http://35.175.111.73:8899` | us-east-1c |

Each zone runs independent consensus with its own GPU sequencer + 3 validators. Combined throughput: ~3.6M TPS.

---

## Keypair Management

ETO uses Ed25519 keypairs (same as Solana). Keys are stored at `~/.config/eto/keys/`.

### Generate a new keypair

```bash
eto keygen
# Wrote new keypair to /home/user/.config/eto/keys/DJ7v3EM2.json
# DJ7v3EM2WXyQoZ2g2PpvDKUAt1aupFpjqXhTQeUjuM6g

# Save to specific file
eto keygen ~/mykey.json
```

### List all keypairs

```bash
eto keypair list
#   DJ7v3EM2WXyQoZ2g2PpvDKUAt1aupFpjqXhTQeUjuM6g (DJ7v3EM2.json)
#   9A73VFbasfQKdba7y2dhmNejULZyEF2KWY7a3MtHrNmo (9A73VFba.json)
```

### Set default keypair

```bash
eto keypair set ~/mykey.json
# Default keypair: DJ7v3EM2WXyQoZ2g2PpvDKUAt1aupFpjqXhTQeUjuM6g
```

All signing commands (`transfer`, `deploy`, `create-account`) use the active keypair. Falls back to the built-in faucet keypair if none is set.

### Import a private key

```bash
# From hex (32 bytes)
eto keypair import aabbccdd00112233445566778899aabbccddeeff00112233445566778899aabb

# From base58
eto keypair import 5KfbFUyREnMqR5FjU5QqaVvGewz1f7MTyqY8oqm4Et2z
```

### Show your address

```bash
eto address
# 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
```

---

## Accounts & Balances

### Check balance

```bash
# Default keypair balance
eto balance
# 999873742673855 lamports

# Specific address (base58)
eto balance 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
# 999873742673855 lamports

# EVM hex address
eto balance 0x0000000000000000000000000000000000000001
# 2000000002 lamports
```

### Full account info

```bash
eto account 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
# Public Key: 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
# Balance: 999873742673855 lamports
# Owner: 11111111111111111111111111111111
# Executable: false
# Rent Epoch: 0
# Data Length: 0
```

### Create a new account

```bash
# Create account with 128 bytes of data space, funded with 1 SOL
eto create-account 128 1000000000
# Signature: 4fnUnLW...
# Account: CziCKz8TQzdgScxV7Qr7qiBJt3KPenaioEKw7FfJoc7V
```

---

## Transfers

### Send tokens

```bash
# Transfer 1 SOL (amounts with decimals = SOL, integers = lamports)
eto transfer 11111111111111111111111111111117 1.0
# Signature: 5KfbFUy...

# Transfer 500,000,000 lamports
eto transfer 11111111111111111111111111111117 500000000
# Signature: 217Vgk...
```

### Airdrop (faucet)

```bash
# Airdrop 1 SOL to an address
eto airdrop 1000000000 11111111111111111111111111111116
# Requesting airdrop of 1 SOL to 11111111111111111111111111111116...
# Signature: G7aLH8...
# 1000000002 lamports

# Airdrop to yourself
eto airdrop 1000000000
```

---

## Deploy Contracts

ETO supports deploying contracts on 3 VMs. Each contract becomes an on-chain account owned by its VM's program.

### EVM (Solidity)

Deploy Solidity bytecode directly. The contract gets a unique on-chain address.

```bash
# Compile with solc
solc --bin MyContract.sol -o build/

# Deploy
eto deploy evm $(cat build/MyContract.bin)
# Signature: 4pndbK...
# Program deployed to: EKmsCVcgGHxtdKxrZdRSm7WgDNU5p61MLrMnBh2ZGce3

# Verify on-chain
eto inspect account EKmsCVcgGHxtdKxrZdRSm7WgDNU5p61MLrMnBh2ZGce3
# Balance:  1000000000 lamports (1 SOL)
# Owner:    JEKNVnkbo3...xEy  (EVM Program ID)
# Data:     157 bytes            (Solidity bytecode)
```

**SimpleStorage example (deploy + call):**
```bash
# Deploy SimpleStorage (get/set uint256)
eto deploy evm 608060405234801561001057600080fd5b5060e68061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c806360fe47b11460375780636d4ce63c146049575b600080fd5b60476042366004606157005b600055565b604f6061565b60405190815260200160405180910390f35b60005490565b600060208284031215607257600080fd5b503591905056
```

**How it works under the hood:**
1. CLI creates a `SystemProgram::CreateAccount` transaction
2. Account owner is set to `EVM_PROGRAM_ID` (`0xFF...EE`)
3. Bytecode is stored as the account's data
4. To call the contract, send an instruction to `EVM_PROGRAM_ID` with calldata
5. Execution happens via revm v19 (Cancun spec, EIP-4844 compatible)

### WASM (WebAssembly / CosmWasm)

Deploy compiled WASM modules. Executed via wasmtime v29 with Cranelift JIT.

```bash
# Compile your contract to WASM
cargo build --target wasm32-unknown-unknown --release

# Deploy
eto deploy wasm target/wasm32-unknown-unknown/release/my_contract.wasm
# Signature: 56zEpF...
# Program deployed to: 7KWe7GfvsNRXaJY8vivcvDyEpptQhs8oRZYQQxu1cTej

# Verify on-chain
eto inspect account 7KWe7GfvsNRXaJY8vivcvDyEpptQhs8oRZYQQxu1cTej
# Owner:    JEKNVnkbo3...xAv  (WASM Program ID)
# Data:     34 bytes             (WASM bytecode)
```

**Specs:**
- Max module size: 512 KB
- Linear memory: 16 MB (256 pages)
- Fuel metering for compute budget
- CosmWasm compatible (`instantiate`, `execute`, `query`)

### Move

Deploy Move bytecode modules. Uses the upstream move-vm-runtime.

```bash
# Compile Move module
move build

# Deploy
eto deploy move build/modules/my_module.mv
# Signature: 3cVbr5...
# Program deployed to: J3thJCiMCrq1jjZNg5pinN8aAahVMKuisg5y7RnkrZSr

# Verify on-chain
eto inspect account J3thJCiMCrq1jjZNg5pinN8aAahVMKuisg5y7RnkrZSr
# Owner:    JEKNVnkbo3...xAu  (Move Program ID)
# Data:     61 bytes             (Move bytecode)
```

**Specs:**
- Bytecode verified on deployment via `move_bytecode_verifier`
- Module storage: `SHA256("move:module:" || address || module_name)`
- Resource storage: `SHA256("move:resource:" || address || struct_tag)`
- BCS serialization for arguments and return values

---

## ZK Operations

ETO has native ZK precompiles for BN254 (alt_bn128) elliptic curve operations and Groth16 proof verification. These run on-chain via arkworks.

### BN254 EC Point Addition

```bash
eto zk add
# Signature: 2wwS7T...
```

Computes G1 + G1 = 2*G1 on the BN254 curve. EIP-196 compatible.

### BN254 Scalar Multiplication

```bash
eto zk mul
# Signature: 3d6PAf...
```

Computes 7 * G1 on BN254. EIP-196 compatible.

### Available ZK Operations

| Operation | Opcode | Compute Units | Description |
|-----------|--------|---------------|-------------|
| EC Add | 0 | 500 | BN254 point addition |
| EC Mul | 1 | 40,000 | BN254 scalar multiplication |
| Pairing | 2 | 100K + 80K/pair | BN254 pairing check (EIP-197) |
| Groth16 BN254 | verify | 200,000 | Groth16 proof verification |
| Groth16 BLS12-381 | verify | 300,000 | Groth16 proof verification |

---

## Inspect & Verify

The `inspect` command lets you trace transactions end-to-end and verify state changes on-chain.

### Inspect a transfer (full lifecycle trace)

```bash
eto inspect transfer 11111111111111111111111111111117 1000000000
```

Output:
```
PRE-STATE
  Block Height:  562722
  Sender:        6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6 (999874949040603 lamports)
  Recipient:     11111111111111111111111111111117 (0 lamports)

TRANSACTION
  Operation:     Transfer
  Amount:        1000000000 lamports (1 SOL)
  Program:       System (11111111111111111111111111111111)
  Signature:     5KfbFUyREnMqR5FjU5QqaVvGewz1f7MTyqY8oqm4Et2z...
  Status:        ACCEPTED

CONSENSUS
  Waiting for block inclusion confirmed
  Block Height:  562726

POST-STATE
  Sender:        999874949040603 lamports -> 999873948268970 lamports (delta: -1000771633)
  Recipient:     0 lamports -> 1000000000 lamports (delta: +1000000000)

VERIFICATION
  Recipient credited: 1000000000 lamports  EXACT MATCH
  Signature:     5KfbFUyREnMqR5FjU5QqaVvGewz1f7MTyqY8oqm4Et2z...
  Consensus:     CERTIFIED (1-hop)
```

What this tells you:
- **PRE-STATE** — snapshots both balances before the transaction
- **TRANSACTION** — submits the transfer, shows unique signature
- **CONSENSUS** — waits until the block is included and polls for state change
- **POST-STATE** — shows exact balance deltas
- **VERIFICATION** — confirms the recipient got exactly what was sent

### Deep inspect an account

```bash
eto inspect account EKmsCVcgGHxtdKxrZdRSm7WgDNU5p61MLrMnBh2ZGce3
```

Output:
```
ACCOUNT INSPECTION

  Address:    EKmsCVcgGHxtdKxrZdRSm7WgDNU5p61MLrMnBh2ZGce3
  Balance:    1000000000 lamports (1 SOL)
  Owner:      JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxEy
  Executable: false
  Data:       157 bytes

  Queried at block: #563946
```

For accounts with Universal Token Headers (76 bytes), it automatically parses:
```
  TOKEN HEADER (Universal Token Standard)
    VM Origin:  SVM
    Mint:       TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
    Owner:      6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
    Amount:     1000000000
    Decimals:   9
    Frozen:     false
```

### Inspect current block

```bash
eto inspect block
```

Output:
```
BLOCK INSPECTION

  Height:     563949
  Health:     ok
  Chain ID:   0x454f (17743)
  RPC:        http://98.93.46.219:8899
```

---

## Network Status

### Cluster info

```bash
eto cluster-info
# Health: ok
# Block Height: 560952
# Transaction Count: 31
# TPS (recent): 1192036
# Identity: 6ZrQwARijYWKZZAXe88D97mQqSqqiuBd2n59KmQRvik6
# RPC URL: http://98.93.46.219:8899
```

### Block height

```bash
eto block-height
# 560952

eto slot  # alias
# 560952
```

### Transaction count

```bash
eto transaction-count
# 108989954515
```

---

## All Commands

```
eto-cli 1.0.0
ETO Universal VM -- Command Line Interface

USAGE:
    eto <COMMAND> [OPTIONS]

COMMANDS:
    config              Get or set configuration
    balance             Show account balance
    airdrop             Request testnet tokens
    transfer            Transfer tokens
    deploy              Deploy a contract (evm, wasm, move)
    zk                  ZK operations (add, mul)
    account             Show account details
    block-height        Current block height
    slot                Current slot (alias for block-height)
    transaction-count   Total processed transactions
    cluster-info        Show cluster status
    address             Show default keypair address
    keygen              Generate a new keypair
    keypair list        List all saved keypairs
    keypair set <FILE>  Set default keypair
    keypair import <KEY> Import private key (hex or base58)
    create-account      Create a new on-chain account
    inspect             Trace and verify transactions on-chain
    help                Show this help

OPTIONS:
    -u, --url <URL>     Override RPC URL
    --help              Show help
```

---

## RPC Reference

Standard JSON-RPC 2.0 on port 8899. Compatible with curl, Postman, or any HTTP client.

### SVM Methods

```bash
# Health check
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth","params":[]}'
# → {"result":"ok"}

# Get balance
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getBalance","params":["ADDRESS"]}'
# → {"result":{"context":{"slot":560952},"value":999873742673855}}

# Get account info
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["ADDRESS"]}'
# → {"result":{"value":{"lamports":...,"owner":"...","data":"...","executable":false}}}

# Get block height
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getSlot","params":[]}'
# → {"result":560952}

# Send transaction (base64 borsh-encoded)
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"sendTransaction","params":["BASE64_TX"]}'
# → {"result":"SIGNATURE"}

# Faucet airdrop
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"faucet","params":["ADDRESS",1000000000]}'
# → {"result":"SIGNATURE"}

# Transaction count
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTransactionCount","params":[]}'
# → {"result":108989954515}
```

### EVM Methods (MetaMask compatible)

```bash
# Chain ID
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}'
# → {"result":"0x454f"}

# Block number
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}'
# → {"result":"0x89150"}

# Get balance (hex address)
curl -X POST $RPC -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":["0x0000000000000000000000000000000000000001","latest"]}'
# → {"result":"0x77359400"}
```

---

## Architecture

```
           Zone A              Zone B              Zone C
      ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
      │ Sequencer    │   │ Sequencer    │   │ Sequencer    │
      │ (g4dn GPU)   │   │ (g4dn GPU)   │   │ (g4dn GPU)   │
      │ + 3 Validators│   │ + 3 Validators│   │ + 3 Validators│
      └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
             │                   │                   │
             └───────────────────┼───────────────────┘
                                 │
                        ┌────────┴────────┐
                        │  Unified State  │
                        │  DashMap + QMDB │
                        └────────┬────────┘
                                 │
                    ┌────┬───┬───┼───┬────┐
                    │    │   │   │   │    │
                   SVM  EVM WASM Move ZK  Token
```

| | |
|-|-|
| **Consensus** | Commonware Simplex, 1-hop aggregation |
| **Finality** | ~400-800ms |
| **GPU** | NVIDIA T4 per sequencer (CUDA Ed25519) |
| **TPS** | ~1.2M per zone, ~3.6M combined |
| **VMs** | SVM, EVM (revm), WASM (wasmtime), Move (move-vm), ZK (arkworks) |
| **Token** | Universal Token Header (76 bytes, native cross-VM) |
| **Chain ID** | 0x454F (17743) |

---

## Explorer & Monitoring

| | |
|-|-|
| **Block Explorer** | http://13.222.157.139 |
| **Grafana Dashboard** | http://13.222.157.139:3000 |
| **Prometheus** | http://13.222.157.139:9090 |

---

## License

MIT
