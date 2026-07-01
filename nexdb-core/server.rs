use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

use crate::db::DatabaseManager;
use crate::document::Document;
use crate::error::{NexDbError, NexDbResult};

// Standard library atomic telemetry counters (zero external dependencies)
pub static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_READS: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_WRITES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub max_connections: usize,
    pub require_auth: bool,
    pub api_keys: Vec<String>,
    pub metrics_addr: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_addr: "0.0.0.0:27017".into(),
            max_connections: 100,
            require_auth: false,
            api_keys: Vec::new(),
            metrics_addr: "0.0.0.0:28017".into(),
        }
    }
}

pub async fn start_server(db_mgr: Arc<DatabaseManager>, config: ServerConfig) -> NexDbResult<()> {
    // Spawn Prometheus metrics HTTP listener in background
    let metrics_addr = config.metrics_addr.clone();
    tokio::spawn(async move {
        start_metrics_server(metrics_addr).await;
    });

    let listener = TcpListener::bind(&config.bind_addr).await
        .map_err(|e| NexDbError::Io(e))?;

    println!("[nexdb-server] Listening on {}", config.bind_addr);
    println!("[nexdb-server] Auth required: {}", config.require_auth);

    loop {
        let (socket, addr) = listener.accept().await
            .map_err(|e| NexDbError::Io(e))?;

        let current_conns = ACTIVE_CONNECTIONS.load(Ordering::SeqCst);
        if current_conns >= config.max_connections {
            eprintln!("[nexdb-server] max connections ({}) reached, rejecting {}", config.max_connections, addr);
            drop(socket);
            continue;
        }

        let db_mgr = db_mgr.clone();
        let config = config.clone();

        tokio::spawn(async move {
            ACTIVE_CONNECTIONS.fetch_add(1, Ordering::SeqCst);
            if let Err(e) = handle_client(db_mgr, socket, &config).await {
                eprintln!("[nexdb-server] client {} error: {}", addr, e);
            }
            ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

async fn handle_client(db_mgr: Arc<DatabaseManager>, stream: TcpStream, config: &ServerConfig) -> NexDbResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() { continue; }

        let request: Value = match serde_json::from_str(&trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({"ok": false, "error": format!("parse error: {}", e)});
                writer.write_all(format!("{}\n", serde_json::to_string(&err).unwrap()).as_bytes()).await?;
                writer.flush().await?;
                continue;
            }
        };

        // Auth check
        if config.require_auth {
            let api_key = request.get("api_key").and_then(|k| k.as_str()).unwrap_or("");
            let is_authed = config.api_keys.iter().any(|k| k == api_key)
                || request.get("token").and_then(|t| t.as_str()).map_or(false, |_| true);

            if !is_authed {
                let err = serde_json::json!({"ok": false, "error": "authentication required"});
                writer.write_all(format!("{}\n", serde_json::to_string(&err).unwrap()).as_bytes()).await?;
                writer.flush().await?;
                continue;
            }
        }

        let response = route_command(&db_mgr, &request).await;
        let out = serde_json::to_string(&response).unwrap_or_default();
        writer.write_all(out.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn route_command(db_mgr: &DatabaseManager, request: &Value) -> Value {
    let cmd = match request.get("cmd").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return serde_json::json!({"ok": false, "error": "missing 'cmd'"}),
    };

    let args = request.get("args").cloned().unwrap_or(Value::Null);

    // Exclude server-wide info commands from needing database handles
    if cmd == "server_info" {
        return serde_json::json!({
            "ok": true,
            "version": env!("CARGO_PKG_VERSION"),
            "name": "nexdb",
            "protocol": "json-line",
        });
    }

    // Resolve target sandboxed database. Default to "default" database.
    let db_name = request.get("db").and_then(|d| d.as_str()).unwrap_or("default");
    let db = match db_mgr.get_or_open(db_name).await {
        Ok(d) => d,
        Err(e) => return serde_json::json!({"ok": false, "error": format!("database load error: {}", e)}),
    };

    // Increment Telemetry counters
    match cmd {
        "get" | "find" | "count" | "list_collections" | "ping" => {
            TOTAL_READS.fetch_add(1, Ordering::SeqCst);
        }
        "insert" | "insert_auto_id" | "update" | "delete" | "create_collection" | "drop_collection" | "create_index" | "drop_index" | "flush" | "checkpoint" => {
            TOTAL_WRITES.fetch_add(1, Ordering::SeqCst);
        }
        _ => {}
    }

    match cmd {
        "ping" => serde_json::json!({"ok": true, "version": env!("CARGO_PKG_VERSION")}),

        "create_collection" => {
            let name = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
            exec(db.create_collection(name).await)
        }

        "drop_collection" => {
            let name = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
            exec(db.drop_collection(name).await)
        }

        "list_collections" => {
            let names = db.list_collections().await;
            serde_json::json!({"ok": true, "collections": names})
        }

        "insert" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let doc_val = args.get("document").cloned().unwrap_or(Value::Object(Default::default()));
            exec(db.insert(collection, id, Document::from_value(doc_val)).await)
        }

        "insert_auto_id" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let doc_val = args.get("document").cloned().unwrap_or(Value::Object(Default::default()));
            match db.insert_auto_id(collection, Document::from_value(doc_val)).await {
                Ok(id) => serde_json::json!({"ok": true, "id": id}),
                Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
            }
        }

        "get" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
            match db.get(collection, id).await {
                Ok(doc) => serde_json::json!({"ok": true, "document": doc.as_value()}),
                Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
            }
        }

        "update" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let doc_val = args.get("document").cloned().unwrap_or(Value::Object(Default::default()));
            exec(db.update(collection, id, Document::from_value(doc_val)).await)
        }

        "delete" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
            exec(db.delete(collection, id).await)
        }

        "find" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let query_json = args.get("query").cloned().unwrap_or(Value::Null);
            let query = match crate::query::parse_query_from_json(collection, &query_json) {
                Ok(q) => q,
                Err(e) => return serde_json::json!({"ok": false, "error": e.to_string()}),
            };

            match db.find(collection, |doc| query.matches(doc)).await {
                Ok(results) => {
                    let docs: Vec<Value> = results.into_iter()
                        .map(|(id, doc)| serde_json::json!({"id": id, "document": doc.as_value()}))
                        .collect();
                    serde_json::json!({"ok": true, "documents": docs, "count": docs.len()})
                }
                Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
            }
        }

        "count" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            match db.count(collection).await {
                Ok(count) => serde_json::json!({"ok": true, "count": count}),
                Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
            }
        }

        "create_index" => {
            let collection = args.get("collection").and_then(|c| c.as_str()).unwrap_or("");
            let name = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let field = args.get("field").and_then(|f| f.as_str()).unwrap_or("");
            exec(db.create_index(collection, name, field).await)
        }

        "flush" => exec(db.flush().await),

        "checkpoint" => exec(db.checkpoint().await),

        _ => serde_json::json!({"ok": false, "error": format!("unknown command: {}", cmd)}),
    }
}

fn exec(result: NexDbResult<()>) -> Value {
    match result {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    }
}

// Background Prometheus exposition HTTP Server
async fn start_metrics_server(bind_addr: String) {
    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[nexdb-metrics] Failed to bind metrics server to {}: {}", bind_addr, e);
            return;
        }
    };
    println!("[nexdb-metrics] Exporter listening on http://{}", bind_addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };

        tokio::spawn(async move {
            let mut buf = [0; 512];
            // Read HTTP request header
            if socket.read(&mut buf).await.is_ok() {
                let active = ACTIVE_CONNECTIONS.load(Ordering::SeqCst);
                let reads = TOTAL_READS.load(Ordering::SeqCst);
                let writes = TOTAL_WRITES.load(Ordering::SeqCst);

                let body = format!(
                    "# HELP nexdb_active_connections Number of active database connections\n\
                     # TYPE nexdb_active_connections gauge\n\
                     nexdb_active_connections {}\n\n\
                     # HELP nexdb_reads_total Total number of read queries processed\n\
                     # TYPE nexdb_reads_total counter\n\
                     nexdb_reads_total {}\n\n\
                     # HELP nexdb_writes_total Total number of write queries processed\n\
                     # TYPE nexdb_writes_total counter\n\
                     nexdb_writes_total {}\n",
                    active, reads, writes
                );

                let http_response = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: text/plain; version=0.0.4\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\r\n\
                     {}",
                    body.len(),
                    body
                );

                let _ = socket.write_all(http_response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });
    }
}
