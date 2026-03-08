//! MCP JSON-RPC proxy for forwarding tool calls to external child processes.
//!
//! External MCP servers (word-document-server, mermaid-kroki, puppeteer, threejs)
//! communicate via newline-delimited JSON-RPC 2.0 over stdin/stdout. This module
//! provides the transport layer: serialise a request, write it to the child's
//! stdin, read the response from stdout, and deserialise it.
//!
//! All public functions are generic over `AsyncWriteExt` / `AsyncBufReadExt` so
//! they can be tested with in-memory duplex streams instead of real processes.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// Send an MCP JSON-RPC request to a child process and read the response.
///
/// Each message is a single line of JSON terminated by `\n` (newline-delimited
/// JSON-RPC, as required by the MCP stdio transport).
///
/// # Errors
///
/// Returns an error if:
/// - The request cannot be serialised.
/// - Writing to `stdin` or flushing fails.
/// - Reading from `stdout` returns EOF (zero bytes).
/// - The response line is not valid JSON.
pub async fn send_jsonrpc(
    stdin: &mut (impl AsyncWriteExt + Unpin),
    stdout: &mut (impl AsyncBufReadExt + Unpin),
    method: &str,
    params: Value,
    id: u64,
) -> Result<Value> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });

    let mut line = serde_json::to_string(&request)?;
    line.push('\n');
    stdin.write_all(line.as_bytes()).await?;
    stdin.flush().await?;

    let mut response_line = String::new();
    let bytes_read = stdout.read_line(&mut response_line).await?;
    if bytes_read == 0 {
        bail!("EOF: external MCP server closed its stdout before sending a response");
    }

    let response: Value = serde_json::from_str(&response_line)?;
    Ok(response)
}

/// Perform the MCP initialization handshake with an external server.
///
/// MCP requires a two-step handshake before any tool calls:
/// 1. Send `initialize` request and wait for the response.
/// 2. Send `notifications/initialized` notification (fire-and-forget, no response).
///
/// Returns the server's `initialize` response, which contains its capabilities.
pub async fn initialize_mcp(
    stdin: &mut (impl AsyncWriteExt + Unpin),
    stdout: &mut (impl AsyncBufReadExt + Unpin),
) -> Result<Value> {
    let result = send_jsonrpc(
        stdin,
        stdout,
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "opencheir",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }),
        0,
    )
    .await?;

    // Send the initialized notification (no id => no response expected).
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
    });
    let mut line = serde_json::to_string(&notification)?;
    line.push('\n');
    stdin.write_all(line.as_bytes()).await?;
    stdin.flush().await?;

    Ok(result)
}

/// Proxy a tool call to an external MCP server.
///
/// Wraps [`send_jsonrpc`] with the `tools/call` method and the standard MCP
/// tool-call parameter shape (`{ name, arguments }`).
pub async fn proxy_tool_call(
    stdin: &mut (impl AsyncWriteExt + Unpin),
    stdout: &mut (impl AsyncBufReadExt + Unpin),
    tool_name: &str,
    arguments: Value,
    id: u64,
) -> Result<Value> {
    send_jsonrpc(
        stdin,
        stdout,
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": arguments,
        }),
        id,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{BufReader, BufWriter};

    /// Minimal inline roundtrip test — the integration tests in tests/proxy_test.rs
    /// cover more scenarios.
    #[tokio::test]
    async fn test_send_jsonrpc_basic() {
        let (client_reader, server_writer) = tokio::io::duplex(4096);
        let (server_reader, client_writer) = tokio::io::duplex(4096);

        tokio::spawn(async move {
            let mut reader = BufReader::new(server_reader);
            let mut writer = BufWriter::new(server_writer);

            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();

            let req: Value = serde_json::from_str(&line).unwrap();
            let id = req["id"].clone();
            let resp = json!({"jsonrpc": "2.0", "id": id, "result": {"ok": true}});
            let mut resp_line = serde_json::to_string(&resp).unwrap();
            resp_line.push('\n');
            writer.write_all(resp_line.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
        });

        let mut writer = BufWriter::new(client_writer);
        let mut reader = BufReader::new(client_reader);

        let result = send_jsonrpc(&mut writer, &mut reader, "ping", json!({}), 1)
            .await
            .unwrap();

        assert_eq!(result["id"], 1);
        assert_eq!(result["result"]["ok"], true);
    }
}
