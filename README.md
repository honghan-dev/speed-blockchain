# ⚡ Speed Blockchain

A minimal blockchain implementation written in Rust.  
Speed Blockchain demonstrates the **core components of a blockchain system**, including:

- ✅ JSON-RPC server for interaction
- ✅ Simple mempool for pending transactions
- ✅ Mining process to append blocks
- ✅ Basic account model (balances, nonces)

---

## 🛠 Features

### 1. RPC Server

- Exposes JSON-RPC endpoints for external clients (e.g. MetaMask, curl).
- Supported methods:
  - `sendTransaction` → Submit a transaction to the mempool
  - `getBalance` → Query an account balance
  - `getBlock` → Retrieve block details
  - `getTransaction` → Lookup a transaction by hash

### 2. Mempool

- Stores pending transactions before they are mined.
- Transactions are validated for:
  - Nonce correctness
  - Sufficient balance
  - Valid signature

### 3. Mining Process

- Collects transactions from the mempool.
- Creates a block and appends it to the chain.
- Runs a simple **Proof-of-Work** (or dummy difficulty check).
- Rewards the miner with new tokens.

---

## 📂 Project Structure

```rust
speed-blockchain/
├── src/
│   ├── rpc/          # JSON-RPC server
│   ├── mempool/      # Mempool logic
│   ├── mining/       # Mining process
│   ├── chain.rs      # Block & chain structures
│   ├── tx.rs         # Transaction struct & validation
│   └── main.rs       # Entry point
├── Cargo.toml
└── README.md
````

---

## 🚀 Getting Started

### 1. Clone the repo

```bash
git clone https://github.com/yourname/speed-blockchain.git
cd speed-blockchain
````

### 2. Build

```bash
cargo build
```

### 3. Run the node

```bash
cargo run
```

This starts:

- A blockchain node
- JSON-RPC server at `http://127.0.0.1:8545`

---

## 📡 Example Usage

### Send a transaction

```bash
curl -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendTransaction",
    "params": [{
      "from": "0xabc...",
      "to": "0xdef...",
      "value": 100
    }],
    "id": 1
  }'
```

### Get Block Number

```bash
curl -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'
```

---

## 📖 Notes

- **Not production-ready** — this is for learning only.
- Designed to illustrate **how Ethereum-like blockchains work**:

  - Accounts
  - Transactions
  - Mempool
  - Block production
  - RPC interface
