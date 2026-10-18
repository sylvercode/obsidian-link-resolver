//! Measure cold MCP server startup through the first resolution.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// Number of fresh server processes included in the cold-start report.
const SAMPLE_COUNT: usize = 100;

/// A fresh MCP server process used for exactly one measured resolution.
struct McpProcess {
    /// Child server process, shut down when the sample completes.
    child: Child,
    /// MCP protocol input stream.
    stdin: ChildStdin,
    /// MCP protocol output stream.
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    /// Launch and initialize the server, then resolve the benchmark link.
    ///
    /// # Parameters
    ///
    /// - `vault_root`: Canonical benchmark vault root passed to the server.
    /// - `context_path`: Existing context note used for the first resolution.
    /// - `link`: Link expected to resolve in the generated benchmark vault.
    ///
    /// # Returns
    ///
    /// The elapsed duration from process launch through the first resolution.
    fn start_and_resolve(vault_root: &str, context_path: &str, link: &str) -> Duration {
        let started = Instant::now();
        let mut child = Command::new(server_binary())
            .args(["--vault", vault_root])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("MCP server binary should start; build it before running this benchmark");
        let stdin = child.stdin.take().expect("server stdin should be piped");
        let stdout = child.stdout.take().expect("server stdout should be piped");
        let mut process = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        let initialized = process.request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "mcp-cold-start-benchmark", "version": "1.0"}
            }),
        );
        assert!(initialized.get("result").is_some(), "{initialized}");
        process.write(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));

        let result = process.request(
            2,
            "tools/call",
            json!({
                "name": "resolve_obsidian_link",
                "arguments": {
                    "link": link,
                    "context_path": context_path
                }
            }),
        );
        assert_eq!(result["result"]["structuredContent"]["status"], "resolved");
        started.elapsed()
    }

    /// Write one JSON-RPC request and wait for its response line.
    ///
    /// # Parameters
    ///
    /// - `id`: Request identifier matched in the JSON-RPC response.
    /// - `method`: MCP method name to invoke.
    /// - `params`: Method parameters serialized as JSON.
    ///
    /// # Returns
    ///
    /// The decoded JSON-RPC response.
    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.write(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        let mut response = String::new();
        self.stdout
            .read_line(&mut response)
            .expect("server response should be readable");
        serde_json::from_str(&response).expect("server response should be valid JSON")
    }

    /// Write one protocol message to the server.
    ///
    /// # Parameters
    ///
    /// - `message`: JSON-RPC message sent to the server's stdin.
    fn write(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("server stdin should accept protocol messages");
        self.stdin.flush().expect("server stdin should flush");
    }
}

impl Drop for McpProcess {
    /// Stop the benchmark server subprocess.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Resolve the MCP executable from an explicit override or Cargo's profile directory.
fn server_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("MCP_SERVER_BIN") {
        return PathBuf::from(path);
    }
    let benchmark_executable =
        std::env::current_exe().expect("benchmark executable path should be available");
    let profile_directory = benchmark_executable
        .parent()
        .and_then(|dependencies| dependencies.parent())
        .expect("benchmark should run beneath Cargo's target directory");
    let executable = if cfg!(windows) {
        "obsidian-link-resolver-mcp.exe"
    } else {
        "obsidian-link-resolver-mcp"
    };
    profile_directory.join(executable)
}

/// Build the versioned synthetic corpus and return its paths and canonical link.
fn benchmark_vault() -> (String, String, &'static str) {
    let (root, context, link) =
        bench_vault::ensure_bench_vault().expect("benchmark vault should be generated");
    (
        root.to_string_lossy().into_owned(),
        context.to_string_lossy().into_owned(),
        link,
    )
}

/// Calculate the median of sorted-duration samples.
fn p50(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    let middle = samples.len() / 2;
    if samples.len().is_multiple_of(2) {
        (samples[middle - 1] + samples[middle]) / 2
    } else {
        samples[middle]
    }
}

/// Run 100 fresh-process measurements and print their baseline report.
fn main() {
    let (vault_root, context_path, link) = benchmark_vault();
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        samples.push(McpProcess::start_and_resolve(
            &vault_root,
            &context_path,
            link,
        ));
    }

    let median = p50(&mut samples);
    let runner_image = std::env::var("ImageOS")
        .or_else(|_| std::env::var("RUNNER_IMAGE"))
        .unwrap_or_else(|_| "local".to_owned());
    let report = json!({
        "runner_image": runner_image,
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "fixture_version": "bench-vault-v1",
        "sample_count": SAMPLE_COUNT,
        "p50_ms": median.as_secs_f64() * 1000.0
    });
    println!("{report}");
}

#[path = "../../../tests/fixtures/bench_vault.rs"]
mod bench_vault;
