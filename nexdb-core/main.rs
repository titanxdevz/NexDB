use std::path::Path;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use nexdb::{Document, NexDb, NexDbResult, DatabaseManager, NexDbClient};
use nexdb::migrate;
use nexdb::server;

fn usage() -> ! {
    eprintln!("NexDb v{} - Document Database Client & Server", nexdb::version());
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  nexdb serve <data_dir> [--port N] [--metrics-port M]  Start TCP server");
    eprintln!();
    eprintln!("CLIENT COMMANDS:");
    eprintln!("  nexdb repl <connection_url>                           Start interactive REPL");
    eprintln!("  nexdb insert <connection_url> <col> <id> <json>        Insert a document");
    eprintln!("  nexdb insert-auto-id <connection_url> <col> <json>     Insert with auto ID");
    eprintln!("  nexdb get <connection_url> <col> <id>                  Get a document");
    eprintln!("  nexdb update <connection_url> <col> <id> <json>        Update a document");
    eprintln!("  nexdb delete <connection_url> <col> <id>               Delete a document");
    eprintln!("  nexdb find <connection_url> <col>                      List all documents");
    eprintln!("  nexdb count <connection_url> <col>                     Count documents");
    eprintln!("  nexdb collections <connection_url>                     List collections");
    eprintln!("  nexdb create-collection <connection_url> <col>         Create a collection");
    eprintln!("  nexdb drop-collection <connection_url> <col>           Drop a collection");
    eprintln!("  nexdb checkpoint <connection_url>                      WAL checkpoint (snapshot)");
    eprintln!("  nexdb import <connection_url> <col> <file.json>        Import JSON");
    eprintln!("  nexdb export <connection_url> <col> <file.json>        Export JSON");
    eprintln!("  nexdb import-csv <connection_url> <col> <file.csv>     Import CSV");
    eprintln!("  nexdb export-csv <connection_url> <col> <file.csv>     Export CSV");
    eprintln!("  nexdb migrate <action> [args]                          Migrate data (dump/restore/copy/to-sql/import)");
    eprintln!("  nexdb clean <db_path>                                  Remove local database files");
    eprintln!("  nexdb clean --all <dir>                                Remove all .nexdb files in a directory");
    eprintln!("  nexdb completions <shell>                              Generate shell completions");
    eprintln!();
    eprintln!("Connection URL Format: nexdb://token@host:port/dbname");
    std::process::exit(1);
}

async fn connect_client(url: &str) -> NexDbResult<NexDbClient> {
    let mut client = NexDbClient::new(url)?;
    client.connect().await?;
    Ok(client)
}

#[tokio::main]
async fn main() -> NexDbResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }

    match args[1].as_str() {
        "repl" => {
            if args.len() < 3 { usage(); }
            run_repl(&args[2]).await
        }
        "serve" => {
            if args.len() < 3 { usage(); }
            let port = args.iter().position(|a| a == "--port")
                .and_then(|i| args.get(i + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(27017);
            let metrics_port = args.iter().position(|a| a == "--metrics-port")
                .and_then(|i| args.get(i + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(28017);
            let db_mgr = DatabaseManager::new(&args[2]);
            let db_mgr = std::sync::Arc::new(db_mgr);
            let config = server::ServerConfig {
                bind_addr: format!("0.0.0.0:{}", port),
                metrics_addr: format!("0.0.0.0:{}", metrics_port),
                ..Default::default()
            };
            server::start_server(db_mgr, config).await
        }
        "insert" => {
            if args.len() < 6 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let doc_val: Value = serde_json::from_str(&args[5])
                .map_err(|e| nexdb::NexDbError::InvalidOperation(format!("invalid JSON: {}", e)))?;
            let res = client.insert(&args[3], &args[4], doc_val).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "get" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            match client.get(&args[3], &args[4]).await {
                Ok(res) => {
                    if let Some(doc) = res.get("document") {
                        println!("{}", serde_json::to_string_pretty(doc).unwrap());
                    } else {
                        println!("{}", serde_json::to_string_pretty(&res).unwrap());
                    }
                }
                Err(e) => eprintln!("{}", e),
            }
            Ok(())
        }
        "update" => {
            if args.len() < 6 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let doc_val: Value = serde_json::from_str(&args[5])
                .map_err(|e| nexdb::NexDbError::InvalidOperation(format!("invalid JSON: {}", e)))?;
            let res = client.update(&args[3], &args[4], doc_val).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "delete" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.delete(&args[3], &args[4]).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "count" => {
            if args.len() < 4 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.count(&args[3]).await?;
            if let Some(count) = res.get("count") {
                println!("{}", count);
            } else {
                println!("{}", serde_json::to_string(&res).unwrap());
            }
            Ok(())
        }
        "collections" | "list" => {
            if args.len() < 3 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.list_collections().await?;
            if let Some(collections) = res.get("collections") {
                println!("{}", serde_json::to_string_pretty(collections).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&res).unwrap());
            }
            Ok(())
        }
        "create-collection" => {
            if args.len() < 4 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.create_collection(&args[3]).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "drop-collection" => {
            if args.len() < 4 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.drop_collection(&args[3]).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "find" => {
            if args.len() < 4 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let query_val = if args.len() >= 5 {
                serde_json::from_str(&args[4]).unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            let res = client.find(&args[3], query_val).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "insert-auto-id" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let doc_val: Value = serde_json::from_str(&args[4])
                .map_err(|e| nexdb::NexDbError::InvalidOperation(format!("invalid JSON: {}", e)))?;
            let res = client.insert_auto_id(&args[3], doc_val).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "flush" => {
            if args.len() < 3 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.query("flush", Value::Null).await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "checkpoint" => {
            if args.len() < 3 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let res = client.checkpoint().await?;
            println!("{}", serde_json::to_string(&res).unwrap());
            Ok(())
        }
        "import" | "import-json" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let count = migrate::auto_import(&mut client, &args[3], Path::new(&args[4])).await?;
            println!(r#"{{"ok":true,"imported":{}}}"#, count);
            Ok(())
        }
        "export" | "export-json" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let manifest = migrate::dump(&mut client, &args[4]).await?;
            println!(r#"{{"ok":true,"exported_collections":{}}}"#, serde_json::to_string(&manifest.collections).unwrap());
            Ok(())
        }
        "import-csv" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let count = migrate::auto_import(&mut client, &args[3], Path::new(&args[4])).await?;
            println!(r#"{{"ok":true,"imported":{}}}"#, count);
            Ok(())
        }
        "export-csv" => {
            if args.len() < 5 { usage(); }
            let mut client = connect_client(&args[2]).await?;
            let list_res = client.find(&args[3], Value::Null).await?;
            let docs: Vec<Value> = serde_json::from_value(list_res.get("documents").cloned().unwrap_or(Value::Null))
                .unwrap_or_default();
            
            if docs.is_empty() {
                tokio::fs::write(&args[4], "").await?;
            } else {
                let mut all_keys: Vec<String> = Vec::new();
                for item in &docs {
                    let doc = item.get("document").cloned().unwrap_or(Value::Object(Default::default()));
                    if let Value::Object(ref map) = doc {
                        for key in map.keys() {
                            if !all_keys.contains(key) {
                                all_keys.push(key.clone());
                            }
                        }
                    }
                }
                
                let mut file = std::fs::File::create(&args[4])?;
                use std::io::Write;
                writeln!(file, "{}", all_keys.join(","))?;
                for item in &docs {
                    let doc = item.get("document").cloned().unwrap_or(Value::Object(Default::default()));
                    let row: Vec<String> = all_keys.iter().map(|key| {
                        doc.get(key).map(|v| match v {
                            Value::String(s) => format!("\"{}\"", s.replace('"', "\"\"")),
                            Value::Null => String::new(),
                            other => other.to_string(),
                        }).unwrap_or_default()
                    }).collect();
                    writeln!(file, "{}", row.join(","))?;
                }
            }
            println!(r#"{{"ok":true,"exported_to":"{}"}}"#, args[4]);
            Ok(())
        }
        "migrate" => run_migrate(&args).await,
        "clean" => run_clean(&args).await,
        "completions" => {
            if args.len() < 3 { usage(); }
            generate_completions(&args[2])
        }
        _ => usage(),
    }
}

async fn run_migrate(args: &[String]) -> NexDbResult<()> {
    if args.len() < 3 {
        eprintln!("nexdb migrate <action> [args...]");
        eprintln!();
        eprintln!("ACTIONS:");
        eprintln!("  dump <connection_url> <dir>              Dump all collections to JSON files");
        eprintln!("  restore <connection_url> <dir>            Restore all collections from JSON files");
        eprintln!("  copy <src_url> <dst_url>              Copy data between two databases");
        eprintln!("  to-sql <connection_url> <dialect>         Generate SQL dump (pg, mysql, sqlite)");
        eprintln!("  import <connection_url> <col> <file>      Auto-detect and import (.json/.csv/.ndjson)");
        std::process::exit(1);
    }

    match args[2].as_str() {
        "dump" => {
            if args.len() < 5 { eprintln!("Usage: nexdb migrate dump <connection_url> <dir>"); std::process::exit(1); }
            let mut client = connect_client(&args[3]).await?;
            let manifest = migrate::dump(&mut client, &args[4]).await?;
            println!("Dumped {} collections ({} docs total)", manifest.collections.len(), manifest.total_docs);
            Ok(())
        }
        "restore" => {
            if args.len() < 5 { eprintln!("Usage: nexdb migrate restore <connection_url> <dir>"); std::process::exit(1); }
            let mut client = connect_client(&args[3]).await?;
            let manifest = migrate::restore(&mut client, &args[4]).await?;
            println!("Restored {} collections ({} docs total)", manifest.collections.len(), manifest.total_docs);
            Ok(())
        }
        "copy" => {
            if args.len() < 5 { eprintln!("Usage: nexdb migrate copy <src_url> <dst_url>"); std::process::exit(1); }
            let mut source = connect_client(&args[3]).await?;
            let mut target = connect_client(&args[4]).await?;
            let manifest = migrate::copy(&mut source, &mut target).await?;
            println!("Copied {} collections ({} docs total)", manifest.collections.len(), manifest.total_docs);
            Ok(())
        }
        "to-sql" => {
            if args.len() < 5 { eprintln!("Usage: nexdb migrate to-sql <connection_url> <dialect>"); std::process::exit(1); }
            let mut client = connect_client(&args[3]).await?;
            let sql = migrate::to_sql(&mut client, &args[4]).await?;
            println!("{}", sql);
            Ok(())
        }
        "import" => {
            if args.len() < 6 { eprintln!("Usage: nexdb migrate import <connection_url> <col> <file>"); std::process::exit(1); }
            let mut client = connect_client(&args[3]).await?;
            let count = migrate::auto_import(&mut client, &args[4], &args[5]).await?;
            println!("Imported {} documents", count);
            Ok(())
        }
        _ => {
            eprintln!("Unknown migrate action: {}", args[2]);
            std::process::exit(1);
        }
    }
}

async fn run_clean(args: &[String]) -> NexDbResult<()> {
    if args.len() < 3 {
        eprintln!("Usage: nexdb clean <db_path>          Remove local database files");
        eprintln!("       nexdb clean --all <dir>        Remove all .nexdb files in directory");
        std::process::exit(1);
    }

    match args[2].as_str() {
        "--all" => {
            if args.len() < 4 { eprintln!("Usage: nexdb clean --all <dir>"); std::process::exit(1); }
            let removed = migrate::clean_all(&args[3]).await?;
            println!("Cleaned {} files", removed);
            Ok(())
        }
        path => {
            let removed = migrate::clean(path).await?;
            println!("Cleaned {} file(s)", removed);
            Ok(())
        }
    }
}

fn generate_completions(shell: &str) -> NexDbResult<()> {
    #[cfg(feature = "completions")]
    {
        use clap_complete::{Generator, Shell};
        use std::io;

        let shell = match shell {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
                eprintln!("Unknown shell: {}. Supported: bash, zsh, fish, powershell, elvish", shell);
                std::process::exit(1);
            }
        };

        let cmd = build_clap_app();
        let mut out = io::stdout();
        shell.generate(&cmd, &mut out);
        Ok(())
    }

    #[cfg(not(feature = "completions"))]
    {
        eprintln!("Completions not available. Rebuild with --features completions");
        std::process::exit(1);
    }
}

#[cfg(feature = "completions")]
fn build_clap_app() -> clap::Command {
    use clap::{Arg, Command};

    Command::new("nexdb")
        .about("NexDb - Document Database Client & Server")
        .subcommand(Command::new("repl").about("Start interactive REPL")
            .arg(Arg::new("connection_url").required(true)))
        .subcommand(Command::new("serve").about("Start TCP server")
            .arg(Arg::new("db_dir").required(true))
            .arg(Arg::new("--port").short('p').help("Port to bind")))
        .subcommand(Command::new("insert").about("Insert a document")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("id").required(true))
            .arg(Arg::new("json").required(true)))
        .subcommand(Command::new("get").about("Get a document")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("id").required(true)))
        .subcommand(Command::new("update").about("Update a document")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("id").required(true))
            .arg(Arg::new("json").required(true)))
        .subcommand(Command::new("delete").about("Delete a document")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("id").required(true)))
        .subcommand(Command::new("checkpoint").about("WAL checkpoint")
            .arg(Arg::new("connection_url").required(true)))
        .subcommand(Command::new("import").about("Import JSON")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("file").required(true)))
        .subcommand(Command::new("export").about("Export JSON")
            .arg(Arg::new("connection_url").required(true))
            .arg(Arg::new("collection").required(true))
            .arg(Arg::new("file").required(true)))
        .subcommand(Command::new("completions").about("Generate shell completions")
            .arg(Arg::new("shell").required(true)))
}

async fn run_repl(connection_url: &str) -> NexDbResult<()> {
    let mut client = connect_client(connection_url).await?;
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();

    loop {
        let line = match lines.next_line().await? {
            Some(l) => l,
            None => break,
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        let response = handle_json_client_command(&mut client, &trimmed).await;
        let out = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"ok":false,"error":"serialization error"}"#.to_string()
        });
        stdout.write_all(out.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_json_client_command(client: &mut NexDbClient, json_str: &str) -> Value {
    let cmd_val: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({"ok": false, "error": format!("invalid JSON: {}", e)}),
    };

    let command = match cmd_val.get("cmd").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return serde_json::json!({"ok": false, "error": "missing 'cmd' field"}),
    };

    if command == "exit" || command == "quit" {
        std::process::exit(0);
    }

    let args = cmd_val.get("args").cloned().unwrap_or(Value::Null);

    match client.query(command, args).await {
        Ok(res) => res,
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    }
}
