# NexDB

NexDB is a fast document database engine written in Rust, featuring Write-Ahead Logging (WAL) for durability, field indexing for fast queries, and multi-tenant TCP server support.

---

## Monorepo Layout

- **[nexdb-core](file:///c:/Users/royal/OneDrive/Documents/NexDB/nexdb-core)**: The database server binary (`nexdb serve`), CLI client utility, REPL, and Rust client/embedded implementation.
- **[sdk/node](file:///c:/Users/royal/OneDrive/Documents/NexDB/sdk/node)**: Node.js client SDK to connect to NexDB TCP server.
- **[sdk/python](file:///c:/Users/royal/OneDrive/Documents/NexDB/sdk/python)**: Python client SDK to connect to NexDB TCP server.
- **[nexdb-docs](file:///c:/Users/royal/OneDrive/Documents/NexDB/nexdb-docs)**: Static documentation files for APIs and architecture.

---

## Pure Client-Server Architecture (Like MongoDB or Postgres)

NexDB operates in a **pure client-server model**:
- Only the **NexDB Server** (`nexdb serve <data_dir>`) touches and manages the actual database files on the disk where the database is hosted.
- All client SDKs (Node.js, Python) and even the **NexDB CLI utility** (subcommands like `insert`, `get`, `repl`, `migrate`, etc.) connect to the database server using a connection URL.

### 1. Starting the Database Server
Run the server locally or host it on a cloud server:
```bash
cd nexdb-core
cargo run --release -- serve ./data_dir --port 27017
```

### 2. Connection URL Format
```text
nexdb://<auth_token>@<host>:<port>/<dbname>
```
Example: `nexdb://secrettoken@127.0.0.1:27017/my_app`

### 3. Using the CLI Client
All database operations in the CLI are client-side operations that communicate via the connection URL:
```bash
# Insert a document
cargo run --release -- insert nexdb://secrettoken@127.0.0.1:27017/my_app users u101 '{"name":"Ansh","status":"active"}'

# Get a document
cargo run --release -- get nexdb://secrettoken@127.0.0.1:27017/my_app users u101

# Start the interactive REPL
cargo run --release -- repl nexdb://secrettoken@127.0.0.1:27017/my_app
```

### 4. SDK Usage
- **Node.js**:
  ```javascript
  const { NexDbClient } = require('nexdb-sdk');
  const db = new NexDbClient('nexdb://secrettoken@127.0.0.1:27017/my_app');
  await db.connect();
  await db.insert('users', 'u101', { name: 'Ansh', status: 'active' });
  ```
- **Python**:
  ```python
  from nexdb import NexDbClient
  db = NexDbClient('nexdb://secrettoken@127.0.0.1:27017/my_app')
  db.connect()
  db.insert('users', 'u101', {'name': 'Ansh', 'status': 'active'})
  ```

---

## Direct Embedded Rust Library (Optional)
If writing a native Rust application, you can optionally include the `nexdb` crate as a dependency and use the embedded database engine directly without starting a TCP server:
```rust
let db = NexDb::open("./my_database").await?;
db.insert("users", "u101", doc).await?;
```
