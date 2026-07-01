# NexDB

> A **high-performance document database engine** written in Rust with Write-Ahead Logging (WAL), advanced indexing, and multi-tenant TCP server support.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Status](https://img.shields.io/badge/Status-Active%20Development-brightgreen?style=for-the-badge)](https://github.com/titanxdevz/NexDB)

</div>

---

## 📋 Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Monorepo Structure](#monorepo-structure)
- [Quick Start](#quick-start)
- [Usage Examples](#usage-examples)
- [SDK Documentation](#sdk-documentation)
- [Development](#development)

---

## Overview

NexDB is a **modern document database** designed for simplicity and performance. It provides:

- 🚀 **Fast in-memory + persistent storage** with Write-Ahead Logging (WAL)
- 🔍 **Field-level indexing** for optimized query performance
- 🏢 **Multi-tenant architecture** with built-in isolation
- 🌐 **TCP server** for distributed client connections
- 📦 **Native SDKs** for Node.js, Python, and Rust

### Pure Client-Server Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   NexDB Infrastructure                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Clients (Remote or Local)        NexDB Server         │
│  ├─ Node.js SDK ──────┐           ┌──────────────┐    │
│  ├─ Python SDK ───────┼──────────→│ TCP Server   │    │
│  ├─ CLI Utility ──────┤           │ :27017       │    │
│  └─ Rust Embedded ────┘           └──────┬───────┘    │
│                                          │             │
│                                   Disk Storage         │
│                                   (data_dir/)          │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Key Features

| Feature | Description |
|---------|-------------|
| **Write-Ahead Logging (WAL)** | Durability guarantees with crash recovery |
| **Field Indexing** | Fast, optimized queries on indexed fields |
| **Multi-Tenant Support** | Isolated databases per connection |
| **Authentication** | Secure token-based access control |
| **REPL Interface** | Interactive query shell for development |
| **Embedded Mode** | Use as a Rust library without TCP overhead |

---

## Architecture

### Monorepo Structure

```
NexDB/
├── nexdb-core/           # Core database engine & server
│   ├── src/
│   ├── Cargo.toml
│   └── README.md
├── sdk/
│   ├── node/            # Node.js client SDK
│   ├── python/          # Python client SDK
│   └── rust/            # Rust embedded library
├── nexdb-docs/          # API & architecture documentation
├── .gitignore
└── README.md
```

### Component Overview

| Component | Purpose | Language |
|-----------|---------|----------|
| **nexdb-core** | Database engine, server, CLI, REPL | Rust |
| **sdk/node** | Node.js client library | JavaScript/TypeScript |
| **sdk/python** | Python client library | Python |
| **nexdb-docs** | Full API reference & guides | Markdown |

---

## Quick Start

### 1️⃣ Start the Database Server

```bash
cd nexdb-core
cargo run --release -- serve ./data_dir --port 27017
```

### 2️⃣ Connection URL Format

```
nexdb://<auth_token>@<host>:<port>/<database_name>
```

**Example:**
```
nexdb://secrettoken@127.0.0.1:27017/my_app
```

### 3️⃣ Test with CLI

```bash
# Insert a document
cargo run --release -- insert \
  nexdb://secrettoken@127.0.0.1:27017/my_app \
  users u101 \
  '{"name":"Ansh","status":"active"}'

# Retrieve a document
cargo run --release -- get \
  nexdb://secrettoken@127.0.0.1:27017/my_app \
  users u101

# Start interactive REPL
cargo run --release -- repl \
  nexdb://secrettoken@127.0.0.1:27017/my_app
```

---

## Usage Examples

### Node.js SDK

```javascript
const { NexDbClient } = require('nexdb-sdk');

// Initialize client
const db = new NexDbClient('nexdb://secrettoken@127.0.0.1:27017/my_app');
await db.connect();

// Insert document
await db.insert('users', 'u101', {
  name: 'Ansh',
  status: 'active',
  email: 'ansh@example.com'
});

// Retrieve document
const user = await db.get('users', 'u101');
console.log(user); // { name: 'Ansh', status: 'active', ... }

// Update document
await db.update('users', 'u101', {
  status: 'inactive'
});

// Delete document
await db.delete('users', 'u101');

await db.disconnect();
```

### Python SDK

```python
from nexdb import NexDbClient

# Initialize client
db = NexDbClient('nexdb://secrettoken@127.0.0.1:27017/my_app')
db.connect()

# Insert document
db.insert('users', 'u101', {
    'name': 'Ansh',
    'status': 'active',
    'email': 'ansh@example.com'
})

# Retrieve document
user = db.get('users', 'u101')
print(user)  # {'name': 'Ansh', 'status': 'active', ...}

# Update document
db.update('users', 'u101', {'status': 'inactive'})

# Delete document
db.delete('users', 'u101')

db.disconnect()
```

### Embedded Rust Library

For native Rust applications, use NexDB as an embedded database without TCP overhead:

```rust
use nexdb::NexDb;

#[tokio::main]
async fn main() -> Result<()> {
    // Open embedded database
    let db = NexDb::open("./my_database").await?;
    
    // Insert document
    db.insert("users", "u101", serde_json::json!({
        "name": "Ansh",
        "status": "active"
    })).await?;
    
    // Retrieve document
    let user = db.get("users", "u101").await?;
    println!("{:?}", user);
    
    Ok(())
}
```

---

## SDK Documentation

### Node.js SDK

📍 **Location:** `sdk/node/`

- Connection pooling support
- Promise-based async API
- TypeScript definitions included
- Full error handling

**Installation:**
```bash
npm install nexdb-sdk
```

**Docs:** See `sdk/node/README.md` for full API reference.

### Python SDK

📍 **Location:** `sdk/python/`

- Async/await support
- Connection management
- Comprehensive error handling
- Full type hints

**Installation:**
```bash
pip install nexdb-sdk
```

**Docs:** See `sdk/python/README.md` for full API reference.

### Rust Embedded

📍 **Location:** `nexdb-core/`

Use the `nexdb` crate directly in your Cargo.toml:

```toml
[dependencies]
nexdb = { path = "../nexdb-core" }
tokio = { version = "1", features = ["full"] }
```

**Docs:** See `nexdb-docs/` for API documentation and examples.

---

## Development

### Prerequisites

- **Rust** 1.70+ ([Install](https://rustup.rs/))
- **Node.js** 16+ (for SDK development)
- **Python** 3.8+ (for SDK development)

### Building from Source

```bash
# Clone repository
git clone https://github.com/titanxdevz/NexDB.git
cd NexDB

# Build core database
cd nexdb-core
cargo build --release

# Build Node.js SDK
cd ../sdk/node
npm install
npm run build

# Build Python SDK
cd ../sdk/python
pip install -e .
```

### Running Tests

```bash
cd nexdb-core
cargo test --release
```

### Project Structure

```
nexdb-core/
├── src/
│   ├── server/      # TCP server implementation
│   ├── storage/     # WAL & persistence layer
│   ├── index/       # Field indexing engine
│   ├── cli/         # CLI tools & REPL
│   └── lib.rs
├── tests/
└── Cargo.toml
```

---

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Insert | O(log n) | With field indexing |
| Get (indexed) | O(1) - O(log n) | Depends on index selectivity |
| Get (unindexed) | O(n) | Full collection scan |
| Update | O(log n) | With indexing |
| Delete | O(log n) | With indexing |

---

## Security

- **Token-based authentication** on all connections
- **Per-database isolation** with multi-tenant support
- **Encrypted connections** recommended for production
- **Write-Ahead Logging** ensures data integrity

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

NexDB is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

---

## Support & Community

- 📚 **Documentation:** [nexdb-docs/](nexdb-docs/)
- 🐛 **Issues:** [GitHub Issues](https://github.com/titanxdevz/NexDB/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/titanxdevz/NexDB/discussions)

---

<div align="center">

**Made with ❤️ by the NexDB team**

</div>
