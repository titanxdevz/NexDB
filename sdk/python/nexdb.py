import socket
import json
import re

class NexDbClient:
    def __init__(self, connection_string):
        # Expected format: nexdb://token@host:port/dbname
        pattern = r"nexdb:\/\/([^@]+)@([^:]+):(\d+)\/(.+)"
        match = re.match(pattern, connection_string)
        if not match:
            raise ValueError("Invalid connection string format. Use: nexdb://token@host:port/dbname")
        
        self.token = match.group(1)
        self.host = match.group(2)
        self.port = int(match.group(3))
        self.dbname = match.group(4)
        self.socket = None
        self.is_connected = False

    def connect(self):
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.socket.connect((self.host, self.port))
        self.is_connected = True

    def query(self, cmd, args=None):
        if not self.is_connected:
            raise RuntimeError("Client is not connected. Call connect() first.")
        
        if args is None:
            args = {}
            
        payload = {
            "cmd": cmd,
            "args": args,
            "db": self.dbname,
            "token": self.token
        }
        
        # Send json terminated by newline
        request_str = json.dumps(payload) + "\n"
        self.socket.sendall(request_str.encode('utf-8'))
        
        # Read reply (until newline)
        response_bytes = bytearray()
        while True:
            chunk = self.socket.recv(1024)
            if not chunk:
                raise ConnectionError("Server disconnected unexpectedly")
            response_bytes.extend(chunk)
            if b'\n' in chunk:
                break
                
        response_line = response_bytes.decode('utf-8').split('\n')[0].strip()
        if not response_line:
            raise ValueError("Empty response from server")
            
        response = json.loads(response_line)
        if response.get("ok"):
            return response
        else:
            raise RuntimeError(response.get("error", "Unknown server error"))

    def insert(self, collection, id, document):
        return self.query('insert', {"collection": collection, "id": id, "document": document})

    def insert_auto_id(self, collection, document):
        res = self.query('insert_auto_id', {"collection": collection, "document": document})
        return res.get("id")

    def get(self, collection, id):
        res = self.query('get', {"collection": collection, "id": id})
        return res.get("document")

    def update(self, collection, id, document):
        return self.query('update', {"collection": collection, "id": id, "document": document})

    def delete(self, collection, id):
        return self.query('delete', {"collection": collection, "id": id})

    def find(self, collection, query=None):
        res = self.query('find', {"collection": collection, "query": query})
        return res.get("documents")

    def count(self, collection):
        res = self.query('count', {"collection": collection})
        return res.get("count")

    def list_collections(self):
        res = self.query('list_collections')
        return res.get("collections")

    def create_collection(self, name):
        return self.query('create_collection', {"name": name})

    def drop_collection(self, name):
        return self.query('drop_collection', {"name": name})

    def close(self):
        if self.socket:
            self.socket.close()
            self.is_connected = False
