const net = require('net');

class NexDbClient {
  constructor(connectionString) {
    // Expected format: nexdb://token@host:port/dbname
    const match = connectionString.match(/nexdb:\/\/([^@]+)@([^:]+):(\d+)\/(.+)/);
    if (!match) {
      throw new Error("Invalid connection string format. Use: nexdb://token@host:port/dbname");
    }

    this.token = match[1];
    this.host = match[2];
    this.port = parseInt(match[3], 10);
    this.dbname = match[4];
    this.socket = null;
    this.isConnected = false;

    // Queue to support pipelined concurrent requests over a single TCP socket
    this.pendingQueue = [];
    this.buffer = "";
  }

  connect() {
    return new Promise((resolve, reject) => {
      this.socket = net.createConnection({ host: this.host, port: this.port }, () => {
        this.isConnected = true;
        resolve();
      });

      this.socket.on('error', (err) => {
        reject(err);
      });

      // Unified stream buffer splitter to prevent listener leaks and payload grouping errors
      this.socket.on('data', (chunk) => {
        this.buffer += chunk.toString();
        
        let boundary = this.buffer.indexOf('\n');
        while (boundary !== -1) {
          const line = this.buffer.substring(0, boundary).trim();
          this.buffer = this.buffer.substring(boundary + 1);
          
          if (line) {
            const callback = this.pendingQueue.shift();
            if (callback) {
              try {
                const response = JSON.parse(line);
                if (response.ok) {
                  callback.resolve(response);
                } else {
                  callback.reject(new Error(response.error || "Unknown server error"));
                }
              } catch (err) {
                callback.reject(new Error(`Failed to parse server response: ${err.message}`));
              }
            }
          }
          boundary = this.buffer.indexOf('\n');
        }
      });

      this.socket.on('close', () => {
        this.isConnected = false;
        while (this.pendingQueue.length > 0) {
          const callback = this.pendingQueue.shift();
          callback.reject(new Error("Connection closed by server"));
        }
      });
    });
  }

  query(cmd, args = {}) {
    if (!this.isConnected) {
      return Promise.reject(new Error("Client is not connected. Call connect() first."));
    }

    return new Promise((resolve, reject) => {
      const payload = {
        cmd,
        args,
        db: this.dbname,
        token: this.token
      };

      this.pendingQueue.push({ resolve, reject });
      this.socket.write(JSON.stringify(payload) + '\n');
    });
  }

  // --- Database CRUD APIs ---

  async insert(collection, id, document) {
    return this.query('insert', { collection, id, document });
  }

  async insertAutoId(collection, document) {
    const res = await this.query('insert_auto_id', { collection, document });
    return res.id;
  }

  async get(collection, id) {
    const res = await this.query('get', { collection, id });
    return res.document;
  }

  async update(collection, id, document) {
    return this.query('update', { collection, id, document });
  }

  async delete(collection, id) {
    return this.query('delete', { collection, id });
  }

  async find(collection, query = null) {
    const res = await this.query('find', { collection, query });
    return res.documents;
  }

  async count(collection) {
    const res = await this.query('count', { collection });
    return res.count;
  }

  async listCollections() {
    const res = await this.query('list_collections');
    return res.collections;
  }

  async createCollection(name) {
    return this.query('create_collection', { name });
  }

  async dropCollection(name) {
    return this.query('drop_collection', { name });
  }

  close() {
    if (this.socket) {
      this.socket.end();
      this.isConnected = false;
    }
  }
}

module.exports = { NexDbClient };
