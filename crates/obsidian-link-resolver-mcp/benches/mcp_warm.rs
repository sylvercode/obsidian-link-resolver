//! Measure consecutive warm-cache MCP tool calls and emit a CI report.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// Number of end-to-end tool calls included in the warm-run report.
const SAMPLE_COUNT: usize = 100;

/// A running MCP server subprocess retained across tool calls.
struct McpProcess {
    /// Child server process, shut down when the benchmark completes.
    child: Child,
    /// MCP protocol input stream.
    stdin: ChildStdin,
    /// MCP protocol output stream.
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    /// Launch the server against the benchmark vault and initialize MCP.
    fn start(vault_root: &str) -> Self {
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
                "clientInfo": {"name": "mcp-warm-benchmark", "version": "1.0"}
            }),
        );
        assert!(initialized.get("result").is_some(), "{initialized}");
        process.write(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        process
    }

    /// Perform one tool call and return its JSON-RPC response.
    fn call_tool(&mut self, id: usize, context_path: &str, link: &str) -> Value {
        self.request(
            id as u64,
            "tools/call",
            json!({
                "name": "resolve_obsidian_link",
                "arguments": {
                    "link": link,
                    "context_path": context_path
                }
            }),
        )
    }

    /// Generate and validate the benchmark destination through the MCP tool.
    fn call_generation(&mut self, id: usize, context_path: &str) -> Value {
        self.request(
            id as u64,
            "tools/call",
            json!({
                "name": "generate_obsidian_link",
                "arguments": {
                    "context_path": context_path,
                    "target_path": "Target Note.md",
                    "intent": "existing",
                    "heading": "Target Heading"
                }
            }),
        )
    }

    /// Call the server information tool and return its JSON-RPC response.
    fn call_server_info(&mut self, id: usize) -> Value {
        self.request(
            id as u64,
            "tools/call",
            json!({
                "name": "get_server_info",
                "arguments": {}
            }),
        )
    }

    /// Write one JSON-RPC request and wait for the matching response line.
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

/// Run the 100-call warm-cache benchmark and print its machine-readable report.
fn main() {
    let (vault_root, context_path, link) = benchmark_vault();
    let mut process = McpProcess::start(&vault_root);
    let started = Instant::now();
    let first_server_info = process.call_server_info(2);
    let server_info_first_call = started.elapsed();
    assert_eq!(first_server_info["result"]["isError"], false);

    let warmup = process.call_tool(3, &context_path, link);
    assert_eq!(warmup["result"]["structuredContent"]["status"], "resolved");

    let mut server_info_samples = Vec::with_capacity(SAMPLE_COUNT);
    for request_id in 4..4 + SAMPLE_COUNT {
        let started = Instant::now();
        let result = process.call_server_info(request_id);
        server_info_samples.push(started.elapsed());
        assert_eq!(result["result"]["isError"], false);
    }
    let server_info_median = p50(&mut server_info_samples);

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for request_id in 4 + SAMPLE_COUNT..4 + SAMPLE_COUNT * 2 {
        let started = Instant::now();
        let result = process.call_tool(request_id, &context_path, link);
        samples.push(started.elapsed());
        assert_eq!(result["result"]["structuredContent"]["status"], "resolved");
    }
    let median = p50(&mut samples);

    let generation_warmup = process.call_generation(4 + SAMPLE_COUNT * 2, &context_path);
    assert_eq!(
        generation_warmup["result"]["structuredContent"]["status"],
        "validated"
    );
    let mut generation_samples = Vec::with_capacity(SAMPLE_COUNT);
    for request_id in 5 + SAMPLE_COUNT * 2..5 + SAMPLE_COUNT * 3 {
        let started = Instant::now();
        let result = process.call_generation(request_id, &context_path);
        generation_samples.push(started.elapsed());
        assert_eq!(result["result"]["structuredContent"]["status"], "validated");
    }
    let generation_median = p50(&mut generation_samples);
    let runner_image = std::env::var("ImageOS")
        .or_else(|_| std::env::var("RUNNER_IMAGE"))
        .unwrap_or_else(|_| "local".to_owned());
    let report = json!({
        "runner_image": runner_image,
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "fixture_version": "bench-vault-v1",
        "sample_count": SAMPLE_COUNT,
        "server_info_first_call_ms": server_info_first_call.as_secs_f64() * 1000.0,
        "server_info_p50_ms": server_info_median.as_secs_f64() * 1000.0,
        "p50_ms": median.as_secs_f64() * 1000.0,
        "generation_p50_ms": generation_median.as_secs_f64() * 1000.0
    });
    println!("{report}");
    assert!(
        server_info_first_call < Duration::from_millis(50),
        "server_info first call must be under 50 ms: {report}"
    );
    assert!(
        server_info_median <= Duration::from_millis(50),
        "server_info p50 exceeded 50 ms: {report}"
    );
    assert!(
        median <= Duration::from_millis(100),
        "warm MCP p50 exceeded 100 ms: {report}"
    );
    assert!(
        generation_median <= Duration::from_millis(100),
        "warm MCP generation p50 exceeded 100 ms: {report}"
    );
}

#[path = "../../../tests/fixtures/bench_vault.rs"]
mod bench_vault;
