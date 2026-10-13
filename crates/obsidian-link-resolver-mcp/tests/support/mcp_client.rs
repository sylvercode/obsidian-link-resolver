//! Minimal stdio client used by MCP subprocess tests.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// A running MCP server process and its JSON-lines protocol streams.
pub struct StdioClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioClient {
    /// Start the MCP server with the fixture vault as its default root.
    ///
    /// # Returns
    ///
    /// A client connected to the server's stdin and stdout pipes.
    pub fn new() -> Self {
        Self::with_vaults(vec![fixture_root()])
    }

    /// Start the MCP server with the supplied allowed vault roots.
    pub fn with_vaults(vaults: Vec<String>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_obsidian-link-resolver-mcp"));
        for vault in &vaults {
            command.args(["--vault", vault]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("MCP server process should start");
        let stdin = child.stdin.take().expect("server stdin should be piped");
        let stdout = child.stdout.take().expect("server stdout should be piped");
        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    /// Send the MCP initialize request and return its response.
    ///
    /// # Returns
    ///
    /// The JSON-RPC initialize response.
    pub fn initialize(&mut self) -> Value {
        self.request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "mcp-contract-test", "version": "1.0"}
            }),
        )
    }

    /// Notify the server that initialization has completed.
    pub fn notify_initialized(&mut self) {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
    }

    /// Send one JSON-RPC request and read its response.
    ///
    /// # Parameters
    ///
    /// - `id`: Request identifier used in the JSON-RPC envelope.
    /// - `method`: MCP method name.
    /// - `params`: Method-specific parameters.
    ///
    /// # Returns
    ///
    /// The parsed JSON-RPC response.
    pub fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        let mut response = String::new();
        self.stdout
            .read_line(&mut response)
            .expect("server response should be readable");
        serde_json::from_str(&response).expect("server should return a JSON-RPC response")
    }

    /// Call the resolver tool with the supplied arguments.
    ///
    /// # Parameters
    ///
    /// - `id`: Request identifier used in the JSON-RPC envelope.
    /// - `arguments`: Tool argument object.
    ///
    /// # Returns
    ///
    /// The parsed JSON-RPC tool-call response.
    pub fn call_tool(&mut self, id: u64, arguments: Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({
                "name": "resolve_obsidian_link",
                "arguments": arguments
            }),
        )
    }

    /// Write a JSON message followed by the stdio transport's newline delimiter.
    ///
    /// # Parameters
    ///
    /// - `message`: JSON message to write to the child process.
    fn write_message(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("server stdin should accept protocol messages");
        self.stdin.flush().expect("server stdin should flush");
    }
}

impl Drop for StdioClient {
    /// Stop the child process when the test client goes out of scope.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Return the repository's resolver fixture vault path.
///
/// # Returns
///
/// An absolute path to `tests/fixtures/vault`.
pub fn fixture_root() -> String {
    format!("{}/../../tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"))
}

/// Return the fixture note used as the link context.
///
/// # Returns
///
/// An absolute path to `tests/fixtures/vault/notes/a.md`.
pub fn context_file() -> String {
    format!("{}/notes/a.md", fixture_root())
}
