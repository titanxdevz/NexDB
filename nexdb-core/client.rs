use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use serde_json::{json, Value};
use crate::error::{NexDbError, NexDbResult};

pub struct NexDbClient {
    token: String,
    host: String,
    port: u16,
    dbname: String,
    stream: Option<BufReader<TcpStream>>,
}

impl NexDbClient {
    pub fn new(connection_string: &str) -> NexDbResult<Self> {
        // Parse URI: nexdb://token@host:port/dbname
        let clean = connection_string.replace("nexdb://", "");
        let parts: Vec<&str> = clean.split('@').collect();
        if parts.len() != 2 {
            return Err(NexDbError::InvalidOperation("Invalid connection string format: missing '@'".into()));
        }
        let token = parts[0].to_string();
        let rest = parts[1];
        let subparts: Vec<&str> = rest.split('/').collect();
        if subparts.len() != 2 {
            return Err(NexDbError::InvalidOperation("Invalid connection string format: missing '/'".into()));
        }
        let host_port = subparts[0];
        let dbname = subparts[1].to_string();
        
        let hp: Vec<&str> = host_port.split(':').collect();
        if hp.len() != 2 {
            return Err(NexDbError::InvalidOperation("Invalid connection string format: missing ':'".into()));
        }
        let host = hp[0].to_string();
        let port = hp[1].parse::<u16>().map_err(|_| NexDbError::InvalidOperation("Invalid port in connection string".into()))?;

        Ok(NexDbClient {
            token,
            host,
            port,
            dbname,
            stream: None,
        })
    }

    pub async fn connect(&mut self) -> NexDbResult<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(addr).await
            .map_err(|e| NexDbError::Io(e))?;
        self.stream = Some(BufReader::new(stream));
        Ok(())
    }

    pub async fn query(&mut self, cmd: &str, args: Value) -> NexDbResult<Value> {
        let reader = match &mut self.stream {
            Some(r) => r,
            None => return Err(NexDbError::InvalidOperation("Client not connected. Call connect() first.".into())),
        };

        let payload = json!({
            "cmd": cmd,
            "args": args,
            "db": self.dbname,
            "token": self.token
        });

        let mut request_line = serde_json::to_string(&payload).unwrap();
        request_line.push('\n');

        // Write to network stream
        let socket = reader.get_mut();
        socket.write_all(request_line.as_bytes()).await
            .map_err(|e| NexDbError::Io(e))?;
        socket.flush().await
            .map_err(|e| NexDbError::Io(e))?;

        // Read response line
        let mut line = String::new();
        reader.read_line(&mut line).await
            .map_err(|e| NexDbError::Io(e))?;

        let response: Value = serde_json::from_str(&line)
            .map_err(|e| NexDbError::InvalidOperation(format!("Failed to parse response: {}", e)))?;

        if response.get("ok").and_then(|ok| ok.as_bool()).unwrap_or(false) {
            Ok(response)
        } else {
            let err = response.get("error").and_then(|e| e.as_str()).unwrap_or("Unknown server error");
            Err(NexDbError::InvalidOperation(err.to_string()))
        }
    }

    // --- High level client wrappers ---

    pub async fn create_collection(&mut self, name: &str) -> NexDbResult<Value> {
        self.query("create_collection", json!({ "name": name })).await
    }

    pub async fn drop_collection(&mut self, name: &str) -> NexDbResult<Value> {
        self.query("drop_collection", json!({ "name": name })).await
    }

    pub async fn list_collections(&mut self) -> NexDbResult<Value> {
        self.query("list_collections", json!({})).await
    }

    pub async fn insert(&mut self, collection: &str, id: &str, document: Value) -> NexDbResult<Value> {
        self.query("insert", json!({
            "collection": collection,
            "id": id,
            "document": document
        })).await
    }

    pub async fn insert_auto_id(&mut self, collection: &str, document: Value) -> NexDbResult<Value> {
        self.query("insert_auto_id", json!({
            "collection": collection,
            "document": document
        })).await
    }

    pub async fn get(&mut self, collection: &str, id: &str) -> NexDbResult<Value> {
        self.query("get", json!({
            "collection": collection,
            "id": id
        })).await
    }

    pub async fn update(&mut self, collection: &str, id: &str, document: Value) -> NexDbResult<Value> {
        self.query("update", json!({
            "collection": collection,
            "id": id,
            "document": document
        })).await
    }

    pub async fn delete(&mut self, collection: &str, id: &str) -> NexDbResult<Value> {
        self.query("delete", json!({
            "collection": collection,
            "id": id
        })).await
    }

    pub async fn find(&mut self, collection: &str, query_val: Value) -> NexDbResult<Value> {
        self.query("find", json!({
            "collection": collection,
            "query": query_val
        })).await
    }

    pub async fn count(&mut self, collection: &str) -> NexDbResult<Value> {
        self.query("count", json!({
            "collection": collection
        })).await
    }

    pub async fn create_index(&mut self, collection: &str, name: &str, field: &str) -> NexDbResult<Value> {
        self.query("create_index", json!({
            "collection": collection,
            "name": name,
            "field": field
        })).await
    }

    pub async fn checkpoint(&mut self) -> NexDbResult<Value> {
        self.query("checkpoint", json!({})).await
    }
}
