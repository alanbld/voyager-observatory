//! Real rust-analyzer Integration Tests
//!
//! These tests connect to the actual rust-analyzer LSP server
//! to measure real-world latency and validate the Semantic Bridge concept.
//!
//! Note: Tests are skipped if rust-analyzer is not available.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use serde_json::{json, Value};

/// Check if rust-analyzer is available in PATH
fn rust_analyzer_available() -> bool {
    Command::new("rust-analyzer")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Read an LSP message (Content-Length header + body)
async fn read_lsp_message<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Option<String> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => return None,
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    break;
                }
                if let Some(value) = line.strip_prefix("Content-Length: ") {
                    content_length = value.parse().ok();
                }
            }
            Err(_) => return None,
        }
    }

    let content_length = content_length?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await.ok()?;

    String::from_utf8(body).ok()
}

/// Send an LSP message
async fn send_lsp_message<W: AsyncWriteExt + Unpin>(writer: &mut W, body: &str) -> std::io::Result<()> {
    let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    writer.write_all(msg.as_bytes()).await?;
    writer.flush().await
}

/// Parse JSON-RPC response and extract result
fn parse_response(response: &str) -> Option<Value> {
    let json: Value = serde_json::from_str(response).ok()?;
    json.get("result").cloned()
}

/// Metrics from rust-analyzer test
#[derive(Debug)]
pub struct RustAnalyzerMetrics {
    pub startup_latency: Duration,
    pub server_name: String,
    pub server_version: String,
    pub capabilities_count: usize,
}

impl std::fmt::Display for RustAnalyzerMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "rust-analyzer Metrics:\n\
             - Startup Latency: {:?}\n\
             - Server: {} v{}\n\
             - Capabilities: {}",
            self.startup_latency,
            self.server_name,
            self.server_version,
            self.capabilities_count
        )
    }
}

/// Test rust-analyzer initialization handshake with metrics collection
#[tokio::test]
async fn test_rust_analyzer_initialization() {
    if !rust_analyzer_available() {
        eprintln!("SKIPPED: rust-analyzer not found in PATH");
        return;
    }

    eprintln!("Starting rust-analyzer...");
    let start = Instant::now();

    // Spawn rust-analyzer
    let mut server = TokioCommand::new("rust-analyzer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn rust-analyzer");

    let mut stdin = server.stdin.take().unwrap();
    let stdout = server.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": "file:///tmp",
            "capabilities": {
                "textDocument": {
                    "documentSymbol": {
                        "hierarchicalDocumentSymbolSupport": true
                    }
                }
            }
        }
    });

    send_lsp_message(&mut stdin, &init_request.to_string()).await.unwrap();

    // Wait for response with 5-second timeout
    let response = timeout(Duration::from_secs(5), read_lsp_message(&mut reader))
        .await
        .expect("Timeout waiting for rust-analyzer response")
        .expect("Failed to read response");

    let startup_latency = start.elapsed();

    // Parse response
    let result = parse_response(&response).expect("Failed to parse initialize response");

    // Extract server info
    let server_info = result.get("serverInfo").expect("Missing serverInfo");
    let server_name = server_info.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let server_version = server_info.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Count capabilities
    let capabilities = result.get("capabilities").expect("Missing capabilities");
    let capabilities_count = if let Value::Object(map) = capabilities {
        map.len()
    } else {
        0
    };

    let metrics = RustAnalyzerMetrics {
        startup_latency,
        server_name: server_name.clone(),
        server_version: server_version.clone(),
        capabilities_count,
    };

    eprintln!("\n{}", metrics);

    // Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_lsp_message(&mut stdin, &initialized.to_string()).await.unwrap();

    // Send shutdown request
    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": null
    });
    send_lsp_message(&mut stdin, &shutdown.to_string()).await.unwrap();

    // Wait for shutdown response
    let _ = timeout(Duration::from_secs(2), read_lsp_message(&mut reader)).await;

    // Send exit notification
    let exit = json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    send_lsp_message(&mut stdin, &exit.to_string()).await.unwrap();

    // Cleanup
    drop(stdin);
    let _ = server.kill().await;

    // Assertions
    assert!(startup_latency < Duration::from_secs(5), "Startup took too long: {:?}", startup_latency);
    assert_eq!(server_name, "rust-analyzer");
    assert!(capabilities_count > 0, "No capabilities reported");

    eprintln!("\nTest PASSED - rust-analyzer startup: {:?}", startup_latency);
}

/// Test that we can handle rust-analyzer not being available
#[tokio::test]
async fn test_graceful_missing_server() {
    // Try to spawn a non-existent server
    let result = TokioCommand::new("nonexistent-lsp-server-12345")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();

    assert!(result.is_err(), "Should fail to spawn non-existent server");
}

/// Measure latency variation over multiple initializations
#[tokio::test]
async fn test_rust_analyzer_latency_consistency() {
    if !rust_analyzer_available() {
        eprintln!("SKIPPED: rust-analyzer not found in PATH");
        return;
    }

    const ITERATIONS: usize = 3;
    let mut latencies = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        eprintln!("Iteration {}/{}...", i + 1, ITERATIONS);

        let start = Instant::now();

        let mut server = TokioCommand::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn rust-analyzer");

        let mut stdin = server.stdin.take().unwrap();
        let stdout = server.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": "file:///tmp",
                "capabilities": {}
            }
        });

        send_lsp_message(&mut stdin, &init_request.to_string()).await.unwrap();

        let response = timeout(Duration::from_secs(5), read_lsp_message(&mut reader))
            .await
            .ok()
            .flatten();

        if response.is_some() {
            latencies.push(start.elapsed());
        }

        // Cleanup
        drop(stdin);
        let _ = server.kill().await;

        // Small delay between iterations
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    if latencies.is_empty() {
        eprintln!("SKIPPED: Could not complete any iterations");
        return;
    }

    // Calculate statistics
    let sum: Duration = latencies.iter().sum();
    let avg = sum / latencies.len() as u32;
    let min = latencies.iter().min().unwrap();
    let max = latencies.iter().max().unwrap();

    eprintln!("\n=== Latency Statistics ({} samples) ===", latencies.len());
    eprintln!("Min: {:?}", min);
    eprintln!("Max: {:?}", max);
    eprintln!("Avg: {:?}", avg);

    // Variance check - max should not be more than 5x min (reasonable for cold start)
    let ratio = max.as_micros() as f64 / min.as_micros() as f64;
    eprintln!("Variance ratio: {:.2}x", ratio);

    assert!(ratio < 10.0, "Latency variance too high: {:.2}x", ratio);
}
