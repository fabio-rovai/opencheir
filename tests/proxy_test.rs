use opencheir::gateway::proxy;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

/// Helper: spawn a mock MCP server task that reads one JSON-RPC request from
/// its "stdin" and writes back a valid JSON-RPC response on its "stdout".
/// Returns (client_writer, client_reader) that the proxy functions use.
fn mock_server() -> (
    BufWriter<tokio::io::DuplexStream>,
    BufReader<tokio::io::DuplexStream>,
    tokio::task::JoinHandle<()>,
) {
    // client_writer -> server_reader  (client sends requests)
    // server_writer -> client_reader  (server sends responses)
    let (client_reader, server_writer) = tokio::io::duplex(4096);
    let (server_reader, client_writer) = tokio::io::duplex(4096);

    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = req["id"].clone();

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{"type": "text", "text": "mock response"}]
            }
        });
        let mut resp_line = serde_json::to_string(&response).unwrap();
        resp_line.push('\n');
        writer.write_all(resp_line.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    });

    (
        BufWriter::new(client_writer),
        BufReader::new(client_reader),
        handle,
    )
}

/// Helper: spawn a mock MCP server that handles multiple requests in sequence.
/// Each request gets a response with the request id and method echoed back.
fn mock_multi_server(
    expected_requests: usize,
) -> (
    BufWriter<tokio::io::DuplexStream>,
    BufReader<tokio::io::DuplexStream>,
    tokio::task::JoinHandle<()>,
) {
    let (client_reader, server_writer) = tokio::io::duplex(8192);
    let (server_reader, client_writer) = tokio::io::duplex(8192);

    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);

        for _ in 0..expected_requests {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();

            let req: serde_json::Value = serde_json::from_str(&line).unwrap();

            // Skip notifications (no id field)
            if req.get("id").is_none() {
                continue;
            }

            let id = req["id"].clone();
            let method = req["method"].as_str().unwrap_or("unknown").to_string();

            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "method_echo": method,
                    "content": [{"type": "text", "text": "ok"}]
                }
            });
            let mut resp_line = serde_json::to_string(&response).unwrap();
            resp_line.push('\n');
            writer.write_all(resp_line.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
        }
    });

    (
        BufWriter::new(client_writer),
        BufReader::new(client_reader),
        handle,
    )
}

// ---- send_jsonrpc tests ----

#[tokio::test]
async fn test_send_jsonrpc_roundtrip() {
    let (mut writer, mut reader, handle) = mock_server();

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "tools/call",
        json!({"name": "test_tool"}),
        42,
    )
    .await
    .unwrap();

    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 42);
    assert_eq!(result["result"]["content"][0]["text"], "mock response");

    handle.await.unwrap();
}

#[tokio::test]
async fn test_send_jsonrpc_preserves_id() {
    let (mut writer, mut reader, handle) = mock_server();

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "initialize",
        json!({}),
        999,
    )
    .await
    .unwrap();

    assert_eq!(result["id"], 999);
    handle.await.unwrap();
}

#[tokio::test]
async fn test_send_jsonrpc_result_contains_content() {
    let (mut writer, mut reader, handle) = mock_server();

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "tools/call",
        json!({"name": "word_add_paragraph", "arguments": {"text": "hello"}}),
        1,
    )
    .await
    .unwrap();

    let content = result["result"]["content"].as_array().unwrap();
    assert!(!content.is_empty());
    assert_eq!(content[0]["type"], "text");
    handle.await.unwrap();
}

// ---- proxy_tool_call tests ----

#[tokio::test]
async fn test_proxy_tool_call() {
    let (mut writer, mut reader, handle) = mock_server();

    let result = proxy::proxy_tool_call(
        &mut writer,
        &mut reader,
        "word_add_paragraph",
        json!({"file_path": "test.docx", "text": "Hello"}),
        7,
    )
    .await
    .unwrap();

    assert_eq!(result["id"], 7);
    assert_eq!(result["result"]["content"][0]["text"], "mock response");
    handle.await.unwrap();
}

#[tokio::test]
async fn test_proxy_tool_call_with_complex_arguments() {
    let (mut writer, mut reader, handle) = mock_server();

    let args = json!({
        "file_path": "output.docx",
        "rows": [["A", "B"], ["1", "2"]],
        "style": {"bold": true, "font_size": 12}
    });

    let result = proxy::proxy_tool_call(
        &mut writer,
        &mut reader,
        "word_add_table",
        args,
        100,
    )
    .await
    .unwrap();

    assert_eq!(result["id"], 100);
    handle.await.unwrap();
}

// ---- initialize_mcp tests ----

#[tokio::test]
async fn test_initialize_mcp() {
    // initialize sends 2 messages: the initialize request + the initialized notification
    let (mut writer, mut reader, handle) = mock_multi_server(2);

    let result = proxy::initialize_mcp(&mut writer, &mut reader).await.unwrap();

    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 0);
    assert_eq!(result["result"]["method_echo"], "initialize");
    handle.await.unwrap();
}

// ---- error handling tests ----

#[tokio::test]
async fn test_send_jsonrpc_error_response_is_returned() {
    // An MCP server may return a JSON-RPC error; our proxy should return it as-is.
    let (client_reader, server_writer) = tokio::io::duplex(4096);
    let (server_reader, client_writer) = tokio::io::duplex(4096);

    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = req["id"].clone();

        let error_response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });
        let mut resp_line = serde_json::to_string(&error_response).unwrap();
        resp_line.push('\n');
        writer.write_all(resp_line.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    });

    let mut writer = BufWriter::new(client_writer);
    let mut reader = BufReader::new(client_reader);

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "nonexistent/method",
        json!({}),
        5,
    )
    .await
    .unwrap();

    // Error responses are still valid JSON-RPC; the proxy returns them without failing
    assert_eq!(result["id"], 5);
    assert_eq!(result["error"]["code"], -32601);
    assert_eq!(result["error"]["message"], "Method not found");

    handle.await.unwrap();
}

#[tokio::test]
async fn test_send_jsonrpc_invalid_json_response_is_error() {
    let (client_reader, server_writer) = tokio::io::duplex(4096);
    let (server_reader, client_writer) = tokio::io::duplex(4096);

    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        // Write invalid JSON back
        writer.write_all(b"this is not json\n").await.unwrap();
        writer.flush().await.unwrap();
    });

    let mut writer = BufWriter::new(client_writer);
    let mut reader = BufReader::new(client_reader);

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "tools/call",
        json!({}),
        1,
    )
    .await;

    assert!(result.is_err(), "Should fail on invalid JSON response");
    handle.await.unwrap();
}

#[tokio::test]
async fn test_send_jsonrpc_eof_response_is_error() {
    let (client_reader, server_writer) = tokio::io::duplex(4096);
    let (server_reader, client_writer) = tokio::io::duplex(4096);

    let handle = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        let _writer = server_writer; // drop immediately to close the stream

        // Read request then drop writer (EOF)
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        // writer is dropped -> EOF
    });

    let mut writer = BufWriter::new(client_writer);
    let mut reader = BufReader::new(client_reader);

    let result = proxy::send_jsonrpc(
        &mut writer,
        &mut reader,
        "tools/call",
        json!({}),
        1,
    )
    .await;

    assert!(result.is_err(), "Should fail when server closes connection (EOF)");
    handle.await.unwrap();
}
