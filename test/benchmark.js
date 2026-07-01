const { NexDbClient } = require('../sdk/node/index.js');

const CONN_STR = 'nexdb://testtoken@127.0.0.1:27017/benchmark_db';

async function runSuite() {
  console.log("==================================================");
  console.log("        NEXDB PERFORMANCE & SECURITY SUITE        ");
  console.log("==================================================");
  console.log(`Target: ${CONN_STR}\n`);

  const client = new NexDbClient(CONN_STR);
  try {
    await client.connect();
    console.log("✓ Established connection to NexDB server.");
  } catch (err) {
    console.error("✕ Connection failed! Please make sure nexdb server is running on port 27017.");
    console.error("  Start command: nexdb serve ./data --port 27017");
    process.exit(1);
  }

  // Ensure test collection is clean
  try {
    await client.createCollection('benchmark_col');
  } catch {}

  const NUM_DOCS = 10000;

  const runId = Math.random().toString(36).substring(2, 7);

  // ─── 1. WRITE BENCHMARK ───
  console.log(`\n[1/4] Running Write Benchmark (${NUM_DOCS} inserts)...`);
  const writeStart = Date.now();
  const writePromises = [];

  for (let i = 0; i < NUM_DOCS; i++) {
    writePromises.push(
      client.insert('benchmark_col', `key_${runId}_${i}`, {
        index: i,
        data: "nexdb_speed_test_line_value_12345",
        timestamp: Date.now()
      }).catch(err => ({ error: true, message: err.message }))
    );
  }

  const writeResults = await Promise.all(writePromises);
  const writeEnd = Date.now();
  const writeDuration = (writeEnd - writeStart) / 1000;
  const writeErrors = writeResults.filter(r => r && r.error);
  const writeSuccesses = NUM_DOCS - writeErrors.length;
  const writeOps = writeSuccesses / writeDuration;
  const avgWriteLatency = (writeEnd - writeStart) / NUM_DOCS;

  console.log(`  - Total Duration: ${writeDuration.toFixed(3)}s`);
  console.log(`  - Success rate:   ${writeSuccesses}/${NUM_DOCS} writes`);
  console.log(`  - Write Speed:    ${writeOps.toFixed(0)} OPS (Operations Per Second)`);
  console.log(`  - Avg Latency:    ${avgWriteLatency.toFixed(2)}ms`);
  if (writeErrors.length > 0) {
    console.error(`  ✕ Errors encountered: ${writeErrors[0].message}`);
  }

  // ─── 2. READ BENCHMARK ───
  console.log(`\n[2/4] Running Read Benchmark (${NUM_DOCS} reads)...`);
  const readStart = Date.now();
  const readPromises = [];

  for (let i = 0; i < NUM_DOCS; i++) {
    readPromises.push(
      client.get('benchmark_col', `key_${runId}_${i}`).catch(err => ({ error: true, message: err.message }))
    );
  }

  const readResults = await Promise.all(readPromises);
  const readEnd = Date.now();
  const readDuration = (readEnd - readStart) / 1000;
  const readErrors = readResults.filter(r => r && r.error);
  const readSuccesses = NUM_DOCS - readErrors.length;
  const readOps = readSuccesses / readDuration;
  const avgReadLatency = (readEnd - readStart) / NUM_DOCS;

  console.log(`  - Total Duration: ${readDuration.toFixed(3)}s`);
  console.log(`  - Success rate:   ${readSuccesses}/${NUM_DOCS} reads`);
  console.log(`  - Read Speed:     ${readOps.toFixed(0)} OPS (Operations Per Second)`);
  console.log(`  - Avg Latency:    ${avgReadLatency.toFixed(2)}ms`);
  if (readErrors.length > 0) {
    console.error(`  ✕ Errors encountered: ${readErrors[0].message}`);
  }

  // ─── 3. ROBUSTNESS & CRASH RESILIENCY CHECKS ───
  console.log("\n[3/4] Running Robustness & Crash Checks...");
  
  // Test invalid command format
  try {
    await client.query("non_existent_command", {});
    console.log("  ✕ Fail: Server processed invalid command without error");
  } catch (err) {
    console.log(`  ✓ Pass: Server rejected invalid command correctly: "${err.message}"`);
  }

  // Test invalid JSON command line (checks if server crashes on bad bytes)
  const net = require('net');
  const crashTestPromise = new Promise((resolve) => {
    const socket = net.createConnection({ port: 27017, host: '127.0.0.1' }, () => {
      // Send corrupted JSON packet
      socket.write("{invalid_json:true,\n");
      
      socket.once('data', (data) => {
        const reply = data.toString().trim();
        if (reply.includes("parse error")) {
          console.log(`  ✓ Pass: Server rejected corrupted JSON data line: "${reply}"`);
        } else {
          console.log(`  ✕ Warning: Server response did not indicate parse error: "${reply}"`);
        }
        socket.end();
        resolve();
      });
    });
    socket.on('error', () => {
      console.log("  ✕ Fail: Connection refused or server crashed");
      resolve();
    });
  });
  await crashTestPromise;

  // ─── 4. SECURITY SCANS ───
  console.log("\n[4/4] Running Security Vulnerability Scans...");

  // Scan A: Path Traversal Check (Ensures DatabaseManager directory traversal is blocked)
  try {
    const traversalClient = new NexDbClient('nexdb://testtoken@127.0.0.1:27017/../../hacked_db');
    await traversalClient.connect();
    await traversalClient.query('ping');
    console.log("  ✕ FAIL: Path traversal allowed! Server processed connection string containing '/../'");
    traversalClient.close();
  } catch (err) {
    console.log(`  ✓ Pass: Path traversal connection string blocked: "${err.message}"`);
  }

  // Scan B: Authentication Bypass Check
  const bypassPromise = new Promise((resolve) => {
    const socket = net.createConnection({ port: 27017, host: '127.0.0.1' }, () => {
      // Send query payload without any auth token (Note: server must run with authentication required or auth flag enabled to activate this)
      socket.write(JSON.stringify({ cmd: "list_collections", db: "benchmark_db" }) + "\n");
      
      socket.once('data', (data) => {
        const reply = data.toString().trim();
        if (reply.includes("authentication required")) {
          console.log(`  ✓ Pass: Command query rejected due to missing auth token: "${reply}"`);
        } else {
          console.log(`  ✕ Info: Auth not active on server or bypassed: "${reply}"`);
        }
        socket.end();
        resolve();
      });
    });
    socket.on('error', () => resolve());
  });
  await bypassPromise;

  client.close();
  console.log("\n==================================================");
  console.log("              TEST SUITE CONCLUDED                ");
  console.log("==================================================");
}

runSuite().catch(console.error);
